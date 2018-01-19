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
use contracts::foreign;
use util::web3_filter;
use database::Database;
use error::Error;

fn withdraws_filter(foreign: &foreign::ForeignBridge, address: Address) -> FilterBuilder {
	let filter = foreign.events().withdraw().create_filter();
	web3_filter(filter, address)
}

fn withdraw_confirm_sign_payload(foreign: &foreign::ForeignBridge, log: Log) -> Result<Bytes, Error> {
	let raw_log = RawLog {
		topics: log.topics.into_iter().map(|t| t.0).collect(),
		data: log.data.0,
	};
	let withdraw_log = foreign.events().withdraw().parse_log(raw_log)?;
	let hash = log.transaction_hash.expect("log to be mined and contain `transaction_hash`");
	let mut result = vec![0u8; 84];
	result[0..20].copy_from_slice(&withdraw_log.recipient);
	result[20..52].copy_from_slice(&withdraw_log.value);
	result[52..84].copy_from_slice(&hash);
	Ok(result.into())
}

fn withdraw_submit_signature_payload(foreign: &foreign::ForeignBridge, withdraw_payload: Bytes, signature: H520) -> Bytes {
	foreign.functions().submit_signature().input(signature.to_vec(), withdraw_payload.0).into()
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
		request_timeout: app.config.foreign.request_timeout,
		poll_interval: app.config.foreign.poll_interval,
		confirmations: app.config.foreign.required_confirmations,
		filter: withdraws_filter(&app.foreign_bridge, init.foreign_contract_address.clone()),
	};

	WithdrawConfirm {
		logs: api::log_stream(app.connections.foreign.clone(), app.timer.clone(), logs_init),
		foreign_contract: init.foreign_contract_address.clone(),
		state: WithdrawConfirmState::Wait,
		app,
	}
}

pub struct WithdrawConfirm<T: Transport> {
	app: Arc<App<T>>,
	logs: LogStream<T>,
	state: WithdrawConfirmState<T>,
	foreign_contract: Address,
}

impl<T: Transport> Stream for WithdrawConfirm<T> {
	type Item = u64;
	type Error = Error;

	fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
		loop {
			let next_state = match self.state {
				WithdrawConfirmState::Wait => {
					let item = try_stream!(self.logs.poll());
					info!("got {} new withdraws to sign", item.logs.len());
					let withdraws = item.logs
						.into_iter()
						.map(|log| {
							 info!("withdraw is ready for signature submission. tx hash {}", log.transaction_hash.unwrap());
							 withdraw_confirm_sign_payload(&self.app.foreign_bridge, log)
						})
						.collect::<Result<Vec<_>, _>>()?;

					let requests = withdraws.clone()
						.into_iter()
						.map(|bytes| {
							self.app.timer.timeout(
								api::sign(&self.app.connections.foreign, self.app.config.foreign.account.clone(), bytes),
								self.app.config.foreign.request_timeout)
						})
						.collect::<Vec<_>>();

					info!("signing");
					WithdrawConfirmState::SignWithdraws {
						future: join_all(requests),
						withdraws: withdraws,
						block: item.to,
					}
				},
				WithdrawConfirmState::SignWithdraws { ref mut future, ref mut withdraws, block } => {
					let signatures = try_ready!(future.poll());
					info!("signing complete");
					// borrow checker...
					let app = &self.app;
					let foreign_contract = &self.foreign_contract;
					let confirmations = withdraws
						.drain(ops::RangeFull)
						.zip(signatures.into_iter())
						.map(|(withdraw, signature)| withdraw_submit_signature_payload(&app.foreign_bridge, withdraw, signature))
						.map(|payload| TransactionRequest {
							from: app.config.foreign.account.clone(),
							to: Some(foreign_contract.clone()),
							gas: Some(app.config.txs.withdraw_confirm.gas.into()),
							gas_price: Some(app.config.txs.withdraw_confirm.gas_price.into()),
							value: None,
							data: Some(payload),
							nonce: None,
							condition: None,
						})
						.map(|request| {
							info!("submitting signature");
							app.timer.timeout(
								api::send_transaction(&app.connections.foreign, request),
								app.config.foreign.request_timeout)
						})
						.collect::<Vec<_>>();

					info!("submitting {} signatures", confirmations.len());
					WithdrawConfirmState::ConfirmWithdraws {
						future: join_all(confirmations),
						block,
					}
				},
				WithdrawConfirmState::ConfirmWithdraws { ref mut future, block } => {
					let _ = try_ready!(future.poll());
					info!("submitting signatures complete");
					WithdrawConfirmState::Yield(Some(block))
				},
				WithdrawConfirmState::Yield(ref mut block) => match block.take() {
					None => {
						info!("waiting for new withdraws that should get signed");
						WithdrawConfirmState::Wait
					},
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
	use contracts::foreign;
	use super::{withdraw_confirm_sign_payload, withdraw_submit_signature_payload};

	#[test]
	fn test_withdraw_confirm_sign_payload() {
		let foreign = foreign::ForeignBridge::default();

		let data = "000000000000000000000000aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0".from_hex().unwrap();
		let log = Log {
			data: data.into(),
			topics: vec!["0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364".parse().unwrap()],
			transaction_hash: Some("0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364".parse().unwrap()),
			..Default::default()
		};

		let payload = withdraw_confirm_sign_payload(&foreign, log).unwrap();
		let expected: Bytes = "aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364".from_hex().unwrap().into();
		assert_eq!(expected, payload);
	}

	#[test]
	fn test_withdraw_submit_signature_payload() {
		let foreign = foreign::ForeignBridge::default();

		let message: Bytes = "aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364".from_hex().unwrap().into();
		let signature = "0x8697c15331677e6ebccccaff3454fce5edbc8cca8697c15331677aff3454fce5edbc8cca8697c15331677e6ebccccaff3454fce5edbc8cca8697c15331677e6ebc".parse().unwrap();

		let payload = withdraw_submit_signature_payload(&foreign, message, signature);
		let expected: Bytes = "630cea8e000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000c000000000000000000000000000000000000000000000000000000000000000418697c15331677e6ebccccaff3454fce5edbc8cca8697c15331677aff3454fce5edbc8cca8697c15331677e6ebccccaff3454fce5edbc8cca8697c15331677e6ebc000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000054aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364000000000000000000000000".from_hex().unwrap().into();
		assert_eq!(expected, payload);
	}
}
