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

//! concerning reading/writing `State` from/to toml file

use error::{Error, ErrorKind, ResultExt};
use std::io::{Read, Write};
/// the state of a bridge node process and ways to persist it
use std::path::{Path, PathBuf};
use std::{fmt, fs, io, str};
use toml;
use web3::types::{Address, TransactionReceipt};

/// bridge process state
#[derive(Debug, PartialEq, Deserialize, Serialize, Default, Clone)]
pub struct State {
    /// Address of home contract.
    pub main_contract_address: Address,
    /// Address of foreign contract.
    pub side_contract_address: Address,
    /// Number of block at which home contract has been deployed.
    pub main_deployed_at_block: u64,
    /// Number of block at which foreign contract has been deployed.
    pub side_deployed_at_block: u64,
    /// Number of last block which has been checked for deposit relays.
    pub last_main_to_side_sign_at_block: u64,
    /// Number of last block which has been checked for withdraw relays.
    pub last_side_to_main_signatures_at_block: u64,
    /// Number of last block which has been checked for withdraw confirms.
    pub last_side_to_main_sign_at_block: u64,
}

impl State {
    /// creates initial state for the bridge processes
    /// from transaction receipts of contract deployments
    pub fn from_transaction_receipts(
        main_contract_deployment_receipt: &TransactionReceipt,
        side_contract_deployment_receipt: &TransactionReceipt,
    ) -> Self {
        let main_block_number = main_contract_deployment_receipt
            .block_number
            .expect("main contract creation receipt must have a block number; qed")
            .as_u64();

        let side_block_number = side_contract_deployment_receipt
            .block_number
            .expect("main contract creation receipt must have a block number; qed")
            .as_u64();

        Self {
            main_contract_address: main_contract_deployment_receipt
                .contract_address
                .expect("main contract creation receipt must have an address; qed"),
            side_contract_address: side_contract_deployment_receipt
                .contract_address
                .expect("side contract creation receipt must have an address; qed"),
            main_deployed_at_block: main_block_number,
            side_deployed_at_block: side_block_number,
            last_main_to_side_sign_at_block: main_block_number,
            last_side_to_main_sign_at_block: side_block_number,
            last_side_to_main_signatures_at_block: side_block_number,
        }
    }
}

impl State {
    /// write state to a `std::io::write`
    pub fn write<W: Write>(&self, mut write: W) -> Result<(), Error> {
        let serialized = toml::to_string(self).expect("serialization can't fail. q.e.d.");
        write.write_all(serialized.as_bytes())?;
        write.flush()?;
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
            state,
        })
    }
}

impl Database for TomlFileDatabase {
    fn read(&self) -> State {
        self.state.clone()
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
