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
extern crate tokio_core;
extern crate tokio_timer;
extern crate toml;
extern crate web3;
extern crate bridge_contracts as contracts;
extern crate tiny_keccak;

#[macro_use]
mod macros;

#[cfg(test)]
#[macro_use]
mod test;

pub mod config;
pub mod bridge;
pub mod database;
pub mod error;
pub mod relay_stream;
pub mod side_contract;
pub mod main_contract;
pub mod deploy;
pub mod main_to_side_sign;
pub mod side_to_main_sign;
pub mod side_to_main_signatures;
pub mod helpers;
pub mod future_heap;

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
