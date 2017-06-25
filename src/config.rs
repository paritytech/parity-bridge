use std::path::{PathBuf, Path};
use std::fs;
use std::io::Read;
use error::SetupError;
use toml;

/// Application config.
#[derive(Debug, PartialEq)]
pub struct Config {
	mainnet: Node,
	testnet: Node,
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
				ipc: config.mainnet.ipc.unwrap_or(default.mainnet.ipc),
			},
			testnet: Node {
				ipc: config.testnet.ipc.unwrap_or(default.testnet.ipc),
			}
		}
	}
}

impl Config {
	pub fn load<P: AsRef<Path>>(path: P) -> Result<Config, SetupError> {
		let mut file = fs::File::open(path)?;
		let mut buffer = String::new();
		file.read_to_string(&mut buffer);
		Self::load_from_str(&buffer)
	}

	fn load_from_str(s: &str) -> Result<Config, SetupError> {
		let config: load::Config = toml::from_str(s)?;
		Ok(config.into())
	}
}

#[derive(Debug, PartialEq)]
struct Node {
	ipc: PathBuf,
}

impl Node {
	fn default_mainnet() -> Self {
		// TODO: hardcode default mainnet ipc path
		Node {
			ipc: "".into(),
		}
	}

	fn default_testnet() -> Self {
		// TODO: hardcode default testnet ipc path
		Node {
			ipc: "".into(),
		}
	}
}

/// Some config values may not be defined in `toml` file, but they should be specified at runtime.
/// `load` module separates `Config` representation in file with optional from the one used 
/// in application.
mod load {
	use std::path::PathBuf;

	#[derive(Deserialize)]
	pub struct Config {
		pub mainnet: Node,
		pub testnet: Node,
	}

	#[derive(Deserialize)]
	pub struct Node {
		pub ipc: Option<PathBuf>,
	}
}

#[cfg(test)]
mod tests {
	use super::{Config, Node};

	#[test]
	fn load_full_setup_from_str() {
		let toml = r#"
[mainnet]
ipc = "/mainnet.ipc"

[testnet]
ipc = "/testnet.ipc"
"#;
		
		let expected = Config {
			mainnet: Node {
				ipc: "/mainnet.ipc".into(),
			},
			testnet: Node {
				ipc: "/testnet.ipc".into(),
			}
		};
		let config = Config::load_from_str(toml).unwrap();
		assert_eq!(expected, config);
	}

	#[test]
	fn laod_minimal_setup_from_str() {
		let toml = r#"
[mainnet]
[testnet]
"#;
		let expected = Config::default();
		let config = Config::load_from_str(toml).unwrap();
		assert_eq!(expected, config);
	}
}
