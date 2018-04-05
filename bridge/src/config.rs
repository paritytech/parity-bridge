use std::path::{PathBuf, Path};
use std::fs;
use std::io::Read;
use std::time::Duration;
use rustc_hex::FromHex;
use web3::types::{Address, Bytes};
use ethereum_types::U256;
use error::{ResultExt, Error};
use {toml};

const DEFAULT_POLL_INTERVAL: u64 = 1;
const DEFAULT_CONFIRMATIONS: usize = 12;
const DEFAULT_TIMEOUT: u64 = 5;

/// Application config.
#[derive(Debug, PartialEq, Clone)]
pub struct Config {
	pub home: Node,
	pub foreign: Node,
	pub authorities: Authorities,
	pub txs: Transactions,
	pub estimated_gas_cost_of_withdraw: U256,
	pub max_total_home_contract_balance: U256,
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
			home: Node::from_load_struct(config.home)?,
			foreign: Node::from_load_struct(config.foreign)?,
			authorities: Authorities {
				accounts: config.authorities.accounts,
				required_signatures: config.authorities.required_signatures,
			},
			txs: config.transactions.map(Transactions::from_load_struct).unwrap_or_default(),
			estimated_gas_cost_of_withdraw: config.estimated_gas_cost_of_withdraw,
			max_total_home_contract_balance: config.max_total_home_contract_balance,
			max_single_deposit_value: config.max_single_deposit_value,
		};

		Ok(result)
	}
}

#[derive(Debug, PartialEq, Clone)]
pub struct Node {
	pub account: Address,
	pub contract: ContractConfig,
	pub jsonrpc: String,
	pub request_timeout: Duration,
	pub poll_interval: Duration,
	pub required_confirmations: usize,
}

impl Node {
	fn from_load_struct(node: load::Node) -> Result<Node, Error> {
		let result = Node {
			account: node.account,
			contract: ContractConfig {
				bin: {
					let mut read = String::new();
					let mut file = fs::File::open(node.contract.bin)?;
					file.read_to_string(&mut read)?;
					Bytes(read.from_hex()?)
				}
			},
			jsonrpc: node.jsonrpc,
			request_timeout: Duration::from_secs(node.request_timeout.unwrap_or(DEFAULT_TIMEOUT)),
			poll_interval: Duration::from_secs(node.poll_interval.unwrap_or(DEFAULT_POLL_INTERVAL)),
			required_confirmations: node.required_confirmations.unwrap_or(DEFAULT_CONFIRMATIONS),
		};

		Ok(result)
	}
}

#[derive(Debug, PartialEq, Default, Clone)]
pub struct Transactions {
	pub home_deploy: TransactionConfig,
	pub foreign_deploy: TransactionConfig,
	pub deposit_relay: TransactionConfig,
	pub withdraw_confirm: TransactionConfig,
	pub withdraw_relay: TransactionConfig,
}

impl Transactions {
	fn from_load_struct(cfg: load::Transactions) -> Self {
		Transactions {
			home_deploy: cfg.home_deploy.map(TransactionConfig::from_load_struct).unwrap_or_default(),
			foreign_deploy: cfg.foreign_deploy.map(TransactionConfig::from_load_struct).unwrap_or_default(),
			deposit_relay: cfg.deposit_relay.map(TransactionConfig::from_load_struct).unwrap_or_default(),
			withdraw_confirm: cfg.withdraw_confirm.map(TransactionConfig::from_load_struct).unwrap_or_default(),
			withdraw_relay: cfg.withdraw_relay.map(TransactionConfig::from_load_struct).unwrap_or_default(),
		}
	}
}

#[derive(Debug, PartialEq, Default, Clone)]
pub struct TransactionConfig {
	pub gas: u64,
	pub gas_price: u64,
}

