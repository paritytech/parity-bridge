// Copyright 2017 Parity Technologies (UK) Ltd.
// This file is part of Parity-Bridge.

// Parity-Bridge is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity-Bridge is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity-Bridge.  If not, see <http://www.gnu.org/licenses/>.

//! concerning deployment of the bridge contracts

use config::Config;
use contracts;
use error::{self, ResultExt};
use futures::{Future, Poll};
use rustc_hex::ToHex;
use send_tx_with_receipt::{SendTransactionWithReceipt, SendTransactionWithReceiptOptions};
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use web3::types::{TransactionReceipt, TransactionRequest};
use web3::Transport;

pub enum DeployState<T: Transport + Clone> {
	NotDeployed,
	Deploying {
		data: Vec<u8>,
		future: SendTransactionWithReceipt<T>,
	},
	Deployed {
		contract: DeployedContract,
	},
}

pub struct DeployMain<T: Transport + Clone> {
	config: Config,
	main_transport: T,
	state: DeployState<T>,
}

impl<T: Transport + Clone> DeployMain<T> {
	pub fn new(config: Config, main_transport: T) -> Self {
		Self {
			config,
			main_transport,
			state: DeployState::NotDeployed,
		}
	}
}

impl<T: Transport + Clone> Future for DeployMain<T> {
	type Item = DeployedContract;
	type Error = error::Error;

	fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
		loop {
			let next_state = match self.state {
				DeployState::Deployed { ref contract } => return Ok(contract.clone().into()),
				DeployState::NotDeployed => {
					let data = contracts::main::constructor(
						self.config.main.contract.bin.clone().0,
						self.config.authorities.required_signatures,
						self.config.authorities.accounts.clone(),
					);

					let tx_request = TransactionRequest {
						from: self.config.address,
						to: None,
						gas: Some(self.config.txs.main_deploy.gas.into()),
						gas_price: Some(self.config.txs.main_deploy.gas_price.into()),
						value: None,
						data: Some(data.clone().into()),
						nonce: None,
						condition: None,
					};

					let future =
						SendTransactionWithReceipt::new(SendTransactionWithReceiptOptions {
							transport: self.main_transport.clone(),
							request_timeout: self.config.main.request_timeout,
							poll_interval: self.config.main.poll_interval,
							confirmations: self.config.main.required_confirmations,
							transaction: tx_request,
						});

					info!("sending MainBridge contract deployment transaction and waiting for {} confirmations...", self.config.main.required_confirmations);

					DeployState::Deploying { data, future }
				}
				DeployState::Deploying {
					ref mut future,
					ref data,
				} => {
					let receipt = try_ready!(future
						.poll()
						.chain_err(|| "DeployMain: deployment transaction failed"));
					let address = receipt
						.contract_address
						.expect("contract creation receipt must have an address; qed");
					info!("MainBridge deployment completed to {:?}", address);

					DeployState::Deployed {
						contract: DeployedContract::new(
							"Main".into(),
							include_str!("../../arbitrary/contracts/bridge.sol").into(),
							include_str!("../../compiled_contracts/Main.abi").into(),
							include_str!("../../compiled_contracts/Main.bin").into(),
							data.to_hex(),
							receipt,
						),
					}
				}
			};

			self.state = next_state;
		}
	}
}

pub struct DeploySide<T: Transport + Clone> {
	config: Config,
	side_transport: T,
	state: DeployState<T>,
}

impl<T: Transport + Clone> DeploySide<T> {
	pub fn new(config: Config, side_transport: T) -> Self {
		Self {
			config,
			side_transport,
			state: DeployState::NotDeployed,
		}
	}
}

impl<T: Transport + Clone> Future for DeploySide<T> {
	type Item = DeployedContract;
	type Error = error::Error;

	fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
		loop {
			let next_state = match self.state {
				DeployState::Deployed { ref contract } => return Ok(contract.clone().into()),
				DeployState::NotDeployed => {
					let data = contracts::side::constructor(
						self.config.side.contract.bin.clone().0,
						self.config.authorities.required_signatures,
						self.config.authorities.accounts.clone(),
					);

					let tx_request = TransactionRequest {
						from: self.config.address,
						to: None,
						gas: Some(self.config.txs.side_deploy.gas.into()),
						gas_price: Some(self.config.txs.side_deploy.gas_price.into()),
						value: None,
						data: Some(data.clone().into()),
						nonce: None,
						condition: None,
					};

					let future =
						SendTransactionWithReceipt::new(SendTransactionWithReceiptOptions {
							transport: self.side_transport.clone(),
							request_timeout: self.config.side.request_timeout,
							poll_interval: self.config.side.poll_interval,
							confirmations: self.config.side.required_confirmations,
							transaction: tx_request,
						});

					info!("sending SideBridge contract deployment transaction and waiting for {} confirmations...", self.config.side.required_confirmations);

					DeployState::Deploying { data, future }
				}
				DeployState::Deploying {
					ref mut future,
					ref data,
				} => {
					let receipt = try_ready!(future
						.poll()
						.chain_err(|| "DeploySide: deployment transaction failed"));
					let address = receipt
						.contract_address
						.expect("contract creation receipt must have an address; qed");
					info!("SideBridge deployment completed to {:?}", address);

					DeployState::Deployed {
						contract: DeployedContract::new(
							"SideBridge".into(),
							include_str!("../../arbitrary/contracts/bridge.sol").into(),
							include_str!("../../compiled_contracts/Side.abi").into(),
							include_str!("../../compiled_contracts/Side.bin").into(),
							data.to_hex(),
							receipt,
						),
					}
				}
			};

			self.state = next_state;
		}
	}
}

#[derive(Clone)]
pub struct DeployedContract {
	pub contract_name: String,
	pub contract_address: String,
	pub contract_source: String,
	pub abi: String,
	pub bytecode_hex: String,
	pub contract_creation_code_hex: String,
	pub receipt: TransactionReceipt,
}

impl DeployedContract {
	pub fn new(
		contract_name: String,
		contract_source: String,
		abi: String,
		bytecode_hex: String,
		contract_creation_code_hex: String,
		receipt: TransactionReceipt,
	) -> Self {
		assert_eq!(
			bytecode_hex,
			&contract_creation_code_hex[..bytecode_hex.len()],
			"deployed byte code is contract bytecode followed by constructor args; qed"
		);

		Self {
			contract_name,
			contract_address: format!(
				"{:x}",
				receipt
					.contract_address
					.expect("contract creation receipt must have an address; qed")
			),
			contract_source,
			abi,
			bytecode_hex,
			contract_creation_code_hex,
			receipt,
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
	pub fn dump_info<P: AsRef<Path>>(&self, dir: P) -> Result<(), error::Error> {
		let dir = dir.as_ref();

		if Path::new(dir).exists() {
			info!("{:?} exists. removing", dir);
			fs::remove_dir_all(dir)?;
		}
		fs::create_dir(dir)?;
		info!("{:?} created", dir);

		let mut file = File::create(dir.join("bridge_version"))?;
		file.write_all(env!("CARGO_PKG_VERSION").as_bytes())?;

		let mut file = File::create(dir.join("commit_hash"))?;
		file.write_all(env!("GIT_HASH").as_bytes())?;

		let mut file = File::create(dir.join("compiler"))?;
		file.write_all(env!("SOLC_VERSION").as_bytes())?;

		let mut file = File::create(dir.join("optimization"))?;
		file.write_all("yes".as_bytes())?;

		let mut file = File::create(dir.join("contract_name"))?;
		file.write_all(self.contract_name.as_bytes())?;

		let mut file = File::create(dir.join("contract_address"))?;
		file.write_all(self.contract_address.as_bytes())?;

		let mut file = File::create(dir.join("contract_source.sol"))?;
		file.write_all(self.contract_source.as_bytes())?;

		let mut file = File::create(dir.join("transaction_hash"))?;
		file.write_all(format!("{:x}", self.receipt.transaction_hash).as_bytes())?;

		let mut file = File::create(dir.join("deployed_bytecode"))?;
		file.write_all(self.bytecode_hex.as_bytes())?;

		let constructor_arguments_bytecode =
			&self.contract_creation_code_hex[self.bytecode_hex.len()..];

		let mut file = File::create(dir.join("constructor_arguments_bytecode"))?;
		file.write_all(constructor_arguments_bytecode.as_bytes())?;

		File::create(dir.join("abi"))?.write_all(self.abi.as_bytes())?;

		Ok(())
	}
}
