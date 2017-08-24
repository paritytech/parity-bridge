use std::sync::Arc;
use std::ops;
use futures::{Future, Stream, Poll};
use futures::future::{JoinAll, join_all};
use ethabi::RawLog;
use web3::Transport;
use web3::helpers::CallResult;
use web3::types::{H256, H520, Address, TransactionRequest, Log, Bytes, FilterBuilder};
use api::{self, LogStream};
use app::App;
use contracts::testnet;
use util::web3_filter;
use database::Database;
use error::{Error, ErrorKind};

fn withdraws_filter(testnet: &testnet::KovanBridge, address: Address) -> FilterBuilder {
	let filter = testnet.events().withdraw().create_filter();
	web3_filter(filter, address)
}

fn withdraw_confirm_payload(testnet: &testnet::KovanBridge, log: Log) -> Result<Bytes, Error> {
	let raw_log = RawLog {
		topics: log.topics.into_iter().map(|t| t.0).collect(),
		data: log.data.0,
	};
	let withdraw_log = testnet.events().withdraw().parse_log(raw_log)?;
	let hash = log.transaction_hash.expect("log to be mined and contain `transaction_hash`");
	let mut result = vec![0u8; 84];
	result[0..20].copy_from_slice(&withdraw_log.recipient);
	result[20..52].copy_from_slice(&withdraw_log.value);
	result[52..84].copy_from_slice(&hash);
	Ok(result.into())
}

fn withdraw_submit_signature_payload(testnet: &testnet::KovanBridge, withdraw_payload: Bytes, signature: H520) -> Bytes {
	testnet.functions().submit_signature().input(signature.to_vec(), withdraw_payload.0).into()
}

/// State of withdraw confirmation.
enum WithdrawConfirmState<T: Transport> {
	/// Withdraw confirm is waiting for logs.
	Wait,
	/// Signing withdraws.
	SignWithraws {
		withdraws: Vec<Bytes>,
		future: JoinAll<Vec<CallResult<H520, T::Out>>>,
		block: u64,
	},
	/// Confirming withdraws.
	ConfirmWithdraws {
		future: JoinAll<Vec<CallResult<H256, T::Out>>>,
		block: u64,
	},
	/// All withdraws till given block has been confirmed.
	Yield(Option<u64>),
}

pub fn create_withdraw_confirm<T: Transport + Clone>(app: Arc<App<T>>, init: &Database) -> WithdrawConfirm<T> {
	let logs_init = api::LogStreamInit {
		after: init.checked_withdraw_confirm,
		poll_interval: app.config.testnet.poll_interval,
		confirmations: app.config.testnet.required_confirmations,
		filter: withdraws_filter(&app.testnet_bridge, init.testnet_contract_address.clone()),
	};

	WithdrawConfirm {
		logs: api::log_stream(app.connections.testnet.clone(), logs_init),
		testnet_contract: init.testnet_contract_address.clone(),
		state: WithdrawConfirmState::Wait,
		app,
	}
}

pub struct WithdrawConfirm<T: Transport> {
	app: Arc<App<T>>,
	logs: LogStream<T>,
	state: WithdrawConfirmState<T>,
	testnet_contract: Address,
}

impl<T: Transport> Stream for WithdrawConfirm<T> {
	type Item = u64;
	type Error = Error;

	fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
		loop {
			let next_state = match self.state {
				WithdrawConfirmState::Wait => {
					let item = try_stream!(self.logs.poll());
					let withdraws = item.logs
						.into_iter()
						.map(|log| withdraw_confirm_payload(&self.app.testnet_bridge, log))
						.collect::<Result<Vec<_>, _>>()?;

					let requests = withdraws.clone()
						.into_iter()
						.map(|bytes| api::sign(&self.app.connections.testnet, self.app.config.testnet.account.clone(), bytes))
						.collect::<Vec<_>>();

					WithdrawConfirmState::SignWithraws {
						future: join_all(requests),
						withdraws: withdraws,
						block: item.to,
					}
				},
				WithdrawConfirmState::SignWithraws { ref mut future, ref mut withdraws, block } => {
					let signatures = try_ready!(future.poll().map_err(ErrorKind::Web3));
					// borrow checker...
					let app = &self.app;
					let testnet_contract = &self.testnet_contract;
					let confirmations = withdraws
						.drain(ops::RangeFull)
						.zip(signatures.into_iter())
						.map(|(withdraw, signature)| withdraw_submit_signature_payload(&app.testnet_bridge, withdraw, signature))
						.map(|payload| TransactionRequest {
							from: app.config.testnet.account.clone(),
							to: Some(testnet_contract.clone()),
							gas: Some(app.config.txs.withdraw_confirm.gas.into()),
							gas_price: Some(app.config.txs.withdraw_confirm.gas_price.into()),
							value: None,
							data: Some(payload),
							nonce: None,
							condition: None,
						})
						.map(|request| api::send_transaction(&app.connections.testnet, request))
						.collect::<Vec<_>>();

					WithdrawConfirmState::ConfirmWithdraws {
						future: join_all(confirmations),
						block,
					}
				},
				WithdrawConfirmState::ConfirmWithdraws { ref mut future, block } => {
					let _ = try_ready!(future.poll().map_err(ErrorKind::Web3));
					WithdrawConfirmState::Yield(Some(block))
				},
				WithdrawConfirmState::Yield(ref mut block) => match block.take() {
					None => WithdrawConfirmState::Wait,
					some => return Ok(some.into()),
				}
			};
			self.state = next_state;
		}
	}
}
