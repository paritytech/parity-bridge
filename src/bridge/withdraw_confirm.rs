use std::sync::Arc;
use std::ops;
use futures::{Future, Stream, Poll};
use futures::future::{JoinAll, join_all};
use web3::Transport;
use web3::helpers::CallResult;
use web3::types::{H256, H520, Address, TransactionRequest};
use api::{self, LogStream};
use app::App;
use contracts::KovanWithdraw;
use error::{Error, ErrorKind};

pub enum WithdrawConfirmState<T: Transport> {
	Wait,
	SignWithraws {
		withdraws: Vec<KovanWithdraw>,
		future: JoinAll<Vec<CallResult<H520, T::Out>>>,
		block: u64,
	},
	ConfirmWithdraws {
		future: JoinAll<Vec<CallResult<H256, T::Out>>>,
		block: u64,
	},
	Yield(Option<u64>),
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
						.map(|log| self.app.testnet_bridge().withdraw_from_log(log))
						.collect::<Result<Vec<_>, _>>()?;

					let requests = withdraws.iter()
						.map(KovanWithdraw::bytes)
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
						.map(|(withdraw, signature)| app.testnet_bridge().collect_signatures_payload(signature, withdraw))
						.map(|payload| TransactionRequest {
							// TODO: gas pricing should be taken from correct config option!!!
							from: app.config.testnet.account.clone(),
							to: Some(testnet_contract.clone()),
							gas: Some(app.config.testnet.txs.deposit.gas.into()),
							gas_price: Some(app.config.testnet.txs.deposit.gas_price.into()),
							value: Some(app.config.testnet.txs.deposit.value.into()),
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
