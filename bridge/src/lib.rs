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
#[macro_use]
extern crate error_chain;
extern crate ethabi;
extern crate ethereum_types;
#[macro_use]
extern crate futures;
#[macro_use]
extern crate log;
#[macro_use]
extern crate pretty_assertions;
#[cfg(test)]
#[macro_use]
extern crate quickcheck;
extern crate rustc_hex;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;
extern crate bridge_contracts as contracts;
extern crate tiny_keccak;
extern crate tokio_core;
extern crate tokio_timer;
extern crate toml;
extern crate web3;

#[macro_use]
mod macros;

#[cfg(test)]
#[macro_use]
mod test;

pub mod bridge;
pub mod config;
pub mod database;
pub mod deploy;
pub mod error;
pub mod future_heap;
pub mod helpers;
pub mod main_contract;
pub mod main_to_side_sign;
pub mod relay_stream;
pub mod side_contract;
pub mod side_to_main_sign;
pub mod side_to_main_signatures;

mod log_stream;
pub use log_stream::{LogStream, LogStreamOptions};

mod signature;
pub use signature::Signature;

mod message_to_main;
pub use message_to_main::{MessageToMain, MESSAGE_LENGTH};

#[cfg(test)]
extern crate jsonrpc_core;

#[cfg(test)]
pub use test::MockTransport;
