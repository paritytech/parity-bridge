use std::vec;
use std::sync::Arc;
use futures::{Future, Stream, Poll};
use futures::future::{JoinAll, join_all};
use web3::Transport;
use web3::helpers::CallResult;
use web3::types::{TransactionRequest, H256, Log};
use api::{LogStream, self};
use error::{Error, ErrorKind};
use app::App;

pub enum WithdrawRelay {
	WaitForNextLog,
	RelayTransaction,	
}

pub enum WithdrawConfirm {
	WaitForNextLog,
	ConfirmTransaction,
}

pub enum DepositRelayState<T: Transport> {
	Wait,
	RelayDeposits {
		future: JoinAll<Vec<CallResult<H256, T::Out>>>,
		block: u64,
	},
	Yield(Option<u64>),
}

pub struct DepositRelay<T: Transport> {
	app: Arc<App<T>>,
	logs: LogStream<T>,
	state: DepositRelayState<T>,
}

impl<T: Transport> Stream for DepositRelay<T> {
	type Item = u64;
	type Error = Error;

	fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
		loop {
			let next_state = match self.state {
				DepositRelayState::Wait => match try_ready!(self.logs.poll()) {
					Some(item) => {
						let deposits = item.logs
							.into_iter()
							.map(|log| self.app.mainnet_bridge().log_to_deposit(log))
							.collect::<Result<Vec<_>, _>>()?
							.into_iter()
							.map(|deposit| -> TransactionRequest {
								unimplemented!();
							})
							.map(|request| api::send_transaction(&self.app.connections.testnet, request))
							.collect::<Vec<_>>();

						DepositRelayState::RelayDeposits {
							future: join_all(deposits),
							block: item.to
						}
					},
					None => return Ok(None.into()),
				},
				DepositRelayState::RelayDeposits { ref mut future, block } => {
					let _ = try_ready!(future.poll().map_err(ErrorKind::Web3));
					DepositRelayState::Yield(Some(block))
				},
				DepositRelayState::Yield(ref mut block) => match block.take() {
					None => DepositRelayState::Wait,
					some => return Ok(some.into()),
				}
			};
			self.state = next_state;
		}
	}
}

pub enum DepositConfirm {
	WaitForNextLog,
	ConfirmDeposit,
}
