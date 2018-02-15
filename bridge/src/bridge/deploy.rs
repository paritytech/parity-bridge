use std::sync::Arc;
use futures::{Future, Poll, future};
use web3::Transport;
use web3::confirm::SendTransactionWithConfirmation;
use web3::types::{TransactionRequest};
use app::App;
use database::Database;
use error::{Error, ErrorKind};
use api;

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
						let home_data = self.app.home_bridge.constructor(
							self.app.config.home.contract.bin.clone().0,
							self.app.config.authorities.required_signatures,
							self.app.config.authorities.accounts.clone(),
							self.app.config.estimated_gas_cost_of_withdraw
						);
						let foreign_data = self.app.foreign_bridge.constructor(
							self.app.config.foreign.contract.bin.clone().0,
							self.app.config.authorities.required_signatures,
							self.app.config.authorities.accounts.clone(),
							self.app.config.estimated_gas_cost_of_withdraw
						);

						let home_tx_request = TransactionRequest {
							from: self.app.config.home.account,
							to: None,
							gas: Some(self.app.config.txs.home_deploy.gas.into()),
							gas_price: Some(self.app.config.txs.home_deploy.gas_price.into()),
							value: None,
							data: Some(home_data.into()),
							nonce: None,
							condition: None,
						};

						let foreign_tx_request = TransactionRequest {
							from: self.app.config.foreign.account,
							to: None,
							gas: Some(self.app.config.txs.foreign_deploy.gas.into()),
							gas_price: Some(self.app.config.txs.foreign_deploy.gas_price.into()),
							value: None,
							data: Some(foreign_data.into()),
							nonce: None,
							condition: None,
						};

						let home_future = api::send_transaction_with_confirmation(
							self.app.connections.home.clone(),
							home_tx_request,
							self.app.config.home.poll_interval,
							self.app.config.home.required_confirmations
						);

						let foreign_future = api::send_transaction_with_confirmation(
							self.app.connections.foreign.clone(),
							foreign_tx_request,
							self.app.config.foreign.poll_interval,
							self.app.config.foreign.required_confirmations
						);

						DeployState::Deploying(home_future.join(foreign_future))
					},
					Err(err) => return Err(err.into()),
				},
				DeployState::Deploying(ref mut future) => {
					let (home_receipt, foreign_receipt) = try_ready!(future.poll().map_err(ErrorKind::Web3));
					let database = Database {
						home_contract_address: home_receipt.contract_address.expect("contract creation receipt must have an address; qed"),
						foreign_contract_address: foreign_receipt.contract_address.expect("contract creation receipt must have an address; qed"),
						home_deploy: home_receipt.block_number.low_u64(),
						foreign_deploy: foreign_receipt.block_number.low_u64(),
						checked_deposit_relay: home_receipt.block_number.low_u64(),
						checked_withdraw_relay: foreign_receipt.block_number.low_u64(),
						checked_withdraw_confirm: foreign_receipt.block_number.low_u64(),
					};
					return Ok(Deployed::New(database).into())
				},
			};

			self.state = next_state;
		}
	}
}
