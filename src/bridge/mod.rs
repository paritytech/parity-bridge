mod deposit_relay;
mod withdraw_confirm;
mod withdraw_relay;

use std::sync::Arc;
use futures::{Stream, Poll};
use web3::Transport;
use app::App;
use database::Database;
use error::Error;
use self::deposit_relay::{DepositRelay, create_deposit_relay};
use self::withdraw_relay::WithdrawRelay;
use self::withdraw_confirm::{WithdrawConfirm, create_withdraw_confirm};

#[derive(Clone, Copy)]
pub enum BridgeChecked {
	DepositRelay(u64),
	WithdrawRelay(u64),
	WithdrawConfirm(u64),
}

enum BridgeStatus {
	Wait,
	NextItem(Option<Vec<BridgeChecked>>),
}

pub fn create_bridge<T: Transport + Clone>(app: Arc<App<T>>, init: &Database) -> Bridge<T> {
	Bridge {
		deposit_relay: create_deposit_relay(app.clone(), init),
		withdraw_relay: { unimplemented!(); },
		withdraw_confirm: create_withdraw_confirm(app.clone(), init),
		state: BridgeStatus::Wait,
	}
}

pub struct Bridge<T: Transport> {
	deposit_relay: DepositRelay<T>,
	withdraw_relay: WithdrawRelay<T>,
	withdraw_confirm: WithdrawConfirm<T>,
	state: BridgeStatus,
}

impl<T: Transport> Stream for Bridge<T> {
	type Item = Vec<BridgeChecked>;
	type Error = Error;

	fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
		loop {
			let next_state = match self.state {
				BridgeStatus::Wait => {
					let d_relay = try_channel!(self.deposit_relay.poll()).map(BridgeChecked::DepositRelay);
					let w_relay = try_channel!(self.withdraw_relay.poll()).map(BridgeChecked::WithdrawRelay);
					let w_confirm = try_channel!(self.withdraw_confirm.poll()).map(BridgeChecked::WithdrawConfirm);

					let result = [d_relay, w_relay, w_confirm]
						.into_iter()
						.filter_map(|c| *c)
						.collect();
					BridgeStatus::NextItem(Some(result))
				},
				BridgeStatus::NextItem(ref mut v) => match v.take() {
					None => BridgeStatus::Wait,
					some => return Ok(some.into()),
				},
			};
			
			self.state = next_state;
		}
	}
}
