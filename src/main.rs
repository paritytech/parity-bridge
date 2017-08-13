#[macro_use]
extern crate futures;
extern crate futures_cpupool;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate toml;
extern crate web3;
extern crate docopt;
extern crate tokio_core;
extern crate tokio_timer;
#[macro_use]
extern crate log;
extern crate env_logger;
#[macro_use]
extern crate error_chain;
extern crate ethabi;

#[macro_use]
mod macros;

pub mod api;
pub mod app;
pub mod config;
pub mod bridge;
pub mod contracts;
pub mod database;
pub mod error;

use std::env;
use std::sync::Arc;
use std::path::PathBuf;
use docopt::Docopt;
use tokio_core::reactor::Core;
use app::App;
use bridge::{create_deploy, Deployed};
use config::Config;
use error::Error;

const USAGE: &'static str = r#"
Ethereum-Kovan bridge.
    Copyright 2017 Parity Technologies (UK) Limited

Usage:
    bridge --config <config> --database <database>
    bridge -h | --help

Options:
    -h, --help           Display help message and exit.
"#;

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
	let message = err.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n\nCaused by:\n  ");
	println!("{}", message);
}

fn execute<S, I>(command: I) -> Result<String, Error> where I: IntoIterator<Item=S>, S: AsRef<str> {
	trace!(target: "bridge", "Parsing cli arguments");
	let args: Args = Docopt::new(USAGE)
		.and_then(|d| d.argv(command).deserialize())?;

	trace!(target: "bridge", "Loading config");
	let config = Config::load(args.arg_config)?;

	trace!(target: "bridge", "Starting event loop");
	let mut event_loop = Core::new().unwrap();

	trace!(target: "bridge", "Establishing ipc connection");
	let app = App::new_ipc(config, &args.arg_database, &event_loop.handle())?;
	let app_ref = Arc::new(app.as_ref());

	trace!(target: "bridge", "Deploying contracts (if needed)");
	let deployed = event_loop.run(create_deploy(app_ref.clone()))?;
		
	match deployed {
		Deployed::New(database) => {
			trace!(target: "bridge", "Deployed new bridge contracts");
			trace!(target: "bridge", "\n\n{}\n", database);
		},
		Deployed::Existing(_database) => {
			trace!(target: "bridge", "Loaded database");
		},
	}

	Ok("Done".into())
}


#[cfg(test)]
mod tests {
}
