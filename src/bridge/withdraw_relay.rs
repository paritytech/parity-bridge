use std::sync::Arc;
use futures::{Future, Stream, Poll};
use futures::future::{JoinAll, join_all};
use web3::Transport;
use web3::helpers::CallResult;
use web3::types::{H256, Address, FilterBuilder, Log, Bytes, CallRequest};
use ethabi::{RawLog, self};
use app::App;
use api::{self, LogStream};
use contracts::{testnet, web3_filter};
use database::Database;
use error::{self, Error, ErrorKind};

fn collected_signatures_filter(testnet: &testnet::KovanBridge, address: Address) -> FilterBuilder {
	let filter = testnet.events().collected_signatures().create_filter();
	web3_filter(filter, address)
}

enum RelayAssignment {
	Ignore,
	Do(Vec<Bytes>),
}

impl RelayAssignment {
	fn flatten(self) -> Vec<Bytes> {
		match self {
			RelayAssignment::Ignore => vec![],
			RelayAssignment::Do(v) => v,
		}
	}
}

fn signatures_payload(testnet: &testnet::KovanBridge, signatures: u32, my_address: Address, log: Log) -> error::Result<RelayAssignment> {
	let raw_log = RawLog {
		topics: log.topics.into_iter().map(|t| t.0).collect(),
		data: log.data.0,
	};
	let collected_signatures = testnet.events().collected_signatures().parse_log(raw_log)?;
	if collected_signatures.authority != my_address.0 {
		// someone else will relay this transaction to mainnet
		return Ok(RelayAssignment::Ignore);
	}
	let payloads = (0..signatures).into_iter()
		.map(|index| ethabi::util::pad_u32(index))
		.map(|index| testnet.functions().signature().input(collected_signatures.message_hash, index))
		.map(Into::into)
		.collect();
	Ok(RelayAssignment::Do(payloads))
}

pub enum WithdrawRelayState<T: Transport> {
	Wait,
	FetchSignatures {
		future: JoinAll<Vec<JoinAll<Vec<CallResult<Bytes, T::Out>>>>>,
		block: u64,
	},
	RelayWithdraws {
		future: JoinAll<Vec<CallResult<H256, T::Out>>>,
		block: u64,
	},
	Yield(Option<u64>),
}

pub fn create_withdraw_relay<T: Transport + Clone>(app: Arc<App<T>>, init: &Database) -> WithdrawRelay<T> {
	let logs_init = api::LogStreamInit {
		after: init.checked_withdraw_relay,
		poll_interval: app.config.testnet.poll_interval,
		confirmations: app.config.testnet.required_confirmations,
		filter: collected_signatures_filter(&app.testnet_bridge, init.testnet_contract_address.clone()),
	};

	WithdrawRelay {
		logs: api::log_stream(app.connections.testnet.clone(), logs_init),
		mainnet_contract: init.mainnet_contract_address.clone(),
		testnet_contract: init.testnet_contract_address.clone(),
		state: WithdrawRelayState::Wait,
		app,
	}
}

pub struct WithdrawRelay<T: Transport> {
	app: Arc<App<T>>,
	logs: LogStream<T>,
	state: WithdrawRelayState<T>,
	testnet_contract: Address,
	mainnet_contract: Address,
}

impl<T: Transport> Stream for WithdrawRelay<T> {
	type Item = u64;
	type Error = Error;

	fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
		loop {
			let next_state = match self.state {
				WithdrawRelayState::Wait => {
					let item = try_stream!(self.logs.poll());
					let assignments = item.logs
						.into_iter()
						.map(|log| signatures_payload(
								&self.app.testnet_bridge, 
								self.app.config.authorities.required_signatures,
								self.app.config.testnet.account.clone(),
								log))
						.collect::<error::Result<Vec<_>>>()?;
					let all_calls = assignments.into_iter()
						.map(RelayAssignment::flatten)
						.map(|payloads| {
							payloads.into_iter()
								.map(|payload| CallRequest {
									from: None,
									to: self.testnet_contract.clone(),
									gas: None,
									gas_price: None,
									value: None,
									data: Some(payload),
								})
								.map(|request| api::call(&self.app.connections.testnet, request))
								.collect::<Vec<_>>()
						})
						.map(|calls| join_all(calls))
						.collect::<Vec<_>>();
					
					WithdrawRelayState::FetchSignatures {
						future: join_all(all_calls),
						block: item.to,
					}
				},
				WithdrawRelayState::FetchSignatures { ref mut future, block } => {
					let _ = try_ready!(future.poll().map_err(ErrorKind::Web3));
					unimplemented!();
				},
				WithdrawRelayState::RelayWithdraws { ref mut future, block } => {
					let _ = try_ready!(future.poll().map_err(ErrorKind::Web3));
					WithdrawRelayState::Yield(Some(block))
				},
				WithdrawRelayState::Yield(ref mut block) => match block.take() {
					None => WithdrawRelayState::Wait,
					some => return Ok(some.into()),
				}
			};
			self.state = next_state;
		}
	}
}
