extern crate futures;
extern crate web3;

mod noop;
mod standard;

use futures::future::BoxFuture;
use web3::types::Address;

pub use web3::Error;
pub use noop::NoopDeploy;
pub use standard::StandardDeploy;

#[derive(Debug)]
pub struct Config {
	/// Number of authorities signatures required to confirm an event.
	pub signatures_needed: usize,
	/// Authorities auhtorized to confirm transactions.
	pub authorities: Authorities,
}

pub type DeployFuture<T> = BoxFuture<T, Error>;

#[derive(Debug)]
pub struct Authorities {
	authorities: Vec<Address>,
}

#[derive(Debug)]
pub struct Contract(pub Address);

pub struct Deployed {
	pub remote: Contract,
	pub main: Contract,
}

pub trait Deploy {
	fn deploy(&self, config: Config) -> DeployFuture<Deployed>;
}


