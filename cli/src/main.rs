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
use bridge::database::{TomlFileDatabase, Database};

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

    info!("Parsing cli arguments");
    let args: Args = Docopt::new(usage)
        .and_then(|d| d.argv(command).deserialize())
        .map_err(|e| e.to_string())?;

    info!("Loading config from {:?}", args.arg_config);
    let config = Config::load(args.arg_config)?;

    info!("Starting event loop");
    let mut event_loop = Core::new().unwrap();

    info!("Establishing IPC connection to home {:?}", config.home.ipc);
    let home_connection = Ipc::with_event_loop(&config.home.ipc, &event_loop.handle())?;

    info!("Establishing IPC connection to foreign {:?}", config.foreign.ipc);
    let foreign_connection = Ipc::with_event_loop(&config.foreign.ipc, &event_loop.handle())?;

    info!("Loading database from {:?}", args.arg_database);
    let mut database = TomlFileDatabase::from_path(&args.arg_database)?;

    info!("Reading initial state from database");
    let initial_state = database.read();

    let bridge_stream = Bridge::new(config, initial_state, home_connection, foreign_connection);
    info!("Listening to events");
    let persisted_bridge_stream = bridge_stream
        .and_then(|state| {
            database.write(&state)?;
            info!("state change: {}", state);
            Ok(())
        });

    for result in persisted_bridge_stream.wait() {
        let _ = result?;
    }

    Ok("Done".into())
}
