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
extern crate bridge;
extern crate docopt;
extern crate env_logger;
extern crate futures;
#[macro_use]
extern crate log;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate tokio_core;
extern crate web3;

use docopt::Docopt;
use futures::Stream;
use std::env;
use std::path::PathBuf;
use tokio_core::reactor::Core;
use web3::transports::http::Http;

use bridge::config::Config;
use bridge::database::{Database, TomlFileDatabase};
use bridge::error::{self, ResultExt};
use bridge::helpers::StreamExt;

const MAX_PARALLEL_REQUESTS: usize = 10;

#[derive(Debug, Deserialize)]
pub struct Args {
    arg_config: PathBuf,
    arg_database: PathBuf,
}

fn main() {
    let _ = env_logger::init();
    let result = execute(env::args());

    match result {
        Ok(s) => println!("{}", s),
        Err(err) => print_err(err),
    }
}

fn print_err(err: error::Error) {
    let message = err.iter()
        .map(|e| e.to_string())
        .collect::<Vec<_>>()
        .join("\n\nCaused by:\n  ");
    println!("{}", message);
}

fn execute<S, I>(command: I) -> Result<String, error::Error>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let usage = format!(
        r#"
Parity-bridge
    Copyright 2017 Parity Technologies (UK) Limited
    Version: {}
    Commit: {}

Usage:
    parity-bridge --config <config> --database <database>
    parity-bridge -h | --help

Options:
    -h, --help           Display help message and exit.
"#,
        env!("CARGO_PKG_VERSION"),
        env!("GIT_HASH")
    );

    info!("Parsing cli arguments");
    let args: Args = Docopt::new(usage)
        .and_then(|d| d.argv(command).deserialize())
        .map_err(|e| e.to_string())?;

    info!("Loading config from {:?}", args.arg_config);
    let config = Config::load(&args.arg_config)?;

    info!("Starting event loop");
    let mut event_loop = Core::new().unwrap();

    info!(
        "Establishing HTTP connection to parity node connected to main chain at {:?}",
        config.main.http
    );
    let main_transport = Http::with_event_loop(
        &config.main.http,
        &event_loop.handle(),
        MAX_PARALLEL_REQUESTS,
    ).chain_err(|| {
        format!(
            "Cannot connect to parity node connected to main chain at {}",
            config.main.http
        )
    })?;

    info!(
        "Establishing HTTP connection to parity node connected to side chain at {:?}",
        config.side.http
    );
    let side_transport = Http::with_event_loop(
        &config.side.http,
        &event_loop.handle(),
        MAX_PARALLEL_REQUESTS,
    ).chain_err(|| {
        format!(
            "Cannot connect to parity node connected to side chain at {}",
            config.side.http
        )
    })?;

    info!("Loading database from {:?}", args.arg_database);
    let mut database = TomlFileDatabase::from_path(&args.arg_database)?;

    info!("Reading initial state from database");
    let initial_state = database.read();

    let main_contract = bridge::MainContract::new(main_transport.clone(), &config, &initial_state);
    event_loop
        .run(main_contract.is_main_contract())
        .chain_err(|| {
            format!(
            "call to main contract `is_main_bridge_contract` failed. this is likely due to field `main_contract_address = {}` in database file {:?} not pointing to a bridge main contract. please verify!",
            initial_state.main_contract_address,
            args.arg_database
        )
        })?;

    let side_contract = bridge::SideContract::new(side_transport.clone(), &config, &initial_state);
    event_loop
        .run(side_contract.is_side_contract())
        .chain_err(|| {
            format!(
            "call to side contract `is_side_bridge_contract` failed. this is likely due to field `side_contract_address = {}` in database file {:?} not pointing to a bridge side contract. please verify!",
            initial_state.side_contract_address,
            args.arg_database
        )
        })?;

    let bridge_stream = bridge::Bridge::new(initial_state, main_contract, side_contract);
    info!("Started polling logs");
    let persisted_bridge_stream = bridge_stream.and_then(|state| {
        database.write(&state)?;
        // info!("state change: {}", state);
        Ok(())
    });

    event_loop.run(persisted_bridge_stream.last())?;

    Ok("Done".into())
}