impl TransactionConfig {
	fn from_load_struct(cfg: load::TransactionConfig) -> Self {
		TransactionConfig {
			gas: cfg.gas.unwrap_or_default(),
			gas_price: cfg.gas_price.unwrap_or_default(),
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
	use std::path::PathBuf;
	use web3::types::Address;
	use ethereum_types::U256;
	use serde::{Deserialize, Deserializer};
	use serde::de::Error;

	/// the toml crate parses integer literals as `i64`.
	/// certain config options (example: `max_total_home_contract_balance`)
	/// frequently don't fit into `i64`.
	/// workaround: put them in string literals, use this custom
	/// deserializer and parse them as U256.
	fn deserialize_u256<'de, D>(deserializer: D) -> Result<U256, D::Error>
		where
			D: Deserializer<'de>,
	{
		let s: &str = Deserialize::deserialize(deserializer)?;
		U256::from_dec_str(s).map_err(|_| D::Error::custom("failed to parse U256 from dec str"))
	}

	#[derive(Deserialize)]
	#[serde(deny_unknown_fields)]
	pub struct Config {
		pub home: Node,
		pub foreign: Node,
		pub authorities: Authorities,
		pub transactions: Option<Transactions>,
		#[serde(deserialize_with="deserialize_u256")]
		pub estimated_gas_cost_of_withdraw: U256,
		#[serde(deserialize_with="deserialize_u256")]
		pub max_total_home_contract_balance: U256,
		#[serde(deserialize_with="deserialize_u256")]
		pub max_single_deposit_value: U256,
	}

	#[derive(Deserialize)]
	#[serde(deny_unknown_fields)]
	pub struct Node {
		pub account: Address,
		pub contract: ContractConfig,
		pub jsonrpc: String,
		pub request_timeout: Option<u64>,
		pub poll_interval: Option<u64>,
		pub required_confirmations: Option<usize>,
	}

	#[derive(Deserialize)]
	#[serde(deny_unknown_fields)]
	pub struct Transactions {
		pub home_deploy: Option<TransactionConfig>,
		pub foreign_deploy: Option<TransactionConfig>,
		pub deposit_relay: Option<TransactionConfig>,
		pub withdraw_confirm: Option<TransactionConfig>,
		pub withdraw_relay: Option<TransactionConfig>,
	}

	#[derive(Deserialize)]
	#[serde(deny_unknown_fields)]
	pub struct TransactionConfig {
		pub gas: Option<u64>,
		pub gas_price: Option<u64>,
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
	use std::time::Duration;
	use rustc_hex::FromHex;
	use super::{Config, Node, ContractConfig, Transactions, Authorities, TransactionConfig};
	use ethereum_types::U256;

	#[test]
	fn load_full_setup_from_str() {
		let toml = r#"
estimated_gas_cost_of_withdraw = "100000"
max_total_home_contract_balance = "10000000000000000000"
max_single_deposit_value = "1000000000000000000"

[home]
account = "0x1B68Cb0B50181FC4006Ce572cF346e596E51818b"
jsonrpc = "http://localhost:8545"
poll_interval = 2
required_confirmations = 100

[home.contract]
bin = "../compiled_contracts/HomeBridge.bin"

[foreign]
account = "0x0000000000000000000000000000000000000001"
jsonrpc = "http://localhost:8546"

[foreign.contract]
bin = "../compiled_contracts/ForeignBridge.bin"

[authorities]
accounts = [
	"0x0000000000000000000000000000000000000001",
	"0x0000000000000000000000000000000000000002",
	"0x0000000000000000000000000000000000000003"
]
required_signatures = 2

[transactions]
home_deploy = { gas = 20 }
"#;

		let mut expected = Config {
			txs: Transactions::default(),
			home: Node {
				account: "1B68Cb0B50181FC4006Ce572cF346e596E51818b".into(),
				jsonrpc: "http://localhost:8545".into(),
				contract: ContractConfig {
					bin: include_str!("../../compiled_contracts/HomeBridge.bin").from_hex().unwrap().into(),
				},
				poll_interval: Duration::from_secs(2),
				request_timeout: Duration::from_secs(5),
				required_confirmations: 100,
			},
			foreign: Node {
				account: "0000000000000000000000000000000000000001".into(),
				contract: ContractConfig {
					bin: include_str!("../../compiled_contracts/ForeignBridge.bin").from_hex().unwrap().into(),
				},
				jsonrpc: "http://localhost:8546".into(),
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
			max_total_home_contract_balance: U256::from_dec_str("10000000000000000000").unwrap(),
			max_single_deposit_value: U256::from_dec_str("1000000000000000000").unwrap(),
		};

		expected.txs.home_deploy = TransactionConfig {
			gas: 20,
			gas_price: 0,
		};

		let config = Config::load_from_str(toml).unwrap();
		assert_eq!(expected, config);
	}

	#[test]
	fn load_minimal_setup_from_str() {
		let toml = r#"
estimated_gas_cost_of_withdraw = "200000000"
max_total_home_contract_balance = "10000000000000000000"
max_single_deposit_value = "1000000000000000000"

[home]
account = "0x1B68Cb0B50181FC4006Ce572cF346e596E51818b"
jsonrpc = ""

[home.contract]
bin = "../compiled_contracts/HomeBridge.bin"

[foreign]
account = "0x0000000000000000000000000000000000000001"
jsonrpc = ""

[foreign.contract]
bin = "../compiled_contracts/ForeignBridge.bin"

[authorities]
accounts = [
	"0x0000000000000000000000000000000000000001",
	"0x0000000000000000000000000000000000000002",
	"0x0000000000000000000000000000000000000003"
]
required_signatures = 2
"#;
		let expected = Config {
			txs: Transactions::default(),
			home: Node {
				account: "1B68Cb0B50181FC4006Ce572cF346e596E51818b".into(),
				jsonrpc: "".into(),
				contract: ContractConfig {
					bin: include_str!("../../compiled_contracts/HomeBridge.bin").from_hex().unwrap().into(),
				},
				poll_interval: Duration::from_secs(1),
				request_timeout: Duration::from_secs(5),
				required_confirmations: 12,
			},
			foreign: Node {
				account: "0000000000000000000000000000000000000001".into(),
				jsonrpc: "".into(),
				contract: ContractConfig {
					bin: include_str!("../../compiled_contracts/ForeignBridge.bin").from_hex().unwrap().into(),
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
			max_total_home_contract_balance: U256::from_dec_str("10000000000000000000").unwrap(),
			max_single_deposit_value: U256::from_dec_str("1000000000000000000").unwrap(),
		};

		let config = Config::load_from_str(toml).unwrap();
		assert_eq!(expected, config);
	}
}
