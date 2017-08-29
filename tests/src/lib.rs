extern crate serde_json;
extern crate futures;
extern crate jsonrpc_core as rpc;
extern crate web3;
extern crate bridge;
#[macro_use]
extern crate pretty_assertions;

use std::cell::RefCell;
use web3::Transport;

pub struct MockedTransport {
	pub requests: RefCell<Vec<(String, String)>>,
	pub mocked_responses: Vec<&'static str>,
}

impl MockedTransport {
	pub fn compare_requests(&self, expected: &[(&str, &str)]) {
		let requests_borrow = self.requests.borrow();
		let requests: Vec<_> = requests_borrow.iter()
			.map(|&(ref l, ref r)| (l as &str, r as &str))
			.collect();
		assert_eq!(expected, &requests as &[_]);
	}
}

impl Transport for MockedTransport {
	type Out = web3::Result<rpc::Value>;

	fn prepare(&self, method: &str, params: Vec<rpc::Value>) -> (usize, rpc::Call) {
		let params_string = serde_json::to_string(&params).unwrap();
		println!("method: {}\nparams:\n{:#?}", method, params_string);
		self.requests.borrow_mut().push((method.into(), params_string));
		let request = web3::helpers::build_request(1, method, params);
		(self.requests.borrow().len(), request)
	}

	fn send(&self, _id: usize, _request: rpc::Call) -> web3::Result<rpc::Value> {
		let response = self.mocked_responses.iter().nth(self.requests.borrow().len() - 1).expect("missing response");
		let f = futures::finished(serde_json::from_str(response).expect("invalid response"));
		Box::new(f)
	}
}

#[macro_export]
macro_rules! test_transport_stream {
	(
		name => $name: ident,
		init => $init_stream: expr,
		expected => $expected: expr,
		$($method: expr => req => $req: expr, res => $res: expr ;)*
	) => {
		#[test]
		fn $name() {
			use self::futures::{Future, Stream};

			let transport = $crate::MockedTransport {
				requests: Default::default(),
				mocked_responses: vec![$($res),*],
			};
			let stream = $init_stream(&transport);
			let res = stream.collect().wait();
			let expected_requests: Vec<_> = vec![$($method),*].into_iter().zip(vec![$($req),*].into_iter()).collect();
			transport.compare_requests(&expected_requests);
			assert_eq!($expected, res.unwrap());
		}
	}
}

#[macro_export]
macro_rules! test_app_stream {
	(
		name => $name: ident,
		database => $db: expr,
		mainnet => account => $mainnet_acc: expr, confirmations => $mainnet_conf: expr;
		testnet => account => $testnet_acc: expr, confirmations => $testnet_conf: expr;
		authorities => accounts => $authorities_accs: expr, signatures => $signatures: expr;
		txs => $txs: expr,
		init => $init_stream: expr,
		expected => $expected: expr,
		mainnet_transport => [$($mainnet_method: expr => req => $mainnet_req: expr, res => $mainnet_res: expr ;)*],
		testnet_transport => [$($testnet_method: expr => req => $testnet_req: expr, res => $testnet_res: expr ;)*]
	) => {
		#[test]
		#[allow(unused_imports)]
		fn $name() {
			use self::std::sync::Arc;
			use self::std::time::Duration;
			use self::futures::{Future, Stream};
			use self::bridge::app::{App, Connections};
			use self::bridge::contracts::{testnet, mainnet};
			use self::bridge::config::{Config, Authorities, Node, ContractConfig, Transactions, TransactionConfig};
			use self::bridge::database::Database;

			let mainnet = $crate::MockedTransport {
				requests: Default::default(),
				mocked_responses: vec![$($mainnet_res),*],
			};

			let testnet = $crate::MockedTransport {
				requests: Default::default(),
				mocked_responses: vec![$($testnet_res),*],
			};

			let config = Config {
				txs: $txs,
				mainnet: Node {
					account: $mainnet_acc.parse().unwrap(),
					ipc: "".into(),
					contract: ContractConfig {
						bin: Default::default(),
					},
					poll_interval: Duration::from_secs(0),
					required_confirmations: $mainnet_conf,
				},
				testnet: Node {
					account: $testnet_acc.parse().unwrap(),
					ipc: "".into(),
					contract: ContractConfig {
						bin: Default::default(),
					},
					poll_interval: Duration::from_secs(0),
					required_confirmations: $testnet_conf,
				},
				authorities: Authorities {
					accounts: $authorities_accs.iter().map(|a: &&str| a.parse().unwrap()).collect(),
					required_signatures: $signatures,
				}
			};

			let app = App {
				config,
				database_path: "".into(),
				connections: Connections {
					mainnet: &mainnet,
					testnet: &testnet,
				},
				mainnet_bridge: mainnet::EthereumBridge::default(),
				testnet_bridge: testnet::KovanBridge::default(),
			};

			let app = Arc::new(app);
			let stream = $init_stream(app, &$db);
			let res = stream.collect().wait();

			let expected_mainnet_requests: Vec<_> = vec![$($mainnet_method),*].into_iter().zip(vec![$($mainnet_req),*].into_iter()).collect();
			mainnet.compare_requests(&expected_mainnet_requests);
			let expected_testnet_requests: Vec<_> = vec![$($testnet_method),*].into_iter().zip(vec![$($testnet_req),*].into_iter()).collect();
			testnet.compare_requests(&expected_testnet_requests);

			assert_eq!($expected, res.unwrap());
		}
	}
}

#[cfg(test)]
mod tests {
}
