use std::sync::Arc;
use futures::{Future, Stream, Poll};
use futures::future::{JoinAll, join_all};
use web3::Transport;
use web3::helpers::CallResult;
use web3::types::{TransactionRequest, H256, Address, Bytes, Log, FilterBuilder};
use ethabi::RawLog;
use api::{LogStream, self};
use error::{Error, ErrorKind, Result};
use database::Database;
use contracts::{mainnet, testnet};
use util::web3_filter;
use app::App;

fn deposits_filter(mainnet: &mainnet::EthereumBridge, address: Address) -> FilterBuilder {
	let filter = mainnet.events().deposit().create_filter();
	web3_filter(filter, address)
}

fn deposit_relay_payload(mainnet: &mainnet::EthereumBridge, testnet: &testnet::KovanBridge, log: Log) -> Result<Bytes> {
	let raw_log = RawLog {
		topics: log.topics.into_iter().map(|t| t.0).collect(),
		data: log.data.0,
	};
	let deposit_log = mainnet.events().deposit().parse_log(raw_log)?;
	let hash = log.transaction_hash.expect("log to be mined and contain `transaction_hash`");
	let payload = testnet.functions().deposit().input(deposit_log.recipient, deposit_log.value, hash.0);
	Ok(payload.into())
}

/// State of deposits relay.
enum DepositRelayState<T: Transport> {
	/// Deposit relay is waiting for logs.
	Wait,
	/// Relaying deposits in progress.
	RelayDeposits {
		future: JoinAll<Vec<CallResult<H256, T::Out>>>,
		block: u64,
	},
	/// All deposits till given block has been relayed.
	Yield(Option<u64>),
}

pub fn create_deposit_relay<T: Transport + Clone>(app: Arc<App<T>>, init: &Database) -> DepositRelay<T> {
	let logs_init = api::LogStreamInit {
		after: init.checked_deposit_relay,
		poll_interval: app.config.mainnet.poll_interval,
		confirmations: app.config.mainnet.required_confirmations,
		filter: deposits_filter(&app.mainnet_bridge, init.mainnet_contract_address.clone()),
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
						.map(|log| deposit_relay_payload(&self.app.mainnet_bridge, &self.app.testnet_bridge, log))
						.collect::<Result<Vec<_>>>()?
						.into_iter()
						.map(|payload| TransactionRequest {
							from: self.app.config.testnet.account.clone(),
							to: Some(self.testnet_contract.clone()),
							gas: Some(self.app.config.txs.deposit_relay.gas.into()),
							gas_price: Some(self.app.config.txs.deposit_relay.gas_price.into()),
							value: None,
							data: Some(payload),
							nonce: None,
							condition: None,
						})
						.map(|request| api::send_transaction(&self.app.connections.testnet, request))
						.collect::<Vec<_>>();

					DepositRelayState::RelayDeposits {
						future: join_all(deposits),
						block: item.to,
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

#[cfg(test)]
mod tests {
	use rustc_hex::FromHex;
	use web3::types::{Log, Bytes};
	use contracts::{mainnet, testnet};
	use super::deposit_relay_payload;

	#[test]
	fn test_deposit_relay_payload() {
		let mainnet = mainnet::EthereumBridge::default();
		let testnet = testnet::KovanBridge::default();

		let data = "000000000000000000000000aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0".from_hex().unwrap();
		let log = Log {
			data: data.into(),
			topics: vec!["0xe1fffcc4923d04b559f4d29a8bfc6cda04eb5b0d3c460751c2402c5c5cc9109c".parse().unwrap()],
			transaction_hash: Some("0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364".parse().unwrap()),
			..Default::default()
		};

		let payload = deposit_relay_payload(&mainnet, &testnet, log).unwrap();
		let expected: Bytes = "26b3293f000000000000000000000000aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364".from_hex().unwrap().into();
		assert_eq!(expected, payload);
	}
}
