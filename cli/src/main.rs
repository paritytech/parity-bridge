extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate docopt;
extern crate futures;
extern crate tokio_core;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate bridge;

use std::{env, fs};
use std::sync::Arc;
use std::path::PathBuf;
use docopt::Docopt;
use futures::{Stream, future};
use tokio_core::reactor::Core;

use bridge::app::App;
use bridge::bridge::create_bridge;
use bridge::config::Config;
use bridge::error::Error;
use bridge::database::Database;

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
	let message = err.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n\nCaused by:\n	");
	println!("{}", message);
}

fn execute<S, I>(command: I) -> Result<String, Error> where I: IntoIterator<Item=S>, S: AsRef<str> {
	let usage = format!(
r#"
Parity-bridge
	Copyright 2017 Parity Technologies (UK) Limited
	Version: {}
	Commit: {}

Usage:
	bridge --config <config> --database <database>
	bridge -h | --help

Options:
	-h, --help
"#, env!("CARGO_PKG_VERSION"), env!("GIT_HASH"));

	info!(target: "bridge", "Parsing cli arguments");
	let args: Args = Docopt::new(usage)
		.and_then(|d| d.argv(command).deserialize()).map_err(|e| e.to_string())?;

	info!(target: "bridge", "Loading config");
	let config = Config::load(args.arg_config)?;

	info!(target: "bridge", "Starting event loop");
	let mut event_loop = Core::new().unwrap();

	info!(target: "bridge", "Establishing ipc connection");
	let app = App::new_ipc(config, &args.arg_database, &event_loop.handle())?;
	let app_ref = Arc::new(app.as_ref());

	let database = Database::load(&args.arg_database)?;

	info!(target: "bridge", "Starting listening to events");
	let bridge = create_bridge(app_ref, &database).and_then(|_| future::ok(true)).collect();
	event_loop.run(bridge)?;

	Ok("Done".into())
}
