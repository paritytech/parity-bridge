use std::path::Path;
use std::fs;
use std::io::{Read, Write};
use toml;
use error::DatabaseError;

/// Application "database".
#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct Data {
	pub mainnet: BlockchainState,
	pub testnet: BlockchainState,
}

impl Data {
	pub fn load<P: AsRef<Path>>(path: P) -> Result<Data, DatabaseError> {
		let mut file = fs::File::open(path)?;
		let mut buffer = String::new();
		file.read_to_string(&mut buffer);
		Self::load_from_str(&buffer)
	}

	fn load_from_str(s: &str) -> Result<Data, DatabaseError> {
		let data = toml::from_str(s)?;
		Ok(data)
	}

	pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), DatabaseError> {
		let mut file = fs::File::open(path)?;
		file.write_all(self.save_to_string().as_bytes())?;
		Ok(())
	}

	fn save_to_string(&self) -> String {
		toml::to_string(self).expect("serialization can't fail; qed")
	}
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct BlockchainState {
	/// Block number at which bridge has been deployed.
	pub deploy_block_number: u64,
	/// Bridge contract address.
	pub bridge_contract_address: String,
	/// Last handled block number
	pub last_block_number: u64,
}

impl BlockchainState {
	pub fn new(block_number: u64, contract_address: String) -> Self {
		BlockchainState {
			deploy_block_number: block_number,
			bridge_contract_address: contract_address,
			last_block_number: block_number,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::{Data, BlockchainState};

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

		let expected = Data {
			mainnet: BlockchainState {
				deploy_block_number: 100,
				bridge_contract_address: "0x49edf201c1e139282643d5e7c6fb0c7219ad1db7".to_owned(),
				last_block_number: 120,
			},
			testnet: BlockchainState {
				deploy_block_number: 101,
				bridge_contract_address: "0x49edf201c1e139282643d5e7c6fb0c7219ad1db8".to_owned(),
				last_block_number: 121,
			},
		};
		let database = Data::load_from_str(toml).unwrap();
		assert_eq!(expected, database);
	}

	#[test]
	fn save_database_to_string() {
		let database = Data {
			mainnet: BlockchainState {
				deploy_block_number: 100,
				bridge_contract_address: "0x49edf201c1e139282643d5e7c6fb0c7219ad1db7".to_owned(),
				last_block_number: 120,
			},
			testnet: BlockchainState {
				deploy_block_number: 101,
				bridge_contract_address: "0x49edf201c1e139282643d5e7c6fb0c7219ad1db8".to_owned(),
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

		let raw = database.save_to_string();
		assert_eq!(expected, &raw);

	}
}
