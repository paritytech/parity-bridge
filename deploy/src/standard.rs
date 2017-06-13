use std::sync::Arc;
use futures::Future;
use web3::Transport;
use web3::api::Namespace;
use web3::api::eth::Eth;
use web3::types::TransactionRequest;
use {Deploy, DeployFuture, Deployed, Config, Contract};

/// TODO: this struct should be initialized with connection details
pub struct StandardDeploy<T> where T: Transport {
	eth: Arc<Eth<T>>,
}

impl<T> StandardDeploy<T> where T: Transport {
	fn new(eth: Arc<Eth<T>>) -> Self {
		StandardDeploy {
			eth: eth,
		}
	}
}

impl<T> Deploy for StandardDeploy<T> where T: Transport + Send + Sync + 'static, T::Out: Send {
	fn deploy(&self, config: Config) -> DeployFuture<Deployed> {
		let main_tx_request = TransactionRequest {
			// TODO: verifier account
			from: 0.into(), 
			to: None,
			// TODO: make it configurable
			gas: None,
			// TODO: make it configurable
			gas_price: None,
			value: None,
			// TODO: here will be compiled contract code
			data: None,
			nonce: None,
			min_block: None,
		};

		let remote_tx_request = TransactionRequest {
			// TODO: verifier account
			from: 0.into(), 
			to: None,
			// TODO: make it configurable
			gas: None,
			// TODO: make it configurable
			gas_price: None,
			value: None,
			// TODO: here will be compiled contract code
			data: None,
			nonce: None,
			min_block: None,
		};

		let eth = self.eth.clone();
		let main_future = eth.send_transaction(main_tx_request).map(move |hash| eth.transaction_receipt(hash));
		let eth = self.eth.clone();
		let remote_future = eth.send_transaction(remote_tx_request).map(move |hash| eth.transaction_receipt(hash));

		main_future.join(remote_future)
			.map(|(main_receipt, remote_receipt)| {
				Deployed {
					remote: Contract(0.into()),
					main: Contract(0.into()),
				}
			}).boxed()
	}
}


