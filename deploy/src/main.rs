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
use std::path::PathBuf;
use std::{env, fs};
use tokio_core::reactor::Core;
use web3::transports::http::Http;

use bridge::config::Config;
use bridge::database::State;
use bridge::deploy::{DeployMain, DeploySide};
use bridge::error::{self, ResultExt};

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
        .join("\n\nCaused by:\n	");
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
    parity-bridge-deploy --config <config> --database <database>
    parity-bridge-deploy -h | --help

Options:
    -h, --help           Display help message and exit.
"#,
        env!("CARGO_PKG_VERSION"),
        env!("GIT_HASH")
    );

    info!(target: "parity-bridge-deploy", "Parsing cli arguments");
    let args: Args = Docopt::new(usage)
        .and_then(|d| d.argv(command).deserialize())
        .map_err(|e| e.to_string())?;

    info!(target: "parity-bridge-deploy", "Loading config");
    let config = Config::load(args.arg_config)?;

    info!(target: "parity-bridge-deploy", "Starting event loop");
    let mut event_loop = Core::new().unwrap();

    info!(
        "Establishing HTTP connection to main {:?}",
        config.main.http
    );
    let main_transport =
        Http::with_event_loop(
            &config.main.http,
            &event_loop.handle(),
            MAX_PARALLEL_REQUESTS,
        ).chain_err(|| format!("Cannot connect to main at {}", config.main.http))?;

    info!(
        "Establishing HTTP connection to side {:?}",
        config.side.http
    );
    let side_transport =
        Http::with_event_loop(
            &config.side.http,
            &event_loop.handle(),
            MAX_PARALLEL_REQUESTS,
        ).chain_err(|| format!("Cannot connect to side at {}", config.side.http))?;

    info!(target: "parity-bridge-deploy", "Deploying MainBridge contract");
    let main_deployed = event_loop.run(DeployMain::new(config.clone(), main_transport))?;
    info!(target: "parity-bridge-deploy", "Successfully deployed MainBridge contract");

    main_deployed.dump_info(format!(
        "deployment-main-{}",
        main_deployed.contract_address
    ))?;

    info!(target: "parity-bridge-deploy", "Deploying SideBridge contract");
    let side_deployed = event_loop.run(DeploySide::new(config.clone(), side_transport))?;
    info!(target: "parity-bridge-deploy", "Successfully deployed SideBridge contract");

    side_deployed.dump_info(format!(
        "deployment-side-{}",
        side_deployed.contract_address
    ))?;

    let state = State::from_transaction_receipts(&main_deployed.receipt, &side_deployed.receipt);
    info!(target: "parity-bridge-deploy", "\n\n{}\n", state);
    state.write(fs::File::create(args.arg_database)?)?;

    Ok("Done".into())
}
