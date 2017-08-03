#[macro_use]
extern crate futures;
extern crate futures_cpupool;
extern crate futures_after;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate toml;
extern crate web3;
extern crate docopt;
extern crate tokio_core;
#[macro_use]
extern crate log;
extern crate env_logger;
#[macro_use]
extern crate error_chain;

mod api;
mod app;
mod config;
mod database;
pub mod error;
//mod l;
pub mod actions;

use std::env;
use std::path::PathBuf;
use docopt::Docopt;
use tokio_core::reactor::Core;
use app::App;
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
	arg_config: Option<PathBuf>,
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
	let config = match args.arg_config {
		Some(path) => Config::load(path)?,
		None => Config::default(),
	};

	trace!(target: "bridge", "Starting event loop");
	let mut event_loop = Core::new().unwrap();

	trace!(target: "bridge", "Establishing ipc connection");
	let app = App::new_ipc(config, args.arg_database, &event_loop.handle())?;

	trace!(target: "bridge", "Deploying contracts (if needed)");
	event_loop.run(app.deploy())?;

	Ok("Done".into())
}


#[cfg(test)]
mod tests {
}
