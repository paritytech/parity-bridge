use std::sync::Arc;
use futures::{Future, Stream, Poll};
use futures::future::{JoinAll, join_all, Join};
use web3::Transport;
use web3::helpers::CallResult;
use web3::types::{H256, Address, FilterBuilder, Log, Bytes, TransactionRequest};
use ethabi::{RawLog, self};
use app::App;
use api::{self, LogStream};
use contracts::{mainnet, testnet};
use util::web3_filter;
use database::Database;
use error::{self, Error, ErrorKind};

fn collected_signatures_filter(testnet: &testnet::KovanBridge, address: Address) -> FilterBuilder {
	let filter = testnet.events().collected_signatures().create_filter();
	web3_filter(filter, address)
}

enum RelayAssignment {
	Ignore,
	Do {
		signature_payloads: Vec<Bytes>,
		message_payload: Bytes,
	},
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
	let signature_payloads = (0..signatures).into_iter()
		.map(|index| ethabi::util::pad_u32(index))
		.map(|index| testnet.functions().signature().input(collected_signatures.message_hash, index))
		.map(Into::into)
		.collect();
	let message_payload = testnet.functions().message().input(collected_signatures.message_hash).into();

	Ok(RelayAssignment::Do {
		signature_payloads,
		message_payload,
	})
}

fn relay_payload(mainnet: &mainnet::EthereumBridge, signatures: Vec<Bytes>, message: Bytes) -> Bytes {
	let mut v_vec = Vec::new();
	let mut r_vec = Vec::new();
	let mut s_vec = Vec::new();
	for signature in signatures {
		let mut r = [0u8; 32];
		let mut s= [0u8; 32];
		let mut v = [0u8; 32];
		r.copy_from_slice(&signature.0[0..32]);
		s.copy_from_slice(&signature.0[32..64]);
		v[31] = signature.0[64];
		v_vec.push(v);
		s_vec.push(s);
		r_vec.push(r);
	}
	mainnet.functions().withdraw().input(v_vec, r_vec, s_vec, message.0).into()
}

pub enum WithdrawRelayState<T: Transport> {
	Wait,
	Fetch {
		future: Join<JoinAll<Vec<CallResult<Bytes, T::Out>>>, JoinAll<Vec<JoinAll<Vec<CallResult<Bytes, T::Out>>>>>>,
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

					let (signatures, messages): (Vec<_>, Vec<_>) = assignments.into_iter()
						.filter_map(|a| match a {
							RelayAssignment::Ignore => None,
							RelayAssignment::Do { signature_payloads, message_payload } => Some((signature_payloads, message_payload))
						})
						.unzip();

					let message_calls = messages.into_iter()
						.map(|payload| api::call(&self.app.connections.testnet, self.testnet_contract.clone(), payload))
						.collect::<Vec<_>>();

					let signature_calls = signatures.into_iter()
						.map(|payloads| {
							payloads.into_iter()
								.map(|payload| api::call(&self.app.connections.testnet, self.testnet_contract.clone(), payload))
								.collect::<Vec<_>>()
						})
						.map(|calls| join_all(calls))
						.collect::<Vec<_>>();

					WithdrawRelayState::Fetch {
						future: join_all(message_calls).join(join_all(signature_calls)),
						block: item.to,
					}
				},
				WithdrawRelayState::Fetch { ref mut future, block } => {
					let (messages, signatures) = try_ready!(future.poll().map_err(ErrorKind::Web3));
					assert_eq!(messages.len(), signatures.len());
					let app = &self.app;
					let mainnet_contract = &self.mainnet_contract;

					let relays = messages.into_iter().zip(signatures.into_iter())
						.map(|(message, signatures)| relay_payload(&app.mainnet_bridge, signatures, message))
						.map(|payload| TransactionRequest {
							from: app.config.mainnet.account.clone(),
							to: Some(mainnet_contract.clone()),
							gas: Some(app.config.txs.withdraw_relay.gas.into()),
							gas_price: Some(app.config.txs.withdraw_relay.gas_price.into()),
							value: None,
							data: Some(payload),
							nonce: None,
							condition: None,
						})
						.map(|request| api::send_transaction(&app.connections.mainnet, request))
						.collect::<Vec<_>>();
					WithdrawRelayState::RelayWithdraws {
						future: join_all(relays),
						block,
					}
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
