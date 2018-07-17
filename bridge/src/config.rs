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
use error::{Error, ResultExt};
use ethereum_types::U256;
use rustc_hex::FromHex;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::time::Duration;
use toml;
use web3::types::{Address, Bytes};

const DEFAULT_POLL_INTERVAL: u64 = 1;
const DEFAULT_TIMEOUT: u64 = 5;

const DEFAULT_CONFIRMATIONS: u32 = 12;

/// Application config.
#[derive(Debug, PartialEq, Clone)]
pub struct Config {
    pub address: Address,
    pub main: NodeConfig,
    pub side: NodeConfig,
    pub authorities: Authorities,
    pub txs: Transactions,
    pub estimated_gas_cost_of_withdraw: U256,
    pub max_total_main_contract_balance: U256,
    pub max_single_deposit_value: U256,
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Config, Error> {
        let mut file = fs::File::open(path).chain_err(|| "Cannot open config")?;
        let mut buffer = String::new();
        file.read_to_string(&mut buffer).expect("TODO");
        Self::load_from_str(&buffer)
    }

    fn load_from_str(s: &str) -> Result<Config, Error> {
        let config: load::Config = toml::from_str(s).chain_err(|| "Cannot parse config")?;
        Config::from_load_struct(config)
    }

    fn from_load_struct(config: load::Config) -> Result<Config, Error> {
        let result = Config {
            address: config.address,
            main: NodeConfig::from_load_struct(config.main)?,
            side: NodeConfig::from_load_struct(config.side)?,
            authorities: Authorities {
                accounts: config.authorities.accounts,
                required_signatures: config.authorities.required_signatures,
            },
            txs: config
                .transactions
                .map(Transactions::from_load_struct)
                .unwrap_or_default(),
            estimated_gas_cost_of_withdraw: config.estimated_gas_cost_of_withdraw,
            max_total_main_contract_balance: config.max_total_main_contract_balance,
            max_single_deposit_value: config.max_single_deposit_value,
        };

        Ok(result)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct NodeConfig {
    pub contract: ContractConfig,
    pub http: String,
    pub request_timeout: Duration,
    pub poll_interval: Duration,
    pub required_confirmations: u32,
}

impl NodeConfig {
    fn from_load_struct(node: load::NodeConfig) -> Result<NodeConfig, Error> {
        let result = Self {
            contract: ContractConfig {
                bin: {
                    let mut read = String::new();
                    let mut file = fs::File::open(&node.contract.bin).chain_err(|| {
                        format!(
                            "Cannot open compiled contract file at {}",
                            node.contract.bin.to_string_lossy()
                        )
                    })?;
                    file.read_to_string(&mut read)?;
                    Bytes(read.from_hex()?)
                },
            },
            http: node.http,
            request_timeout: Duration::from_secs(node.request_timeout.unwrap_or(DEFAULT_TIMEOUT)),
            poll_interval: Duration::from_secs(node.poll_interval.unwrap_or(DEFAULT_POLL_INTERVAL)),
            required_confirmations: node.required_confirmations.unwrap_or(DEFAULT_CONFIRMATIONS),
        };

        Ok(result)
    }
}

#[derive(Debug, PartialEq, Default, Clone)]
pub struct Transactions {
    pub main_deploy: TransactionConfig,
    pub side_deploy: TransactionConfig,
    pub deposit_relay: TransactionConfig,
    pub withdraw_confirm: TransactionConfig,
    pub withdraw_relay: TransactionConfig,
}

impl Transactions {
    fn from_load_struct(cfg: load::Transactions) -> Self {
        Transactions {
            main_deploy: cfg.main_deploy
                .map(TransactionConfig::from_load_struct)
                .unwrap_or_default(),
            side_deploy: cfg.side_deploy
                .map(TransactionConfig::from_load_struct)
                .unwrap_or_default(),
            deposit_relay: cfg.deposit_relay
                .map(TransactionConfig::from_load_struct)
                .unwrap_or_default(),
            withdraw_confirm: cfg.withdraw_confirm
                .map(TransactionConfig::from_load_struct)
                .unwrap_or_default(),
            withdraw_relay: cfg.withdraw_relay
                .map(TransactionConfig::from_load_struct)
                .unwrap_or_default(),
        }
    }
}

#[derive(Debug, PartialEq, Default, Clone)]
pub struct TransactionConfig {
    pub gas: U256,
    pub gas_price: U256,
}

impl TransactionConfig {
    fn from_load_struct(cfg: load::TransactionConfig) -> Self {
        TransactionConfig {
            gas: cfg.gas,
            gas_price: cfg.gas_price,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ContractConfig {
    pub bin: Bytes,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Authorities {
    pub accounts: Vec<Address>,
    pub required_signatures: u32,
}

/// Some config values may not be defined in `toml` file, but they should be specified at runtime.
/// `load` module separates `Config` representation in file with optional from the one used
/// in application.
mod load {
    use ethereum_types::U256;
    use helpers::deserialize_u256;
    use std::path::PathBuf;
    use web3::types::Address;

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Config {
        pub address: Address,
        pub main: NodeConfig,
        pub side: NodeConfig,
        pub authorities: Authorities,
        pub transactions: Option<Transactions>,
        #[serde(deserialize_with = "deserialize_u256")]
        pub estimated_gas_cost_of_withdraw: U256,
        #[serde(deserialize_with = "deserialize_u256")]
        pub max_total_main_contract_balance: U256,
        #[serde(deserialize_with = "deserialize_u256")]
        pub max_single_deposit_value: U256,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct NodeConfig {
        pub contract: ContractConfig,
        pub http: String,
        pub request_timeout: Option<u64>,
        pub poll_interval: Option<u64>,
        pub required_confirmations: Option<u32>,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Transactions {
        pub main_deploy: Option<TransactionConfig>,
        pub side_deploy: Option<TransactionConfig>,
        pub deposit_relay: Option<TransactionConfig>,
        pub withdraw_confirm: Option<TransactionConfig>,
        pub withdraw_relay: Option<TransactionConfig>,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct TransactionConfig {
        #[serde(deserialize_with = "deserialize_u256")]
        pub gas: U256,
        #[serde(deserialize_with = "deserialize_u256")]
        pub gas_price: U256,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct ContractConfig {
        pub bin: PathBuf,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Authorities {
        pub accounts: Vec<Address>,
        pub required_signatures: u32,
    }
}

#[cfg(test)]
mod tests {
    use super::{Authorities, Config, ContractConfig, NodeConfig, TransactionConfig, Transactions};
    use ethereum_types::U256;
    use rustc_hex::FromHex;
    use std::time::Duration;

    #[test]
    fn load_full_setup_from_str() {
        let toml = r#"
address = "0x1B68Cb0B50181FC4006Ce572cF346e596E51818b"
estimated_gas_cost_of_withdraw = "100000"
max_total_main_contract_balance = "10000000000000000000"
max_single_deposit_value = "1000000000000000000"

[main]
http = "http://localhost:8545"
poll_interval = 2
required_confirmations = 100

[main.contract]
bin = "../compiled_contracts/MainBridge.bin"

[side]
http = "http://localhost:8546"

[side.contract]
bin = "../compiled_contracts/SideBridge.bin"

[authorities]
accounts = [
	"0x0000000000000000000000000000000000000001",
	"0x0000000000000000000000000000000000000002",
	"0x0000000000000000000000000000000000000003"
]
required_signatures = 2

[transactions]
main_deploy = { gas = "20", gas_price = "0" }
"#;

        let mut expected = Config {
            address: "1B68Cb0B50181FC4006Ce572cF346e596E51818b".into(),
            txs: Transactions::default(),
            main: NodeConfig {
                http: "http://localhost:8545".into(),
                contract: ContractConfig {
                    bin: include_str!("../../compiled_contracts/MainBridge.bin")
                        .from_hex()
                        .unwrap()
                        .into(),
                },
                poll_interval: Duration::from_secs(2),
                request_timeout: Duration::from_secs(5),
                required_confirmations: 100,
            },
            side: NodeConfig {
                contract: ContractConfig {
                    bin: include_str!("../../compiled_contracts/SideBridge.bin")
                        .from_hex()
                        .unwrap()
                        .into(),
                },
                http: "http://localhost:8546".into(),
                poll_interval: Duration::from_secs(1),
                request_timeout: Duration::from_secs(5),
                required_confirmations: 12,
            },
            authorities: Authorities {
                accounts: vec![
                    "0000000000000000000000000000000000000001".into(),
                    "0000000000000000000000000000000000000002".into(),
                    "0000000000000000000000000000000000000003".into(),
                ],
                required_signatures: 2,
            },
            estimated_gas_cost_of_withdraw: U256::from_dec_str("100000").unwrap(),
            max_total_main_contract_balance: U256::from_dec_str("10000000000000000000").unwrap(),
            max_single_deposit_value: U256::from_dec_str("1000000000000000000").unwrap(),
        };

        expected.txs.main_deploy = TransactionConfig {
            gas: 20.into(),
            gas_price: 0.into(),
        };

        let config = Config::load_from_str(toml).unwrap();
        assert_eq!(expected, config);
    }

    #[test]
    fn load_minimal_setup_from_str() {
        let toml = r#"
address = "0x0000000000000000000000000000000000000001"
estimated_gas_cost_of_withdraw = "200000000"
max_total_main_contract_balance = "10000000000000000000"
max_single_deposit_value = "1000000000000000000"

[main]
http = ""

[main.contract]
bin = "../compiled_contracts/MainBridge.bin"

[side]
http = ""

[side.contract]
bin = "../compiled_contracts/SideBridge.bin"

[authorities]
accounts = [
	"0x0000000000000000000000000000000000000001",
	"0x0000000000000000000000000000000000000002",
	"0x0000000000000000000000000000000000000003"
]
required_signatures = 2
"#;
        let expected = Config {
            address: "0000000000000000000000000000000000000001".into(),
            txs: Transactions::default(),
            main: NodeConfig {
                http: "".into(),
                contract: ContractConfig {
                    bin: include_str!("../../compiled_contracts/MainBridge.bin")
                        .from_hex()
                        .unwrap()
                        .into(),
                },
                poll_interval: Duration::from_secs(1),
                request_timeout: Duration::from_secs(5),
                required_confirmations: 12,
            },
            side: NodeConfig {
                http: "".into(),
                contract: ContractConfig {
                    bin: include_str!("../../compiled_contracts/SideBridge.bin")
                        .from_hex()
                        .unwrap()
                        .into(),
                },
                poll_interval: Duration::from_secs(1),
                request_timeout: Duration::from_secs(5),
                required_confirmations: 12,
            },
            authorities: Authorities {
                accounts: vec![
                    "0000000000000000000000000000000000000001".into(),
                    "0000000000000000000000000000000000000002".into(),
                    "0000000000000000000000000000000000000003".into(),
                ],
                required_signatures: 2,
            },
            estimated_gas_cost_of_withdraw: U256::from_dec_str("200000000").unwrap(),
            max_total_main_contract_balance: U256::from_dec_str("10000000000000000000").unwrap(),
            max_single_deposit_value: U256::from_dec_str("1000000000000000000").unwrap(),
        };

        let config = Config::load_from_str(toml).unwrap();
        assert_eq!(expected, config);
    }
}
