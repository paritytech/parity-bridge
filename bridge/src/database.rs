/// the state of a bridge node process and ways to persist it

use std::path::{Path, PathBuf};
use std::{fs, io, str, fmt};
use std::io::{Read, Write};
use web3::types::{Address, TransactionReceipt};
use toml;
use error::{Error, ErrorKind, ResultExt};

/// bridge process state
#[derive(Debug, PartialEq, Deserialize, Serialize, Default, Clone, Copy)]
pub struct State {
    /// Address of home contract.
    pub home_contract_address: Address,
    /// Address of foreign contract.
    pub foreign_contract_address: Address,
    /// Number of block at which home contract has been deployed.
    pub home_deploy: u64,
    /// Number of block at which foreign contract has been deployed.
    pub foreign_deploy: u64,
    /// Number of last block which has been checked for deposit relays.
    pub checked_deposit_relay: u64,
    /// Number of last block which has been checked for withdraw relays.
    pub checked_withdraw_relay: u64,
    /// Number of last block which has been checked for withdraw confirms.
    pub checked_withdraw_confirm: u64,
}

impl State {
    /// creates initial state for the bridge processes
    /// from transaction receipts of contract deployments
    pub fn from_transaction_receipts(
        home_contract_deployment_receipt: &TransactionReceipt,
        foreign_contract_deployment_receipt: &TransactionReceipt,
    ) -> Self {
        Self {
            home_contract_address: home_contract_deployment_receipt
                .contract_address
                .expect("contract creation receipt must have an address; qed"),
            foreign_contract_address: foreign_contract_deployment_receipt
                .contract_address
                .expect("contract creation receipt must have an address; qed"),
            home_deploy: home_contract_deployment_receipt.block_number.low_u64(),
            foreign_deploy: foreign_contract_deployment_receipt.block_number.low_u64(),
            checked_deposit_relay: home_contract_deployment_receipt.block_number.low_u64(),
            checked_withdraw_relay: foreign_contract_deployment_receipt.block_number.low_u64(),
            checked_withdraw_confirm: foreign_contract_deployment_receipt.block_number.low_u64(),
        }
    }
}

impl State {
    /// write state to a `std::io::write`
    pub fn write<W: Write>(&self, mut write: W) -> Result<(), Error> {
        let serialized = toml::to_string(self).expect("serialization can't fail. q.e.d.");
        write.write_all(serialized.as_bytes())?;
        Ok(())
    }
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&toml::to_string(self).expect("serialization can't fail; qed"))
    }
}

/// persistence for a `State`
pub trait Database {
    fn read(&self) -> State;
    /// persist `state` to the database
    fn write(&mut self, state: &State) -> Result<(), Error>;
}

/// `State` stored in a TOML file
pub struct TomlFileDatabase {
    filepath: PathBuf,
    state: State,
}

impl TomlFileDatabase {
    /// create `TomlFileDatabase` backed by file at `filepath`
    pub fn from_path<P: AsRef<Path>>(filepath: P) -> Result<Self, Error> {
        let mut file = match fs::File::open(&filepath) {
            Ok(file) => file,
            Err(ref err) if err.kind() == io::ErrorKind::NotFound => {
                return Err(ErrorKind::MissingFile(format!("{:?}", filepath.as_ref())).into())
            }
            Err(err) => return Err(err).chain_err(|| "Cannot open database"),
        };

        let mut buffer = String::new();
        file.read_to_string(&mut buffer)?;
        let state: State = toml::from_str(&buffer).chain_err(|| "Cannot parse database")?;
        Ok(Self {
            filepath: filepath.as_ref().to_path_buf(),
            state
        })
    }
}

impl Database for TomlFileDatabase {
    fn read(&self) -> State {
        self.state
    }

    fn write(&mut self, state: &State) -> Result<(), Error> {
        if self.state != *state {
            self.state = state.clone();

            let file = fs::OpenOptions::new()
                .write(true)
                .create(true)
                .open(&self.filepath)?;

            self.state.write(file)?;
        }
        Ok(())
    }
}

// #[cfg(test)]
// mod tests {
//     extern crate tempdir;
//     use self::tempdir::TempDir;
//     use database::Database;
//
//     #[test]
//     fn test_file_backend() {
//         let tempdir = TempDir::new("test_file_backend").unwrap();
//         let mut path = tempdir.path().to_owned();
//         path.push("db");
//         let mut backend = FileBackend {
//             path: path.clone(),
//             database: Database::default(),
//         };
//
//         backend.save(vec![BridgeChecked::DepositRelay(1)]).unwrap();
//         assert_eq!(1, backend.database.checked_deposit_relay);
//         assert_eq!(0, backend.database.checked_withdraw_confirm);
//         assert_eq!(0, backend.database.checked_withdraw_relay);
//         backend
//             .save(vec![
//                 BridgeChecked::DepositRelay(2),
//                 BridgeChecked::WithdrawConfirm(3),
//                 BridgeChecked::WithdrawRelay(2),
//             ])
//             .unwrap();
//         assert_eq!(2, backend.database.checked_deposit_relay);
//         assert_eq!(3, backend.database.checked_withdraw_confirm);
//         assert_eq!(2, backend.database.checked_withdraw_relay);
//
//         let loaded = Database::load(path).unwrap();
//         assert_eq!(backend.database, loaded);
//     }
// }
