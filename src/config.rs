use std::path::{PathBuf, Path};
use std::fs;
use std::io::Read;
use std::time::Duration;
use web3::types::{Address, Bytes};
use error::{ResultExt, Error};
use {toml};

const DEFAULT_POLL_INTERVAL: u64 = 1;
const DEFAULT_CONFIRMATIONS: u64 = 12;

/// Application config.
#[derive(Debug, PartialEq, Clone)]
pub struct Config {
	pub mainnet: Node,
	pub testnet: Node,
	pub authorities: Authorities,
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
			mainnet: Node::from_load_struct(config.mainnet, NodeDefaults::mainnet())?,
			testnet: Node::from_load_struct(config.testnet, NodeDefaults::testnet())?,
			authorities: Authorities {
				accounts: config.authorities.accounts,
				required_signatures: config.authorities.required_signatures,
			}
		};

		Ok(result)
	}
}

#[derive(Debug, PartialEq, Clone)]
pub struct Node {
	pub account: Address,
	pub contract: ContractConfig,
	pub ipc: PathBuf,
	pub txs: Transactions,
	pub poll_interval: Duration,
	pub required_confirmations: u64,
}

struct NodeDefaults {
	ipc: PathBuf,
}

impl NodeDefaults {
	fn mainnet() -> Self {
		NodeDefaults {
			ipc: "".into(),
		}
	}

	fn testnet() -> Self {
		NodeDefaults {
			ipc: "".into(),
		}
	}
}

impl Node {
	fn from_load_struct(node: load::Node, defaults: NodeDefaults) -> Result<Node, Error> {
		let result = Node {
			account: node.account,
			contract: ContractConfig {
				bin: Bytes(fs::File::open(node.contract.bin)?.bytes().collect::<Result<_, _>>()?),
			},
			ipc: node.ipc.unwrap_or(defaults.ipc),
			txs: node.transactions.map(Transactions::from_load_struct).unwrap_or_default(),
			poll_interval: Duration::from_secs(node.poll_interval.unwrap_or(DEFAULT_POLL_INTERVAL)),
			required_confirmations: node.required_confirmations.unwrap_or(DEFAULT_CONFIRMATIONS),
		};
	
		Ok(result)
	}
}

#[derive(Debug, PartialEq, Default, Clone)]
pub struct Transactions {
	pub deploy: TransactionConfig,
	pub deposit: TransactionConfig,
}

impl Transactions {
	fn from_load_struct(cfg: load::Transactions) -> Self {
		Transactions {
			deploy: cfg.deploy.map(TransactionConfig::from_load_struct).unwrap_or_default(),
			deposit: cfg.deposit.map(TransactionConfig::from_load_struct).unwrap_or_default(),
		}
	}
}

#[derive(Debug, PartialEq, Default, Clone)]
pub struct TransactionConfig {
	pub gas: u64,
	pub gas_price: u64,
	pub value: u64,
}

impl TransactionConfig {
	fn from_load_struct(cfg: load::TransactionConfig) -> Self {
		TransactionConfig {
			gas: cfg.gas.unwrap_or_default(),
			gas_price: cfg.gas_price.unwrap_or_default(),
			value: cfg.value.unwrap_or_default(),
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
		pub mainnet: Node,
		pub testnet: Node,
		pub authorities: Authorities,
	}

	#[derive(Deserialize)]
	pub struct Node {
		pub account: Address,
		pub contract: ContractConfig,
		pub ipc: Option<PathBuf>,
		pub transactions: Option<Transactions>,
		pub poll_interval: Option<u64>,
		pub required_confirmations: Option<u64>,
	}

	#[derive(Deserialize)]
	pub struct Transactions {
		pub deploy: Option<TransactionConfig>,
		pub deposit: Option<TransactionConfig>,
	}

	#[derive(Deserialize)]
	pub struct TransactionConfig {
		pub gas: Option<u64>,
		pub gas_price: Option<u64>,
		pub value: Option<u64>,
	}

	#[derive(Deserialize)]
	pub struct ContractConfig {
		pub bin: PathBuf,
	}

	#[derive(Deserialize)]
	pub struct Authorities {
		pub accounts: Vec<Address>,
		pub required_signatures: u32,
	}
}

#[cfg(test)]
mod tests {
	use std::time::Duration;
	use super::{Config, Node, TransactionConfig, ContractConfig, Transactions, Authorities};

