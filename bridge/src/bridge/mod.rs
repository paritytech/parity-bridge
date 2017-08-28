mod deploy;
mod deposit_relay;
mod withdraw_confirm;
mod withdraw_relay;

use std::fs;
use std::sync::Arc;
use std::path::PathBuf;
use futures::{Stream, Poll};
use web3::Transport;
use app::App;
use database::Database;
use error::{Error, Result};

pub use self::deploy::{Deploy, Deployed, create_deploy};
pub use self::deposit_relay::{DepositRelay, create_deposit_relay};
pub use self::withdraw_relay::{WithdrawRelay, create_withdraw_relay};
pub use self::withdraw_confirm::{WithdrawConfirm, create_withdraw_confirm};

/// Last block checked by the bridge components.
#[derive(Clone, Copy)]
pub enum BridgeChecked {
	DepositRelay(u64),
	WithdrawRelay(u64),
	WithdrawConfirm(u64),
}

pub trait BridgeBackend {
	fn save(&mut self, checks: Vec<BridgeChecked>) -> Result<()>;
}

pub struct FileBackend {
	path: PathBuf,
	database: Database,
}

impl BridgeBackend for FileBackend {
	fn save(&mut self, checks: Vec<BridgeChecked>) -> Result<()> {
		for check in checks {
			match check {
				BridgeChecked::DepositRelay(n) => {
					self.database.checked_deposit_relay = n;
				},
				BridgeChecked::WithdrawRelay(n) => {
					self.database.checked_withdraw_relay = n;
				},
				BridgeChecked::WithdrawConfirm(n) => {
					self.database.checked_withdraw_confirm = n;
				},
			}
		}

		let file = fs::OpenOptions::new()
			.write(true)
			.open(&self.path)?;

		self.database.save(file)
	}
}

enum BridgeStatus {
	Wait,
	NextItem(Option<()>),
}

/// Creates new bridge.
pub fn create_bridge<T: Transport + Clone>(app: Arc<App<T>>, init: &Database) -> Bridge<T, FileBackend> {
	let backend = FileBackend {
		path: app.database_path.clone(),
		database: init.clone(),
	};

	create_bridge_backed_by(app, init, backend)
}

/// Creates new bridge writing to custom backend.
pub fn create_bridge_backed_by<T: Transport + Clone, F: BridgeBackend>(app: Arc<App<T>>, init: &Database, backend: F) -> Bridge<T, F> {
	Bridge {
		deposit_relay: create_deposit_relay(app.clone(), init),
		withdraw_relay: create_withdraw_relay(app.clone(), init),
		withdraw_confirm: create_withdraw_confirm(app.clone(), init),
		state: BridgeStatus::Wait,
		backend,
	}
}

pub struct Bridge<T: Transport, F> {
	deposit_relay: DepositRelay<T>,
	withdraw_relay: WithdrawRelay<T>,
	withdraw_confirm: WithdrawConfirm<T>,
	state: BridgeStatus,
	backend: F,
}

impl<T: Transport, F: BridgeBackend> Stream for Bridge<T, F> {
	type Item = ();
	type Error = Error;

	fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
		loop {
			let next_state = match self.state {
				BridgeStatus::Wait => {
					let d_relay = try_bridge!(self.deposit_relay.poll()).map(BridgeChecked::DepositRelay);
					let w_relay = try_bridge!(self.withdraw_relay.poll()).map(BridgeChecked::WithdrawRelay);
					let w_confirm = try_bridge!(self.withdraw_confirm.poll()).map(BridgeChecked::WithdrawConfirm);

					let result = [d_relay, w_relay, w_confirm]
						.into_iter()
						.filter_map(|c| *c)
						.collect();

					self.backend.save(result)?;
					BridgeStatus::NextItem(Some(()))
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
