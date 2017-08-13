use std::sync::Arc;
use futures::{Future, Stream, Poll, Async};
use futures::future::{JoinAll, join_all};
use web3::Transport;
use web3::helpers::CallResult;
use web3::types::{TransactionRequest, H256, Address};
use api::{LogStream, self};
use error::{Error, ErrorKind};
use database::Database;
use app::App;

/// State of deposits relay.
enum DepositRelayState<T: Transport> {
	/// Deposit relay is waiting for logs.
	Wait,
	/// Relaying deposits in progress.
	RelayDeposits {
		future: JoinAll<Vec<CallResult<H256, T::Out>>>,
		block: u64,
	},
	/// All deposits from given block has been relayed.
	Yield(Option<u64>),
}

pub fn create_deposit_relay<T: Transport + Clone>(app: Arc<App<T>>, init: &Database) -> DepositRelay<T> {
	let logs_init = api::LogStreamInit {
		after: init.checked_deposit_relay,
		poll_interval: app.config.mainnet.poll_interval,
		confirmations: app.config.mainnet.required_confirmations,
		filter: app.mainnet_bridge().deposits_filter(init.mainnet_contract_address.clone()),
	};
	DepositRelay {
		logs: api::log_stream(app.connections.mainnet.clone(), logs_init),
		testnet_contract: init.testnet_contract_address.clone(),
		state: DepositRelayState::Wait,
		app,
	}
}

pub struct DepositRelay<T: Transport> {
	app: Arc<App<T>>,
	logs: LogStream<T>,
	state: DepositRelayState<T>,
	testnet_contract: Address,
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
							to: Some(self.testnet_contract.clone()),
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
