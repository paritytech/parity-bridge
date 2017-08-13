use futures::{Future, Stream, Poll};
use futures::future::{JoinAll};
use web3::Transport;
use web3::helpers::CallResult;
use web3::types::H256;
use error::Error;

pub enum WithdrawRelayState<T: Transport> {
	Wait,
	RelayWithdraws {
		future: JoinAll<Vec<CallResult<H256, T::Out>>>,
		block: u64,
	},
	Yield(Option<u64>),
}

pub struct WithdrawRelay<T: Transport> {
	state: WithdrawRelayState<T>,
}

impl<T: Transport> Stream for WithdrawRelay<T> {
	type Item = u64;
	type Error = Error;

	fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
		unimplemented!();
	}
}
