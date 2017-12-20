extern crate serde_json;
extern crate futures;
extern crate jsonrpc_core as rpc;
extern crate web3;
extern crate bridge;
#[macro_use]
extern crate pretty_assertions;

use std::cell::Cell;
use web3::Transport;

pub struct MockedRequest {
	pub method: String,
	pub params: Vec<rpc::Value>,
}

impl From<(&'static str, &'static str)> for MockedRequest {
	fn from(a: (&'static str, &'static str)) -> Self {
		MockedRequest {
			method: a.0.to_owned(),
			params: serde_json::from_str(a.1).unwrap(),
		}
	}
}

pub struct MockedTransport {
	pub requests: Cell<usize>,
	pub expected_requests: Vec<MockedRequest>,
	pub mocked_responses: Vec<&'static str>,
}

impl Transport for MockedTransport {
	type Out = web3::Result<rpc::Value>;

	fn prepare(&self, method: &str, params: Vec<rpc::Value>) -> (usize, rpc::Call) {
		let n = self.requests.get();
		assert_eq!(&self.expected_requests[n].method as &str, method, "invalid method called");
		assert_eq!(self.expected_requests[n].params, params, "invalid method params");
		self.requests.set(n + 1);

		let request = web3::helpers::build_request(1, method, params);
		(n + 1, request)
	}

	fn send(&self, _id: usize, _request: rpc::Call) -> web3::Result<rpc::Value> {
		let response = self.mocked_responses.iter().nth(self.requests.get() - 1).expect("missing response");
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
				expected_requests: vec![$($method),*].into_iter().zip(vec![$($req),*].into_iter()).map(Into::into).collect(),
				mocked_responses: vec![$($res),*],
			};
			let stream = $init_stream(&transport);
			let res = stream.collect().wait();
			assert_eq!($expected, res.unwrap());
		}
	}
}

#[macro_export]
macro_rules! test_app_stream {
	(
		name => $name: ident,
		database => $db: expr,
		home => account => $home_acc: expr, confirmations => $home_conf: expr;
		foreign => account => $foreign_acc: expr, confirmations => $foreign_conf: expr;
		authorities => accounts => $authorities_accs: expr, signatures => $signatures: expr;
		txs => $txs: expr,
		init => $init_stream: expr,
		expected => $expected: expr,
		home_transport => [$($home_method: expr => req => $home_req: expr, res => $home_res: expr ;)*],
		foreign_transport => [$($foreign_method: expr => req => $foreign_req: expr, res => $foreign_res: expr ;)*]
	) => {
		#[test]
		#[allow(unused_imports)]
		fn $name() {
			use self::std::sync::Arc;
			use self::std::time::Duration;
			use self::futures::{Future, Stream};
			use self::bridge::app::{App, Connections};
			use self::bridge::contracts::{foreign, home, validator};
			use self::bridge::config::{
				Config, Authorities, Node, ContractConfig, Transactions, TransactionConfig,
				AuthoritiesSource
			};
			use self::bridge::database::Database;

			let home = $crate::MockedTransport {
				requests: Default::default(),
				expected_requests: vec![$($home_method),*].into_iter().zip(vec![$($home_req),*].into_iter()).map(Into::into).collect(),
				mocked_responses: vec![$($home_res),*],
			};

			let foreign = $crate::MockedTransport {
				requests: Default::default(),
				expected_requests: vec![$($foreign_method),*].into_iter().zip(vec![$($foreign_req),*].into_iter()).map(Into::into).collect(),
				mocked_responses: vec![$($foreign_res),*],
			};

			let config = Config {
				txs: $txs,
				home: Node {
					account: $home_acc.parse().unwrap(),
					ipc: "".into(),
					contract: ContractConfig {
						bin: Default::default(),
					},
					poll_interval: Duration::from_secs(0),
					request_timeout: Duration::from_secs(5),
					required_confirmations: $home_conf,
				},
				foreign: Node {
					account: $foreign_acc.parse().unwrap(),
					ipc: "".into(),
					contract: ContractConfig {
						bin: Default::default(),
					},
					poll_interval: Duration::from_secs(0),
					request_timeout: Duration::from_secs(5),
					required_confirmations: $foreign_conf,
				},
				authorities: Authorities {
					source: AuthoritiesSource::Accounts($authorities_accs.iter().map(|a: &&str| a.parse().unwrap()).collect()),
					required_signatures: $signatures,
				}
			};

			let app = App {
				config,
				database_path: "".into(),
				connections: Connections {
					home: &home,
					foreign: &foreign,
				},
				home_bridge: home::HomeBridge::default(),
				foreign_bridge: foreign::ForeignBridge::default(),
				validators: validator::ValidatorSet::default(),
				timer: Default::default(),
			};

			let app = Arc::new(app);
			let stream = $init_stream(app, &$db);
			let res = stream.collect().wait();
			assert_eq!($expected, res.unwrap());
		}
	}
}

#[cfg(test)]
mod tests {
}
