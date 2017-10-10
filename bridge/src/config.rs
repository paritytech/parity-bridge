use std::path::{PathBuf, Path};
use std::fs;
use std::io::Read;
use std::time::Duration;
use rustc_hex::FromHex;
use web3::types::{Address, Bytes};
use error::{ResultExt, Error};
use {toml};

const DEFAULT_POLL_INTERVAL: u64 = 1;
const DEFAULT_CONFIRMATIONS: u64 = 12;
const DEFAULT_TIMEOUT: u64 = 5;

/// Application config.
#[derive(Debug, PartialEq, Clone)]
pub struct Config {
	pub home: Node,
	pub foreign: Node,
	pub authorities: Authorities,
	pub txs: Transactions,
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
		};

		Ok(result)
	}
}

#[derive(Debug, PartialEq, Clone)]
pub struct Node {
	pub account: Address,
	pub contract: ContractConfig,
	pub ipc: PathBuf,
	pub request_timeout: Duration,
	pub poll_interval: Duration,
	pub required_confirmations: u64,
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
			ipc: node.ipc,
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

	#[derive(Deserialize)]
	#[serde(deny_unknown_fields)]
	pub struct Config {
		pub home: Node,
		pub foreign: Node,
		pub authorities: Authorities,
		pub transactions: Option<Transactions>,
	}

	#[derive(Deserialize)]
	#[serde(deny_unknown_fields)]
	pub struct Node {
		pub account: Address,
		pub contract: ContractConfig,
		pub ipc: PathBuf,
		pub request_timeout: Option<u64>,
		pub poll_interval: Option<u64>,
		pub required_confirmations: Option<u64>,
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

	#[test]
	fn load_full_setup_from_str() {
		let toml = r#"
[home]
account = "0x1B68Cb0B50181FC4006Ce572cF346e596E51818b"
ipc = "/home.ipc"
poll_interval = 2
required_confirmations = 100

[home.contract]
bin = "../contracts/HomeBridge.bin"

[foreign]
account = "0x0000000000000000000000000000000000000001"
ipc = "/foreign.ipc"

[foreign.contract]
bin = "../contracts/ForeignBridge.bin"

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
				account: "0x1B68Cb0B50181FC4006Ce572cF346e596E51818b".parse().unwrap(),
				ipc: "/home.ipc".into(),
				contract: ContractConfig {
					bin: include_str!("../../contracts/HomeBridge.bin").from_hex().unwrap().into(),
				},
				poll_interval: Duration::from_secs(2),
				request_timeout: Duration::from_secs(5),
				required_confirmations: 100,
			},
			foreign: Node {
				account: "0x0000000000000000000000000000000000000001".parse().unwrap(),
				contract: ContractConfig {
					bin: include_str!("../../contracts/ForeignBridge.bin").from_hex().unwrap().into(),
				},
				ipc: "/foreign.ipc".into(),
				poll_interval: Duration::from_secs(1),
				request_timeout: Duration::from_secs(5),
				required_confirmations: 12,
			},
			authorities: Authorities {
				accounts: vec![
					"0x0000000000000000000000000000000000000001".parse().unwrap(),
					"0x0000000000000000000000000000000000000002".parse().unwrap(),
					"0x0000000000000000000000000000000000000003".parse().unwrap(),
				],
				required_signatures: 2,
			}
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
[home]
account = "0x1B68Cb0B50181FC4006Ce572cF346e596E51818b"
ipc = ""

[home.contract]
bin = "../contracts/HomeBridge.bin"

[foreign]
account = "0x0000000000000000000000000000000000000001"
ipc = ""

[foreign.contract]
bin = "../contracts/ForeignBridge.bin"

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
				account: "0x1B68Cb0B50181FC4006Ce572cF346e596E51818b".parse().unwrap(),
				ipc: "".into(),
				contract: ContractConfig {
					bin: include_str!("../../contracts/HomeBridge.bin").from_hex().unwrap().into(),
				},
				poll_interval: Duration::from_secs(1),
				request_timeout: Duration::from_secs(5),
				required_confirmations: 12,
			},
			foreign: Node {
				account: "0x0000000000000000000000000000000000000001".parse().unwrap(),
				ipc: "".into(),
				contract: ContractConfig {
					bin: include_str!("../../contracts/ForeignBridge.bin").from_hex().unwrap().into(),
				},
				poll_interval: Duration::from_secs(1),
				request_timeout: Duration::from_secs(5),
				required_confirmations: 12,
			},
			authorities: Authorities {
				accounts: vec![
					"0x0000000000000000000000000000000000000001".parse().unwrap(),
					"0x0000000000000000000000000000000000000002".parse().unwrap(),
					"0x0000000000000000000000000000000000000003".parse().unwrap(),
				],
				required_signatures: 2,
			}
		};

		let config = Config::load_from_str(toml).unwrap();
		assert_eq!(expected, config);
	}
}
