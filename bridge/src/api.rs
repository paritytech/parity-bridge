use std::time::Duration;
use futures::{Future, Stream, Poll};
use tokio_timer::{Timer, Interval};
use web3::{api, Transport};
use web3::api::Namespace;
use web3::types::{Log, Filter, H256, H520, Block, BlockId, U256, FilterBuilder, TransactionRequest, Bytes, Address, CallRequest};
use web3::helpers::CallResult;
use error::{Error, ErrorKind};

/// Imperative alias for web3 function.
pub use web3::confirm::send_transaction_with_confirmation;

/// Imperative wrapper for web3 function.
pub fn logs<T: Transport>(transport: T, filter: &Filter) -> CallResult<Vec<Log>, T::Out> {
	api::Eth::new(transport).logs(filter)
}

/// Imperative wrapper for web3 function.
pub fn block<T: Transport>(transport: T, id: BlockId) -> CallResult<Block<H256>, T::Out> {
	api::Eth::new(transport).block(id)
}

/// Imperative wrapper for web3 function.
pub fn block_number<T: Transport>(transport: T) -> CallResult<U256, T::Out> {
	api::Eth::new(transport).block_number()
}

/// Imperative wrapper for web3 function.
pub fn send_transaction<T: Transport>(transport: T, tx: TransactionRequest) -> CallResult<H256, T::Out> {
	api::Eth::new(transport).send_transaction(tx)
}

/// Imperative wrapper for web3 function.
pub fn call<T: Transport>(transport: T, address: Address, payload: Bytes) -> CallResult<Bytes, T::Out> {
	api::Eth::new(transport).call(CallRequest {
		from: None,
		to: address,
		gas: None,
		gas_price: None,
		value: None,
		data: Some(payload),
	}, None)
}

pub fn sign<T: Transport>(transport: T, address: Address, data: Bytes) -> CallResult<H520, T::Out> {
	api::Eth::new(transport).sign(address, data)
}

/// Used for `LogStream` initialization.
pub struct LogStreamInit {
	pub after: u64,
	pub filter: FilterBuilder,
	pub poll_interval: Duration,
	pub confirmations: u64,
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
	/// Fetching best block.
	FetchBlockNumber(CallResult<U256, T::Out>),
	/// Fetching logs for new best block.
	FetchLogs {
		from: u64,
		to: u64,
		future: CallResult<Vec<Log>, T::Out>,
	},
	/// All logs has been fetched.
	NextItem(Option<LogStreamItem>),
}

/// Creates new `LogStream`.
pub fn log_stream<T: Transport>(transport: T, init: LogStreamInit) -> LogStream<T> {
	LogStream {
		transport,
		interval: Timer::default().interval(init.poll_interval),
		state: LogStreamState::Wait,
		after: init.after,
		filter: init.filter,
		confirmations: init.confirmations,
	}
}

/// Stream of confirmed logs.
pub struct LogStream<T: Transport> {
	transport: T,
	interval: Interval,
	state: LogStreamState<T>,
	after: u64,
	filter: FilterBuilder,
	confirmations: u64,
}

impl<T: Transport> Stream for LogStream<T> {
	type Item = LogStreamItem;
	type Error = Error;

	fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
		loop {
			let next_state = match self.state {
				LogStreamState::Wait => {
					let _ = try_stream!(self.interval.poll());
					LogStreamState::FetchBlockNumber(block_number(&self.transport))
				},
				LogStreamState::FetchBlockNumber(ref mut future) => {
					let last_block = try_ready!(future.poll().map_err(ErrorKind::Web3)).low_u64();
					let last_confirmed_block = last_block.saturating_sub(self.confirmations);
					if last_confirmed_block > self.after {
						let from = self.after + 1;
						let filter = self.filter.clone()
							.from_block(from.into())
							.to_block(last_confirmed_block.into())
							.build();
						LogStreamState::FetchLogs {
							from: from,
							to: last_confirmed_block,
							future: logs(&self.transport, &filter)
						}
					} else {
						LogStreamState::Wait
					}
				},
				LogStreamState::FetchLogs { ref mut future, from, to } => {
					let logs = try_ready!(future.poll().map_err(ErrorKind::Web3));
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
