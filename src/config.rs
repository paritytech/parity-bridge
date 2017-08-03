use std::path::{PathBuf, Path};
use std::fs;
use std::io::Read;
use std::time::Duration;
use web3::types::Address;
use error::{ResultExt, Error};
use toml;

const DEFAULT_POLL_INTERVAL: u64 = 1;
const DEFAULT_CONFIRMATIONS: u64 = 12;

/// Application config.
#[derive(Debug, PartialEq)]
pub struct Config {
	pub mainnet: Node,
	pub testnet: Node,
}

impl Default for Config {
	fn default() -> Self {
		Config {
			mainnet: Node::default_mainnet(),
			testnet: Node::default_testnet(),
		}
	}
}

impl From<load::Config> for Config {
	fn from(config: load::Config) -> Self {
		let default = Self::default();
		Config {
			mainnet: Node {
				account: config.mainnet.account,
				ipc: config.mainnet.ipc.unwrap_or(default.mainnet.ipc),
				deploy_tx: default.mainnet.deploy_tx.with(config.mainnet.deploy_tx),
				poll_interval: config.mainnet.poll_interval.map(Duration::from_secs).unwrap_or(default.mainnet.poll_interval),
				required_confirmations: config.mainnet.required_confirmations.unwrap_or(default.mainnet.required_confirmations),
			},
			testnet: Node {
				account: config.testnet.account,
				ipc: config.testnet.ipc.unwrap_or(default.testnet.ipc),
				deploy_tx: default.testnet.deploy_tx.with(config.testnet.deploy_tx),
				poll_interval: config.testnet.poll_interval.map(Duration::from_secs).unwrap_or(default.testnet.poll_interval),
				required_confirmations: config.testnet.required_confirmations.unwrap_or(default.testnet.required_confirmations),
			}
		}
	}
}

impl Config {
	pub fn load<P: AsRef<Path>>(path: P) -> Result<Config, Error> {
		let mut file = fs::File::open(path).chain_err(|| "Cannot open config")?;
		let mut buffer = String::new();
		file.read_to_string(&mut buffer);
		Self::load_from_str(&buffer)
	}

	fn load_from_str(s: &str) -> Result<Config, Error> {
		let config: load::Config = toml::from_str(s).chain_err(|| "Cannot parse config")?;
		Ok(config.into())
	}
}

#[derive(Debug, PartialEq)]
pub struct Node {
	pub account: Address,
	pub ipc: PathBuf,
	pub deploy_tx: TransactionConfig,
	pub poll_interval: Duration,
	pub required_confirmations: u64,
}

impl Node {
	fn default_mainnet() -> Self {
		// TODO: hardcode default mainnet ipc path
		Node {
			account: Address::default(),
			ipc: "".into(),
			deploy_tx: TransactionConfig::deploy_mainnet(),
			poll_interval: Duration::from_secs(DEFAULT_POLL_INTERVAL),
			required_confirmations: DEFAULT_CONFIRMATIONS,
		}
	}

	fn default_testnet() -> Self {
		// TODO: hardcode default testnet ipc path
		Node {
			account: Address::default(),
			ipc: "".into(),
			deploy_tx: TransactionConfig::deploy_testnet(),
			poll_interval: Duration::from_secs(DEFAULT_POLL_INTERVAL),
			required_confirmations: DEFAULT_CONFIRMATIONS,
		}
	}
}

#[derive(Debug, PartialEq)]
pub struct TransactionConfig {
	pub gas: u64,
	pub gas_price: u64,
	pub value: u64,
}

impl TransactionConfig {
	fn with(&self, loaded: Option<load::TransactionConfig>) -> Self {
		let loaded_ref = loaded.as_ref();
		TransactionConfig {
			gas: loaded_ref.and_then(|tx| tx.gas).unwrap_or(self.gas),
			gas_price: loaded_ref.and_then(|tx| tx.gas_price).unwrap_or(self.gas_price),
			value: loaded_ref.and_then(|tx| tx.value).unwrap_or(self.value),
		}
	}

	fn deploy_mainnet() -> Self {
		// TODO: values
		TransactionConfig {
			gas: 0,
			gas_price: 0,
			value: 0,
		}
	}

	fn deploy_testnet() -> Self {
		// TODO: values
		TransactionConfig {
			gas: 0,
			gas_price: 0,
			value: 0,
		}
	}
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
	}

	#[derive(Deserialize)]
	pub struct Node {
		pub account: Address,
		pub ipc: Option<PathBuf>,
		pub deploy_tx: Option<TransactionConfig>,
		pub poll_interval: Option<u64>,
		pub required_confirmations: Option<u64>,
	}

	#[derive(Deserialize)]
	pub struct TransactionConfig {
		pub gas: Option<u64>,
		pub gas_price: Option<u64>,
		pub value: Option<u64>,
	}
}

#[cfg(test)]
mod tests {
	use std::time::Duration;
	use super::{Config, Node, TransactionConfig};

	#[test]
	fn load_full_setup_from_str() {
		let toml = r#"
[mainnet]
account = "0x1B68Cb0B50181FC4006Ce572cF346e596E51818b"
ipc = "/mainnet.ipc"
poll_interval = 2
required_confirmations = 100

[testnet]
account = "0x0000000000000000000000000000000000000001"
ipc = "/testnet.ipc"
deploy_tx = { gas = 20, value = 15 }
"#;

		let expected = Config {
			mainnet: Node {
				account: "0x1B68Cb0B50181FC4006Ce572cF346e596E51818b".parse().unwrap(),
				ipc: "/mainnet.ipc".into(),
				deploy_tx: TransactionConfig {
					gas: 0,
					gas_price: 1,
					value: 0,
				},
				poll_interval: Duration::from_secs(2),
				required_confirmations: 100,
			},
			testnet: Node {
				account: "0x0000000000000000000000000000000000000001".parse().unwrap(),
				ipc: "/testnet.ipc".into(),
				deploy_tx: TransactionConfig {
					gas: 20,
					gas_price: 3,
					value: 15,
				},
				poll_interval: Duration::from_secs(1),
				required_confirmations: 12,
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
[testnet]
account = "0x0000000000000000000000000000000000000001"
"#;
		let mut expected = Config::default();
		expected.mainnet.account = "0x1B68Cb0B50181FC4006Ce572cF346e596E51818b".parse().unwrap();
		expected.testnet.account = "0x0000000000000000000000000000000000000001".parse().unwrap();
		let config = Config::load_from_str(toml).unwrap();
		assert_eq!(expected, config);
	}
}
