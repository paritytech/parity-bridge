use std::sync::Arc;
use futures::{Future, Stream, Poll, Async};
use futures::future::{JoinAll, join_all};
use web3::Transport;
use web3::helpers::CallResult;
use web3::types::{TransactionRequest, H256, Log};
use api::{LogStream, self};
use error::{Error, ErrorKind};
use database::Database;
use app::App;

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
	init: Database,
}

impl<T: Transport> Stream for DepositRelay<T> {
	type Item = u64;
	type Error = Error;

	fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
		loop {
			let next_state = match self.state {
				DepositRelayState::Wait => {
					let item = try_stream!(self.logs.poll());
					let deposits = item.logs
						.into_iter()
						.map(|log| self.app.mainnet_bridge().deposit_from_log(log))
						.collect::<Result<Vec<_>, _>>()?
						.into_iter()
						.map(|deposit| self.app.testnet_bridge().deposit_payload(deposit))
						.map(|payload| TransactionRequest {
							from: self.app.config.testnet.account.clone(),
							to: Some(self.init.testnet.contract_address.clone()),
							gas: Some(self.app.config.testnet.txs.deposit.gas.into()),
							gas_price: Some(self.app.config.testnet.txs.deposit.gas_price.into()),
							value: Some(self.app.config.testnet.txs.deposit.value.into()),
							data: Some(payload),
							nonce: None,
							condition: None,
						})
						.map(|request| api::send_transaction(&self.app.connections.testnet, request))
						.collect::<Vec<_>>();

					DepositRelayState::RelayDeposits {
						future: join_all(deposits),
						block: item.to
					}
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
