#[macro_use]
extern crate error_chain;
extern crate ethabi;
#[macro_use]
extern crate ethabi_contract;
#[macro_use]
extern crate ethabi_derive;
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

#[macro_use]
mod macros;

#[cfg(test)]
#[macro_use]
mod test;

pub mod contract_connection;
pub mod config;
pub mod bridge;
pub mod contracts;
pub mod database;
pub mod error;
pub mod relay_stream;
pub mod helpers;

mod log_stream;
pub use log_stream::{LogStream, LogStreamOptions};

mod signature;
pub use signature::Signature;

mod message_to_mainnet;
pub use message_to_mainnet::{MessageToMainnet, MESSAGE_LENGTH};

#[cfg(test)]
extern crate jsonrpc_core;

#[cfg(test)]
pub use test::MockTransport;
