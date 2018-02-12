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
use message_to_mainnet::{MessageToMainnet, MESSAGE_LENGTH};

fn withdraws_filter(foreign: &foreign::ForeignBridge, address: Address) -> FilterBuilder {
	let filter = foreign.events().withdraw().create_filter();
	web3_filter(filter, address)
}

fn withdraw_submit_signature_payload(foreign: &foreign::ForeignBridge, withdraw_message: Vec<u8>, signature: H520) -> Bytes {
	assert_eq!(withdraw_message.len(), MESSAGE_LENGTH, "ForeignBridge never accepts messages with len != {} bytes; qed", MESSAGE_LENGTH);
	foreign.functions().submit_signature().input(signature.0.to_vec(), withdraw_message).into()
}

/// State of withdraw confirmation.
enum WithdrawConfirmState<T: Transport> {
	/// Withdraw confirm is waiting for logs.
	Wait,
	/// Signing withdraws.
	SignWithdraws {
		messages: Vec<Vec<u8>>,
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
		foreign_contract: init.foreign_contract_address,
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
					let withdraw_messages = item.logs
						.into_iter()
						.map(|log| {
							 info!("withdraw is ready for signature submission. tx hash {}", log.transaction_hash.unwrap());
							 Ok(MessageToMainnet::from_log(log)?.to_bytes())
						})
						.collect::<Result<Vec<_>, Error>>()?;

					let requests = withdraw_messages.clone()
						.into_iter()
						.map(|message| {
							self.app.timer.timeout(
								api::sign(&self.app.connections.foreign, self.app.config.foreign.account, Bytes(message)),
								self.app.config.foreign.request_timeout)
						})
						.collect::<Vec<_>>();

					info!("signing");
					WithdrawConfirmState::SignWithdraws {
						future: join_all(requests),
						messages: withdraw_messages,
						block: item.to,
					}
				},
				WithdrawConfirmState::SignWithdraws { ref mut future, ref mut messages, block } => {
					let signatures = try_ready!(future.poll());
					info!("signing complete");
					// borrow checker...
					let app = &self.app;
					let foreign_contract = &self.foreign_contract;
					let confirmations = messages
						.drain(ops::RangeFull)
						.zip(signatures.into_iter())
						.map(|(withdraw_message, signature)| {
							 withdraw_submit_signature_payload(&app.foreign_bridge, withdraw_message, signature)
						})
						.map(|payload| TransactionRequest {
							from: app.config.foreign.account,
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
