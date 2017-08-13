mod deposit_relay;
mod withdraw_confirm;
mod withdraw_relay;

use futures::{Stream, Poll, Async};
use web3::Transport;
use error::Error;
use self::deposit_relay::DepositRelay;
use self::withdraw_relay::WithdrawRelay;
use self::withdraw_confirm::WithdrawConfirm;

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
