use std::sync::Arc;
use std::ops;
use futures::{Future, Stream, Poll};
use futures::future::{JoinAll, join_all};
use tokio_timer::Timeout;
use ethabi::RawLog;
use web3::Transport;
use web3::types::{H256, H520, Address, TransactionRequest, Log, Bytes, FilterBuilder};
use api::{self, LogStream, ApiCall};
use app::App;
use contracts::testnet;
use util::web3_filter;
use database::Database;
use error::Error;

fn withdraws_filter(testnet: &testnet::KovanBridge, address: Address) -> FilterBuilder {
	let filter = testnet.events().withdraw().create_filter();
	web3_filter(filter, address)
}

fn withdraw_confirm_sign_payload(testnet: &testnet::KovanBridge, log: Log) -> Result<Bytes, Error> {
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
	SignWithdraws {
		withdraws: Vec<Bytes>,
		future: JoinAll<Vec<Timeout<ApiCall<H520, T::Out>>>>,
		block: u64,
	},
	/// Confirming withdraws.
	ConfirmWithdraws {
		future: JoinAll<Vec<Timeout<ApiCall<H256, T::Out>>>>,
		block: u64,
	},
	/// All withdraws till given block has been confirmed.
	Yield(Option<u64>),
}

pub fn create_withdraw_confirm<T: Transport + Clone>(app: Arc<App<T>>, init: &Database) -> WithdrawConfirm<T> {
	let logs_init = api::LogStreamInit {
		after: init.checked_withdraw_confirm,
		request_timeout: app.config.testnet.request_timeout,
		poll_interval: app.config.testnet.poll_interval,
		confirmations: app.config.testnet.required_confirmations,
		filter: withdraws_filter(&app.testnet_bridge, init.testnet_contract_address.clone()),
	};

	WithdrawConfirm {
		logs: api::log_stream(app.connections.testnet.clone(), app.timer.clone(), logs_init),
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
						.map(|log| withdraw_confirm_sign_payload(&self.app.testnet_bridge, log))
						.collect::<Result<Vec<_>, _>>()?;

					let requests = withdraws.clone()
						.into_iter()
						.map(|bytes| {
							self.app.timer.timeout(
								api::sign(&self.app.connections.testnet, self.app.config.testnet.account.clone(), bytes),
								self.app.config.testnet.request_timeout)
						})
						.collect::<Vec<_>>();

					WithdrawConfirmState::SignWithdraws {
						future: join_all(requests),
						withdraws: withdraws,
						block: item.to,
					}
				},
				WithdrawConfirmState::SignWithdraws { ref mut future, ref mut withdraws, block } => {
					let signatures = try_ready!(future.poll());
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
						.map(|request| {
							app.timer.timeout(
								api::send_transaction(&app.connections.testnet, request),
								app.config.testnet.request_timeout)
						})
						.collect::<Vec<_>>();

					WithdrawConfirmState::ConfirmWithdraws {
						future: join_all(confirmations),
						block,
					}
				},
				WithdrawConfirmState::ConfirmWithdraws { ref mut future, block } => {
					let _ = try_ready!(future.poll());
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

#[cfg(test)]
mod tests {
	use rustc_hex::FromHex;
	use web3::types::{Log, Bytes};
	use contracts::testnet;
	use super::{withdraw_confirm_sign_payload, withdraw_submit_signature_payload};

	#[test]
	fn test_withdraw_confirm_sign_payload() {
		let testnet = testnet::KovanBridge::default();

		let data = "000000000000000000000000aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0".from_hex().unwrap();
		let log = Log {
			data: data.into(),
			topics: vec!["0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364".parse().unwrap()],
			transaction_hash: Some("0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364".parse().unwrap()),
			..Default::default()
		};

		let payload = withdraw_confirm_sign_payload(&testnet, log).unwrap();
		let expected: Bytes = "aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364".from_hex().unwrap().into();
		assert_eq!(expected, payload);
	}

	#[test]
	fn test_withdraw_submit_signature_payload() {
		let testnet = testnet::KovanBridge::default();

		let message: Bytes = "aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364".from_hex().unwrap().into();
		let signature = "0x8697c15331677e6ebccccaff3454fce5edbc8cca8697c15331677aff3454fce5edbc8cca8697c15331677e6ebccccaff3454fce5edbc8cca8697c15331677e6ebc".parse().unwrap();

		let payload = withdraw_submit_signature_payload(&testnet, message, signature);
		let expected: Bytes = "630cea8e000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000c000000000000000000000000000000000000000000000000000000000000000418697c15331677e6ebccccaff3454fce5edbc8cca8697c15331677aff3454fce5edbc8cca8697c15331677e6ebccccaff3454fce5edbc8cca8697c15331677e6ebc000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000054aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364000000000000000000000000".from_hex().unwrap().into();
		assert_eq!(expected, payload);
	}
}
