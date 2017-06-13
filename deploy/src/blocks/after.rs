use futures::{Future, Poll};
use web3::types::U256;
use {Error, DeployFuture};

pub struct AfterBlock {
	block: U256,
	next: DeployFuture<U256>,
}

impl Future for AfterBlock {
	type Item = U256;
	type Error = Error;

	fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
		unimplemented!();
	}
}
