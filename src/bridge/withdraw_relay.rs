use std::sync::Arc;
use futures::{Future, Stream, Poll};
use futures::future::{JoinAll};
use web3::Transport;
use web3::helpers::CallResult;
use web3::types::{H256, Address};
use app::App;
use api::{self, LogStream};
use database::Database;
use error::{Error, ErrorKind};

pub enum WithdrawRelayState<T: Transport> {
	Wait,
	RelayWithdraws {
		future: JoinAll<Vec<CallResult<H256, T::Out>>>,
		block: u64,
	},
	Yield(Option<u64>),
}

pub fn create_withdraw_relay<T: Transport + Clone>(app: Arc<App<T>>, init: &Database) -> WithdrawRelay<T> {
	let logs_init = api::LogStreamInit {
		after: init.checked_withdraw_relay,
		poll_interval: app.config.testnet.poll_interval,
		confirmations: app.config.testnet.required_confirmations,
		filter: app.testnet_bridge().collect_signatures_filter(init.testnet_contract_address.clone()),
	};

	WithdrawRelay {
		logs: api::log_stream(app.connections.testnet.clone(), logs_init),
		mainnet_contract: init.mainnet_contract_address.clone(),
		state: WithdrawRelayState::Wait,
		app,
	}
}

pub struct WithdrawRelay<T: Transport> {
	app: Arc<App<T>>,
	logs: LogStream<T>,
	state: WithdrawRelayState<T>,
	mainnet_contract: Address,
}

impl<T: Transport> Stream for WithdrawRelay<T> {
	type Item = u64;
	type Error = Error;

	fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
		loop {
			let next_state = match self.state {
				WithdrawRelayState::Wait => {
					let _item = try_stream!(self.logs.poll());
					unimplemented!();
				},
				WithdrawRelayState::RelayWithdraws { ref mut future, block } => {
					let _ = try_ready!(future.poll().map_err(ErrorKind::Web3));
					WithdrawRelayState::Yield(Some(block))
				},
				WithdrawRelayState::Yield(ref mut block) => match block.take() {
					None => WithdrawRelayState::Wait,
					some => return Ok(some.into()),
				}
			};
			self.state = next_state;
		}
	}
}
