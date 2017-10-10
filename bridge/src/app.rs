use std::path::{Path, PathBuf};
use tokio_core::reactor::{Handle};
use tokio_timer::Timer;
use web3::Transport;
use web3::transports::ipc::Ipc;
use error::{Error, ResultExt, ErrorKind};
use config::Config;
use contracts::{home, foreign};

pub struct App<T> where T: Transport {
	pub config: Config,
	pub database_path: PathBuf,
	pub connections: Connections<T>,
	pub home_bridge: home::HomeBridge,
	pub foreign_bridge: foreign::ForeignBridge,
	pub timer: Timer,
}

pub struct Connections<T> where T: Transport {
	pub home: T,
	pub foreign: T,
}

impl Connections<Ipc> {
	pub fn new_ipc<P: AsRef<Path>>(handle: &Handle, home: P, foreign: P) -> Result<Self, Error> {
		let home = Ipc::with_event_loop(home, handle)
			.map_err(ErrorKind::Web3)
			.map_err(Error::from)
			.chain_err(|| "Cannot connect to home node ipc")?;
		let foreign = Ipc::with_event_loop(foreign, handle)
			.map_err(ErrorKind::Web3)
			.map_err(Error::from)
			.chain_err(|| "Cannot connect to foreign node ipc")?;

		let result = Connections {
			home,
			foreign,
		};
		Ok(result)
	}
}

impl<T: Transport> Connections<T> {
	pub fn as_ref(&self) -> Connections<&T> {
		Connections {
			home: &self.home,
			foreign: &self.foreign,
		}
	}
}

impl App<Ipc> {
	pub fn new_ipc<P: AsRef<Path>>(config: Config, database_path: P, handle: &Handle) -> Result<Self, Error> {
		let connections = Connections::new_ipc(handle, &config.home.ipc, &config.foreign.ipc)?;
		let result = App {
			config,
			database_path: database_path.as_ref().to_path_buf(),
			connections,
			home_bridge: home::HomeBridge::default(),
			foreign_bridge: foreign::ForeignBridge::default(),
			timer: Timer::default(),
		};
		Ok(result)
	}
}

impl<T: Transport> App<T> {
	pub fn as_ref(&self) -> App<&T> {
		App {
			config: self.config.clone(),
			connections: self.connections.as_ref(),
			database_path: self.database_path.clone(),
			home_bridge: home::HomeBridge::default(),
			foreign_bridge: foreign::ForeignBridge::default(),
			timer: self.timer.clone(),
		}
	}
}