	#[test]
	fn load_full_setup_from_str() {
		let toml = r#"
[mainnet]
account = "0x1B68Cb0B50181FC4006Ce572cF346e596E51818b"
ipc = "/mainnet.ipc"
poll_interval = 2
required_confirmations = 100

[mainnet.contract]
bin = "contracts/EthereumBridge.bin"

[testnet]
account = "0x0000000000000000000000000000000000000001"
ipc = "/testnet.ipc"

[testnet.transactions]
deploy = { gas = 20, value = 15 }

[testnet.contract]
bin = "contracts/KovanBridge.bin"

[authorities]
accounts = [
	"0x0000000000000000000000000000000000000001",
	"0x0000000000000000000000000000000000000002",
	"0x0000000000000000000000000000000000000003"
]
required_signatures = 2
"#;

		let expected = Config {
			mainnet: Node {
				account: "0x1B68Cb0B50181FC4006Ce572cF346e596E51818b".parse().unwrap(),
				ipc: "/mainnet.ipc".into(),
				contract: ContractConfig {
					bin: include_bytes!("../contracts/EthereumBridge.bin").to_vec().into(),
				},
				txs: Transactions {
					deploy: TransactionConfig {
						gas: 0,
						gas_price: 0,
						value: 0,
					},
					deposit: TransactionConfig {
						gas: 0,
						gas_price: 0,
						value: 0,
					},
				},
				poll_interval: Duration::from_secs(2),
				required_confirmations: 100,
			},
			testnet: Node {
				account: "0x0000000000000000000000000000000000000001".parse().unwrap(),
				contract: ContractConfig {
					bin: include_bytes!("../contracts/KovanBridge.bin").to_vec().into(),
				},
				ipc: "/testnet.ipc".into(),
				txs: Transactions {
					deploy: TransactionConfig {
						gas: 20,
						gas_price: 0,
						value: 15,
					},
					deposit: TransactionConfig {
						gas: 0,
						gas_price: 0,
						value: 0,
					},
				},
				poll_interval: Duration::from_secs(1),
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

	#[test]
	fn laod_minimal_setup_from_str() {
		let toml = r#"
[mainnet]
account = "0x1B68Cb0B50181FC4006Ce572cF346e596E51818b"

[mainnet.contract]
bin = "contracts/EthereumBridge.bin"

[testnet]
account = "0x0000000000000000000000000000000000000001"

[testnet.contract]
bin = "contracts/KovanBridge.bin"

[authorities]
accounts = [
	"0x0000000000000000000000000000000000000001",
	"0x0000000000000000000000000000000000000002",
	"0x0000000000000000000000000000000000000003"
]
required_signatures = 2
"#;
		let expected = Config {
			mainnet: Node {
				account: "0x1B68Cb0B50181FC4006Ce572cF346e596E51818b".parse().unwrap(),
				ipc: "".into(),
				contract: ContractConfig {
					bin: include_bytes!("../contracts/EthereumBridge.bin").to_vec().into(),
				},
				txs: Transactions {
					deploy: TransactionConfig {
						gas: 0,
						gas_price: 0,
						value: 0,
					},
					deposit: TransactionConfig {
						gas: 0,
						gas_price: 0,
						value: 0,
					},
				},
				poll_interval: Duration::from_secs(1),
				required_confirmations: 12,
			},
			testnet: Node {
				account: "0x0000000000000000000000000000000000000001".parse().unwrap(),
				ipc: "".into(),
				contract: ContractConfig {
					bin: include_bytes!("../contracts/KovanBridge.bin").to_vec().into(),
				},
				txs: Transactions {
					deploy: TransactionConfig {
						gas: 0,
						gas_price: 0,
						value: 0,
					},
					deposit: TransactionConfig {
						gas: 0,
						gas_price: 0,
						value: 0,
					},
				},
				poll_interval: Duration::from_secs(1),
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
