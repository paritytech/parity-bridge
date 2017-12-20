use std::sync::Arc;
use futures::{Future, Poll, Async};
use tokio_timer::Timeout;
use web3::Transport;
use web3::types::{Address, Bytes, H160};

use api::{self, ApiCall};
use error::Result;
use config::AuthoritiesSource;
use contracts::validator;
use app::App;
use error::Error;

#[inline]
fn get_validator_payload(validators: &validator::ValidatorSet) -> Bytes {
	validators.functions().get_validators().input().into()
}

#[inline]
fn get_validators_output(validators: &validator::ValidatorSet, output: &[u8]) -> Result<Vec<Address>> {
	Ok(validators.functions().get_validators().output(output)?.into_iter().map(H160).collect())
}

pub fn fetch_authorities<T: Transport>(app: Arc<App<T>>) -> FetchAuthorities<T> {
	FetchAuthorities {
		app,
		state: FetchAuthoritiesState::Wait,
	}
}

enum FetchAuthoritiesState<T: Transport> {
	Wait,
	Call(Timeout<ApiCall<Bytes, T::Out>>),
}

pub struct FetchAuthorities<T: Transport> {
	app: Arc<App<T>>,
	state: FetchAuthoritiesState<T>,
}

impl<T: Transport> Future for FetchAuthorities<T> {
	type Item = Vec<Address>;
	type Error = Error;

	fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
		let validator_address = match self.app.config.authorities.source {
			AuthoritiesSource::Accounts(ref accounts) => return Ok(Async::Ready(accounts.clone())),
			AuthoritiesSource::ValidatorSet(ref address) => address.clone(),
		};

		loop {
			let next_state = match self.state {
				FetchAuthoritiesState::Wait => {
					let future = self.app.timer.timeout(
						api::call(
							&self.app.connections.foreign,
							validator_address,
							get_validator_payload(&self.app.validators)
						), self.app.config.foreign.request_timeout
					);
					FetchAuthoritiesState::Call(future)
				},
				FetchAuthoritiesState::Call(ref mut future) => {
					let bytes = try_ready!(future.poll());
					let auths = get_validators_output(&self.app.validators, &bytes.0)?;
					return Ok(Async::Ready(auths));
				},
			};

			self.state = next_state;
		}
	}
}
