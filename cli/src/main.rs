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

use std::env;
use std::path::PathBuf;
use docopt::Docopt;
use futures::{Stream, future};
use tokio_core::reactor::Core;
use web3::transports::ipc::Ipc;

use bridge::bridge::Bridge;
use bridge::config::Config;
use bridge::error::Error;
use bridge::database::TomlFileDatabase;

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

fn print_err(err: Error) {
    let message = err.iter()
        .map(|e| e.to_string())
        .collect::<Vec<_>>()
        .join("\n\nCaused by:\n  ");
    println!("{}", message);
}

fn execute<S, I>(command: I) -> Result<String, Error>
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

    info!(target: "parity-bridge", "Parsing cli arguments");
    let args: Args = Docopt::new(usage)
        .and_then(|d| d.argv(command).deserialize())
        .map_err(|e| e.to_string())?;

    info!(target: "parity-bridge", "Loading config");
    let config = Config::load(args.arg_config)?;

    info!(target: "parity-bridge", "Starting event loop");
    let mut event_loop = Core::new().unwrap();

    info!(target: "parity-bridge", "Establishing IPC connection to home");
    let home_connection = Ipc::with_event_loop(&config.home.ipc, &event_loop.handle())?;

    info!(target: "parity-bridge", "Establishing IPC connection to foreign");
    let foreign_connection = Ipc::with_event_loop(&config.foreign.ipc, &event_loop.handle())?;

    info!(target: "parity-bridge", "Loading database from file");
    let database = TomlFileDatabase::from_path(&args.arg_database)?;

    let bridge_stream = Bridge::new(config, home_connection, foreign_connection, database);
    info!(target: "parity-bridge", "Listening to events");
    let bridge_future = bridge_stream
        .and_then(|_| future::ok(true))
        .collect();
    event_loop.run(bridge_future)?;

    Ok("Done".into())
}
