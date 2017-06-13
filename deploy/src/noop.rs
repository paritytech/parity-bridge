use futures::{Future, finished};
use {Deploy, Config, DeployFuture, Deployed, Contract};

#[derive(Debug, Default)]
pub struct NoopDeploy;

impl NoopDeploy {
	pub fn new() -> Self {
		NoopDeploy::default()
	}
}

impl Deploy for NoopDeploy {
	fn deploy(&self, config: Config) -> DeployFuture<Deployed> {
		let deployed = Deployed {
			remote: Contract(0.into()),
			main: Contract(0.into()),
		};

		finished(deployed).boxed()
	}
}
