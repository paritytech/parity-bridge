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

//! the `parity-bridge` executable in `cli/src/main.rs` creates a
//! `Bridge` instance, which is a `Stream`, and then endlessly polls it,
//! thereby running the bridge.
//!
//! the `Bridge` instance internally polls three `RelayStream`s.
//! these correspond to the three relay operations a bridge node is responsible for:
//!
//! 1. `MainToSideSign`: signing off on messages from `main` to `side`. currently that means executing `sideContract.deposit` for every `mainContract.Deposit` event.
//! 2. `SideToMainSign`: signing off on messages from `side` to `main`. currently that means executing `sideContract.submitSignature` for every `sideContract.Withdraw` event.
//! 3. `SideToMainSignatures`: submitting the bundle of signatures collected on `side` through `SideToMainSign` to `main`. currently that means executing `mainContract.withdraw` for every `sideContract.CollectedSignatures` event.
//!
//! a `RelayStream` is logic that's common to the three relay operations.
//! it takes a `Stream` of logs and a `LogToFuture` that maps logs to `Futures`.
//! those futures are supposed to each do one single complete relay
//! (1. `MainToSideSign`, 2. `SideToMainSign`, 3. `SideToMainSignatures`).
//! `RelayStream` polls the log stream, calls `LogToFuture.log_to_future` for each log,
//! and yields the numbers of those blocks for which all such created futures
//! have completed. these block numbers are then persisted
//! so the bridge doesn't have to check logs up to them again next time it's started.
//!
//! a `Bridge` instance is constructed as follows (how the parts fit together):
//!
//! - a tokio `event_loop` is created.
//! - `main_transport` and `side_transport` which are web3 http transports
//!   are created and each use an `event_loop` handle
//! - the initial `state` is read from the database file
//! - the `config` is read from the config file
//! - `main_contract` (`side_contract`) which is for interaction with the main (side) bridge contract
//!   is created from `main_transport` (`side_transport`), `config` and `state`
//! - the `Bridge` instance is created from the two contracts and `state`
//!   - retrieves log streams for the three events to watch
//!   - creates the three `RelayStream`s described above
//!
//! when the `Bridge` instance is polled:
//!
//! - it polls the three `RelayStream`s
//! - each `RelayStream` polls all relay futures that are currently running as well as the log
//! stream
//! - if the log stream yields a log the relay stream creates the corresponding relay future
//! - the relay future is responsible for the entire relay operation
//! - currently relay futures check whether the specific relay has already happened,
//!   ignore if it has and execute the corresponding transaction otherwise
//! - relay futures should (currently don't) and easily could observe whether
//!   the transaction succeeds, log it to help with troubleshooting and
//!   retry if the condition can be recovered from

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

mod bridge;
pub use bridge::Bridge;
pub mod config;
pub mod database;
pub mod deploy;
pub mod error;
mod ordered_stream;
pub use ordered_stream::OrderedStream;
pub mod helpers;
mod main_contract;
pub use main_contract::MainContract;
mod main_to_side_sign;
pub use main_to_side_sign::MainToSideSign;
mod relay_stream;
pub use relay_stream::RelayStream;
mod side_contract;
pub use side_contract::SideContract;
mod side_to_main_sign;
pub use side_to_main_sign::SideToMainSign;
mod side_to_main_signatures;
pub use side_to_main_signatures::SideToMainSignatures;

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
