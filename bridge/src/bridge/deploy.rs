use futures::{Future, Poll};
use web3::Transport;
use web3::confirm::{SendTransactionWithConfirmation, send_transaction_with_confirmation};
use web3::types::{TransactionReceipt, TransactionRequest};
use std::path::Path;
use std::fs;
use std::fs::File;
use std::io::Write;
use error::{self, ResultExt};
use rustc_hex::ToHex;
use config::Config;
use contracts::foreign::ForeignBridge;
use contracts::home::HomeBridge;

pub enum DeployState<T: Transport + Clone> {
    NotDeployed,
    Deploying {
        data: Vec<u8>,
        future: SendTransactionWithConfirmation<T>,
    },
    Deployed {
        contract: DeployedContract,
    },
}

pub struct DeployHome<T: Transport + Clone> {
    config: Config,
    home_connection: T,
    state: DeployState<T>,
}

impl<T: Transport + Clone> DeployHome<T> {
    pub fn new(config: Config, home_connection: T) -> Self {
        Self {
            config,
            home_connection,
            state: DeployState::NotDeployed,
        }
    }
}

impl<T: Transport + Clone> Future for DeployHome<T> {
    type Item = DeployedContract;
    type Error = error::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            let next_state = match self.state {
                DeployState::Deployed { ref contract } => return Ok(contract.clone().into()),
                DeployState::NotDeployed => {
                    let data = HomeBridge::default().constructor(
                        self.config.home.contract.bin.clone().0,
                        self.config.authorities.required_signatures,
                        self.config.authorities.accounts.clone(),
                        self.config.estimated_gas_cost_of_withdraw,
                        self.config.max_total_home_contract_balance,
                        self.config.max_single_deposit_value,
                    );

                    let tx_request = TransactionRequest {
                        from: self.config.address,
                        to: None,
                        gas: Some(self.config.txs.home_deploy.gas.into()),
                        gas_price: Some(self.config.txs.home_deploy.gas_price.into()),
                        value: None,
                        data: Some(data.clone().into()),
                        nonce: None,
                        condition: None,
                    };

                    let future = send_transaction_with_confirmation(
                        self.home_connection.clone(),
                        tx_request,
                        self.config.home.poll_interval,
                        self.config.home.required_confirmations,
                    );

                    info!("sending HomeBridge contract deployment transaction and waiting for {} confirmations...", self.config.home.required_confirmations);

                    DeployState::Deploying {
                        data: data,
                        future: future,
                    }
                }
                DeployState::Deploying {
                    ref mut future,
                    ref data,
                } => {
                    let receipt = try_ready!(future.poll()
                        .chain_err(|| "DeployHome: deployment transaction failed"));
                    let address = receipt
                        .contract_address
                        .expect("contract creation receipt must have an address; qed");
                    info!("HomeBridge deployment completed to {:?}", address);

                    DeployState::Deployed {
                        contract: DeployedContract::new(
                            "HomeBridge".into(),
                            include_str!("../../../contracts/bridge.sol").into(),
                            include_str!("../../../compiled_contracts/HomeBridge.abi").into(),
                            include_str!("../../../compiled_contracts/HomeBridge.bin").into(),
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

pub struct DeployForeign<T: Transport + Clone> {
    config: Config,
    foreign_connection: T,
    state: DeployState<T>,
}

impl<T: Transport + Clone> DeployForeign<T> {
    pub fn new(config: Config, foreign_connection: T) -> Self {
        Self {
            config,
            foreign_connection,
            state: DeployState::NotDeployed,
        }
    }
}

impl<T: Transport + Clone> Future for DeployForeign<T> {
    type Item = DeployedContract;
    type Error = error::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            let next_state = match self.state {
                DeployState::Deployed { ref contract } => return Ok(contract.clone().into()),
                DeployState::NotDeployed => {
                    let data = ForeignBridge::default().constructor(
                        self.config.foreign.contract.bin.clone().0,
                        self.config.authorities.required_signatures,
                        self.config.authorities.accounts.clone(),
                        self.config.estimated_gas_cost_of_withdraw,
                    );

                    let tx_request = TransactionRequest {
                        from: self.config.address,
                        to: None,
                        gas: Some(self.config.txs.foreign_deploy.gas.into()),
                        gas_price: Some(self.config.txs.foreign_deploy.gas_price.into()),
                        value: None,
                        data: Some(data.clone().into()),
                        nonce: None,
                        condition: None,
                    };

                    let future = send_transaction_with_confirmation(
                        self.foreign_connection.clone(),
                        tx_request,
                        self.config.foreign.poll_interval,
                        self.config.foreign.required_confirmations,
                    );

                    info!("sending ForeignBridge contract deployment transaction and waiting for {} confirmations...", self.config.foreign.required_confirmations);

                    DeployState::Deploying {
                        data: data,
                        future: future,
                    }
                }
                DeployState::Deploying {
                    ref mut future,
                    ref data,
                } => {
                    let receipt = try_ready!(future.poll()
                        .chain_err(|| "DeployForeign: deployment transaction failed"));
                    let address = receipt
                        .contract_address
                        .expect("contract creation receipt must have an address; qed");
                    info!("ForeignBridge deployment completed to {:?}", address);

                    DeployState::Deployed {
                        contract: DeployedContract::new(
                            "ForeignBridge".into(),
                            include_str!("../../../contracts/bridge.sol").into(),
                            include_str!("../../../compiled_contracts/ForeignBridge.abi").into(),
                            include_str!("../../../compiled_contracts/ForeignBridge.bin").into(),
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
            contract_address: receipt
                .contract_address
                .expect("contract creation receipt must have an address; qed")
                .to_hex(),
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
        file.write_all(self.receipt.transaction_hash.to_hex().as_bytes())?;

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
