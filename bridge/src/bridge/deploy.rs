use std::sync::Arc;
use futures::{Future, Poll, future};
use web3::Transport;
use web3::confirm::SendTransactionWithConfirmation;
use web3::types::{TransactionRequest};
use app::App;
use std::path::Path;
use std::fs;
use std::fs::File;
use std::io::Write;
use ethereum_types::{H256, H160};
use error::{Error, ErrorKind};
use rustc_hex::ToHex;
use database::Database;
use api;

pub enum Deployed {
	/// No existing database found. Deployed new contracts.
	New(Database),
	/// Reusing existing contracts.
	Existing(Database),
}

enum DeployState<T: Transport + Clone> {
	CheckIfNeeded,
	Deploying {
		home_data: Vec<u8>,
		foreign_data: Vec<u8>,
		future: future::Join<SendTransactionWithConfirmation<T>, SendTransactionWithConfirmation<T>>,
	}
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
							self.app.config.estimated_gas_cost_of_withdraw,
							self.app.config.max_total_home_contract_balance,
							self.app.config.max_single_deposit_value
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
							data: Some(home_data.clone().into()),
							nonce: None,
							condition: None,
						};

						let foreign_tx_request = TransactionRequest {
							from: self.app.config.foreign.account,
							to: None,
							gas: Some(self.app.config.txs.foreign_deploy.gas.into()),
							gas_price: Some(self.app.config.txs.foreign_deploy.gas_price.into()),
							value: None,
							data: Some(foreign_data.clone().into()),
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

						DeployState::Deploying {
							home_data: home_data,
							foreign_data: foreign_data,
							future: home_future.join(foreign_future)
						}
					},
					Err(err) => return Err(err.into()),
				},
				DeployState::Deploying {
					ref mut future,
					ref home_data,
					ref foreign_data
				} => {
					let (home_receipt, foreign_receipt) = try_ready!(future.poll().map_err(ErrorKind::Web3));

					let home_contract_address = home_receipt.contract_address.expect("contract creation receipt must have an address; qed");
					let foreign_contract_address = foreign_receipt.contract_address.expect("contract creation receipt must have an address; qed");

					write_deployment_info(
						format!("deployment-home-{}", home_contract_address.to_hex()),
						&home_contract_address,
						&home_receipt.transaction_hash,
						"HomeBridge",
						include_str!("../../../compiled_contracts/HomeBridge.abi"),
						include_str!("../../../compiled_contracts/HomeBridge.bin"),
						&home_data.to_hex()
					)?;

					write_deployment_info(
						format!("deployment-foreign-{}", foreign_contract_address.to_hex()),
						&foreign_contract_address,
						&foreign_receipt.transaction_hash,
						"ForeignBridge",
						include_str!("../../../compiled_contracts/ForeignBridge.abi"),
						include_str!("../../../compiled_contracts/ForeignBridge.bin"),
						&foreign_data.to_hex()
					)?;

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

/// writes useful information about the deployment into `dir`.
/// REMOVES `dir` if it already exists!
/// helps with troubleshooting and verification (https://ropsten.etherscan.io/verifyContract)
/// of deployments.
/// information includes:
/// - solc version used
/// - git commit
/// - contract source code
/// - contract address
/// - hash of transaction the contract got deployed in
/// - contract byte code
/// - input data for contract creation transaction
/// - ...
pub fn write_deployment_info<P: AsRef<Path>>(
	dir: P,
	contract_address: &H160,
	transaction_hash: &H256,
	contract_name: &str,
	abi: &str,
	bytecode_hex: &str,
	deployed_bytecode_hex: &str,
) -> Result<(), Error> {
	let dir = dir.as_ref();

	if Path::new(dir).exists() {
		fs::remove_dir_all(dir)?;
	}
	fs::create_dir(dir)?;

	let mut file = File::create(dir.join("bridge_version"))?;
	file.write_all(env!("CARGO_PKG_VERSION").as_bytes())?;

	let mut file = File::create(dir.join("commit_hash"))?;
	file.write_all(env!("GIT_HASH").as_bytes())?;

	let mut file = File::create(dir.join("compiler"))?;
	file.write_all(env!("SOLC_VERSION").as_bytes())?;

	let mut file = File::create(dir.join("optimization"))?;
	file.write_all("yes".as_bytes())?;

	let mut file = File::create(dir.join("contract_name"))?;
	file.write_all(contract_name.as_bytes())?;

	let mut file = File::create(dir.join("contract_address"))?;
	file.write_all(contract_address.to_hex().as_bytes())?;

	let mut file = File::create(dir.join("transaction_hash"))?;
	file.write_all(transaction_hash.to_hex().as_bytes())?;

	let mut file = File::create(dir.join("deployed_bytecode"))?;
	file.write_all(bytecode_hex.as_bytes())?;

	assert_eq!(
		bytecode_hex,
		&deployed_bytecode_hex[..bytecode_hex.len()],
		"deployed byte code is contract bytecode followed by constructor args; qed"
	);

	let constructor_arguments_bytecode = &deployed_bytecode_hex[bytecode_hex.len()..];

	let mut file = File::create(dir.join("constructor_arguments_bytecode"))?;
	file.write_all(constructor_arguments_bytecode.as_bytes())?;

	File::create(dir.join("abi"))?.write_all(abi.as_bytes())?;

	Ok(())
}
