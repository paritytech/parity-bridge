use std::path::Path;
use std::{io, str, fs, fmt};
use std::io::{Read, Write};
use web3::types::Address;
use toml;
use error::{Error, ResultExt, ErrorKind};

/// Application "database".
#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct Database {
	pub mainnet: BlockchainState,
	pub testnet: BlockchainState,
}

impl str::FromStr for Database {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		toml::from_str(s).chain_err(|| "Cannot parse database")
	}
}

impl fmt::Display for Database {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.write_str(&toml::to_string(self).expect("serialization can't fail; qed"))
	}
}

impl Database {
	pub fn load<P: AsRef<Path>>(path: P) -> Result<Database, Error> {
		let mut file = match fs::File::open(&path) {
			Ok(file) => file,
			Err(ref err) if err.kind() == io::ErrorKind::NotFound => return Err(ErrorKind::MissingFile(format!("{:?}", path.as_ref())).into()),
			Err(err) => return Err(err).chain_err(|| "Cannot open database"),
		};

		let mut buffer = String::new();
		file.read_to_string(&mut buffer);
		buffer.parse()
	}

	pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), Error> {
		let mut file = fs::File::open(path).chain_err(|| "Cannot save database")?;
		file.write_all(self.to_string().as_bytes())?;
		Ok(())
	}
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct BlockchainState {
	/// Block number at which bridge has been deployed.
	pub deploy_block_number: u64,
	/// Bridge contract address.
	pub bridge_contract_address: Address,
	/// Last handled block number
	pub last_block_number: u64,
}

impl BlockchainState {
	pub fn new(block_number: u64, contract_address: Address) -> Self {
		BlockchainState {
			deploy_block_number: block_number,
			bridge_contract_address: contract_address,
			last_block_number: block_number,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::{Database, BlockchainState};

	#[test]
	fn laod_databse_from_str() {
		let toml = r#"
[mainnet]
deploy_block_number = 100
bridge_contract_address = "0x49edf201c1e139282643d5e7c6fb0c7219ad1db7"
last_block_number = 120
[testnet]
deploy_block_number = 101
bridge_contract_address = "0x49edf201c1e139282643d5e7c6fb0c7219ad1db8"
last_block_number = 121
"#;

		let expected = Database {
			mainnet: BlockchainState {
				deploy_block_number: 100,
				bridge_contract_address: "0x49edf201c1e139282643d5e7c6fb0c7219ad1db7".parse().unwrap(),
				last_block_number: 120,
			},
			testnet: BlockchainState {
				deploy_block_number: 101,
				bridge_contract_address: "0x49edf201c1e139282643d5e7c6fb0c7219ad1db8".parse().unwrap(),
				last_block_number: 121,
			},
		};

		let database = toml.parse().unwrap();
		assert_eq!(expected, database);
	}

	#[test]
	fn save_database_to_string() {
		let database = Database {
			mainnet: BlockchainState {
				deploy_block_number: 100,
				bridge_contract_address: "0x49edf201c1e139282643d5e7c6fb0c7219ad1db7".parse().unwrap(),
				last_block_number: 120,
			},
			testnet: BlockchainState {
				deploy_block_number: 101,
				bridge_contract_address: "0x49edf201c1e139282643d5e7c6fb0c7219ad1db8".parse().unwrap(),
				last_block_number: 121,
			},
		};
		
		let expected = r#"[mainnet]
deploy_block_number = 100
bridge_contract_address = "0x49edf201c1e139282643d5e7c6fb0c7219ad1db7"
last_block_number = 120

[testnet]
deploy_block_number = 101
bridge_contract_address = "0x49edf201c1e139282643d5e7c6fb0c7219ad1db8"
last_block_number = 121
"#;

		let raw = database.to_string();
		assert_eq!(expected, &raw);

	}
}
