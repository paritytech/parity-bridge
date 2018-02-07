use std::time::Duration;
use serde::de::DeserializeOwned;
use serde_json::Value;
use futures::{Future, Stream, Poll};
use tokio_timer::{Timer, Interval, Timeout};
use web3::{self, api, Transport};
use web3::api::Namespace;
use web3::types::{Log, Filter, H256, H520, U256, FilterBuilder, TransactionRequest, Bytes, Address, CallRequest};
use web3::helpers::CallResult;
use error::{Error, ErrorKind};

/// Imperative alias for web3 function.
pub use web3::confirm::send_transaction_with_confirmation;

/// Wrapper type for `CallResult`
pub struct ApiCall<T, F> {
	future: CallResult<T, F>,
	message: &'static str,
}

impl<T, F> ApiCall<T, F> {
	pub fn message(&self) -> &'static str {
		self.message
	}
}

impl<T: DeserializeOwned, F: Future<Item = Value, Error = web3::Error>>Future for ApiCall<T, F> {
	type Item = T;
	type Error = Error;

	fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
		trace!(target: "bridge", "{}", self.message);
		self.future.poll().map_err(ErrorKind::Web3).map_err(Into::into)
	}
}

/// Imperative wrapper for web3 function.
pub fn logs<T: Transport>(transport: T, filter: &Filter) -> ApiCall<Vec<Log>, T::Out> {
	ApiCall {
		future: api::Eth::new(transport).logs(filter),
		message: "eth_getLogs",
	}
}

/// Imperative wrapper for web3 function.
pub fn block_number<T: Transport>(transport: T) -> ApiCall<U256, T::Out> {
	ApiCall {
		future: api::Eth::new(transport).block_number(),
		message: "eth_blockNumber",
	}
}

/// Imperative wrapper for web3 function.
pub fn send_transaction<T: Transport>(transport: T, tx: TransactionRequest) -> ApiCall<H256, T::Out> {
	ApiCall {
		future: api::Eth::new(transport).send_transaction(tx),
		message: "eth_sendTransaction",
	}
}

/// Imperative wrapper for web3 function.
pub fn call<T: Transport>(transport: T, address: Address, payload: Bytes) -> ApiCall<Bytes, T::Out> {
	let future = api::Eth::new(transport).call(CallRequest {
		from: None,
		to: address,
		gas: None,
		gas_price: None,
		value: None,
		data: Some(payload),
	}, None);

	ApiCall {
		future,
		message: "eth_call",
	}
}

pub fn sign<T: Transport>(transport: T, address: Address, data: Bytes) -> ApiCall<H520, T::Out> {
	ApiCall {
		future: api::Eth::new(transport).sign(address, data),
		message: "eth_sign",
	}
}

/// Used for `LogStream` initialization.
pub struct LogStreamInit {
	pub after: u64,
	pub filter: FilterBuilder,
	pub request_timeout: Duration,
	pub poll_interval: Duration,
	pub confirmations: usize,
}

/// Contains all logs matching `LogStream` filter in inclusive range `[from, to]`.
#[derive(Debug, PartialEq)]
pub struct LogStreamItem {
	pub from: u64,
	pub to: u64,
	pub logs: Vec<Log>,
}

/// Log Stream state.
enum LogStreamState<T: Transport> {
	/// Log Stream is waiting for timer to poll.
	Wait,
	/// Fetching best block number.
	FetchBlockNumber(Timeout<ApiCall<U256, T::Out>>),
	/// Fetching logs for new best block.
	FetchLogs {
		from: u64,
		to: u64,
		future: Timeout<ApiCall<Vec<Log>, T::Out>>,
	},
	/// All logs has been fetched.
	NextItem(Option<LogStreamItem>),
}

/// Creates new `LogStream`.
pub fn log_stream<T: Transport>(transport: T, timer: Timer, init: LogStreamInit) -> LogStream<T> {
	LogStream {
		transport,
		interval: timer.interval(init.poll_interval),
		timer,
		state: LogStreamState::Wait,
		after: init.after,
		filter: init.filter,
		confirmations: init.confirmations,
		request_timeout: init.request_timeout,
	}
}

/// Stream of confirmed logs.
pub struct LogStream<T: Transport> {
	transport: T,
	timer: Timer,
	interval: Interval,
	state: LogStreamState<T>,
	after: u64,
	filter: FilterBuilder,
	confirmations: usize,
	request_timeout: Duration,
}

impl<T: Transport> Stream for LogStream<T> {
	type Item = LogStreamItem;
	type Error = Error;

	fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
		loop {
			let next_state = match self.state {
				LogStreamState::Wait => {
					let _ = try_stream!(self.interval.poll());
					LogStreamState::FetchBlockNumber(self.timer.timeout(block_number(&self.transport), self.request_timeout))
				},
				LogStreamState::FetchBlockNumber(ref mut future) => {
					let last_block = try_ready!(future.poll()).low_u64();
					let last_confirmed_block = last_block.saturating_sub(self.confirmations as u64);
					if last_confirmed_block > self.after {
						let from = self.after + 1;
						let filter = self.filter.clone()
							.from_block(from.into())
							.to_block(last_confirmed_block.into())
							.build();
						LogStreamState::FetchLogs {
							from: from,
							to: last_confirmed_block,
							future: self.timer.timeout(logs(&self.transport, &filter), self.request_timeout),
						}
					} else {
						LogStreamState::Wait
					}
				},
				LogStreamState::FetchLogs { ref mut future, from, to } => {
					let logs = try_ready!(future.poll());
					let item = LogStreamItem {
						from,
						to,
						logs,
					};

					self.after = to;
					LogStreamState::NextItem(Some(item))
				},
				LogStreamState::NextItem(ref mut item) => match item.take() {
					None => LogStreamState::Wait,
					some => return Ok(some.into()),
				},
			};

			self.state = next_state;
		}
	}
}
