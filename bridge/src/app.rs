use std::path::{Path, PathBuf};
use tokio_core::reactor::{Handle};
use tokio_timer::Timer;
use web3::Transport;
use web3::transports::ipc::Ipc;
use error::{Error, ResultExt, ErrorKind};
use config::Config;
use contracts::{mainnet, testnet};

pub struct App<T> where T: Transport {
	pub config: Config,
	pub database_path: PathBuf,
	pub connections: Connections<T>,
	pub mainnet_bridge: mainnet::EthereumBridge,
	pub testnet_bridge: testnet::KovanBridge,
	pub timer: Timer,
}

pub struct Connections<T> where T: Transport {
	pub mainnet: T,
	pub testnet: T,
}

impl Connections<Ipc> {
	pub fn new_ipc<P: AsRef<Path>>(handle: &Handle, mainnet: P, testnet: P) -> Result<Self, Error> {
		let mainnet = Ipc::with_event_loop(mainnet, handle)
			.map_err(ErrorKind::Web3)
			.map_err(Error::from)
			.chain_err(|| "Cannot connect to mainnet node ipc")?;
		let testnet = Ipc::with_event_loop(testnet, handle)
			.map_err(ErrorKind::Web3)
			.map_err(Error::from)
			.chain_err(|| "Cannot connect to testnet node ipc")?;

		let result = Connections {
			mainnet,
			testnet,
		};
		Ok(result)
	}
}

impl<T: Transport> Connections<T> {
	pub fn as_ref(&self) -> Connections<&T> {
		Connections {
			mainnet: &self.mainnet,
			testnet: &self.testnet,
		}
	}
}

impl App<Ipc> {
	pub fn new_ipc<P: AsRef<Path>>(config: Config, database_path: P, handle: &Handle) -> Result<Self, Error> {
		let connections = Connections::new_ipc(handle, &config.mainnet.ipc, &config.testnet.ipc)?;
		let result = App {
			config,
			database_path: database_path.as_ref().to_path_buf(),
			connections,
			mainnet_bridge: mainnet::EthereumBridge::default(),
			testnet_bridge: testnet::KovanBridge::default(),
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
			mainnet_bridge: mainnet::EthereumBridge::default(),
			testnet_bridge: testnet::KovanBridge::default(),
			timer: self.timer.clone(),
		}
	}
}
