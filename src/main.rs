#[macro_use]
extern crate futures;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate toml;
extern crate web3;
extern crate docopt;
extern crate tokio_core;

mod api;
mod app;
mod config;
mod database;
mod error;
//mod l;
pub mod actions;

use std::env;
use std::path::PathBuf;
use docopt::Docopt;
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
	let result = execute(env::args());	
	
	match result {
		Ok(s) => println!("{}", s),
		Err(err) => println!("{}", err),
	}
}

fn execute<S, I>(command: I) -> Result<String, Error> where I: IntoIterator<Item=S>, S: AsRef<str> {
	let args: Args = Docopt::new(USAGE)
		.and_then(|d| d.argv(command).deserialize())?;

	let config = match args.arg_config {
		Some(path) => Config::load(path)?,
		None => Config::default(),
	};

	let app = App::new_ipc(config, args.arg_database)?;

	Ok("Done".into())
}


#[cfg(test)]
mod tests {
}
