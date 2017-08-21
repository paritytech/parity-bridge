use std::sync::Arc;
use futures::{Future, Poll, future};
use web3::Transport;
use web3::confirm::SendTransactionWithConfirmation;
use web3::types::{TransactionRequest};
use app::App;
use database::Database;
use error::{Error, ErrorKind};
use {api, ethabi};

pub enum Deployed {
	/// No existing database found. Deployed new contracts.
	New(Database),
	/// Reusing existing contracts.
	Existing(Database),
}

enum DeployState<T: Transport + Clone> {
	CheckIfNeeded,
	Deploying(future::Join<SendTransactionWithConfirmation<T>, SendTransactionWithConfirmation<T>>),
}

pub fn create_deploy<T: Transport + Clone>(app: Arc<App<T>>) -> Deploy<T> {
	Deploy {
		app,
		state: DeployState::CheckIfNeeded,
	}
}

pub struct Deploy<T: Transport + Clone> {
	app: Arc<App<T>>,
	state: DeployState<T>,
}

impl<T: Transport + Clone> Future for Deploy<T> {
	type Item = Deployed;
	type Error = Error;

	fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
		loop {
			let next_state = match self.state {
				DeployState::CheckIfNeeded => match Database::load(&self.app.database_path).map_err(ErrorKind::from) {
					Ok(database) => return Ok(Deployed::Existing(database).into()),
					Err(ErrorKind::MissingFile(_)) => {
						let main_data = self.app.mainnet_bridge.constructor(
							self.app.config.mainnet.contract.bin.clone().0,
							ethabi::util::pad_u32(self.app.config.authorities.required_signatures),
							self.app.config.authorities.accounts.iter().map(|a| a.0.clone()).collect()
						);
						let test_data = self.app.testnet_bridge.constructor(
							self.app.config.testnet.contract.bin.clone().0,
							ethabi::util::pad_u32(self.app.config.authorities.required_signatures),
							self.app.config.authorities.accounts.iter().map(|a| a.0.clone()).collect()
						);

						let main_tx_request = TransactionRequest {
							from: self.app.config.mainnet.account,
							to: None,
							gas: Some(self.app.config.mainnet.txs.deploy.gas.into()),
							gas_price: Some(self.app.config.mainnet.txs.deploy.gas_price.into()),
							value: Some(self.app.config.mainnet.txs.deploy.value.into()),
							data: Some(main_data.into()),
							nonce: None,
							condition: None,
						};

						let test_tx_request = TransactionRequest {
							from: self.app.config.testnet.account,
							to: None,
							gas: Some(self.app.config.testnet.txs.deploy.gas.into()),
							gas_price: Some(self.app.config.testnet.txs.deploy.gas_price.into()),
							value: Some(self.app.config.testnet.txs.deploy.value.into()),
							data: Some(test_data.into()),
							nonce: None,
							condition: None,
						};

						let main_future = api::send_transaction_with_confirmation(
							self.app.connections.mainnet.clone(), 
							main_tx_request, 
							self.app.config.mainnet.poll_interval, 
							self.app.config.mainnet.required_confirmations
						);

						let test_future = api::send_transaction_with_confirmation(
							self.app.connections.testnet.clone(), 
							test_tx_request, 
							self.app.config.testnet.poll_interval, 
							self.app.config.testnet.required_confirmations
						);

						DeployState::Deploying(main_future.join(test_future))
					},
					Err(err) => return Err(err.into()),
				},
				DeployState::Deploying(ref mut future) => {
					let (main_receipt, test_receipt) = try_ready!(future.poll().map_err(ErrorKind::Web3));
					let database = Database {
						mainnet_contract_address: main_receipt.contract_address.expect("contract creation receipt must have an address; qed"),
						testnet_contract_address: test_receipt.contract_address.expect("contract creation receipt must have an address; qed"),
						mainnet_deploy: main_receipt.block_number.low_u64(),
						testnet_deploy: test_receipt.block_number.low_u64(),
						checked_deposit_relay: main_receipt.block_number.low_u64(),
						checked_withdraw_relay: test_receipt.block_number.low_u64(),
						checked_withdraw_confirm: test_receipt.block_number.low_u64(),
					};
					return Ok(Deployed::New(database).into())
				},
			};

			self.state = next_state;
		}
	}
}
