extern crate serde_json;
extern crate futures;
extern crate jsonrpc_core as rpc;
extern crate web3;
extern crate bridge;

use std::cell::RefCell;
use futures::Future;
use web3::Transport;

pub struct MockedTransport {
	pub requests: RefCell<Vec<(String, String)>>,
	pub mocked_responses: Vec<&'static str>,
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
		let response = self.mocked_responses.iter().nth(self.requests.borrow().len() - 1).expect("out of range");
		futures::finished(serde_json::from_str(response).expect("invalid json")).boxed()
	}
}

#[macro_export]
macro_rules! test_transport_stream {
	(name => $name: ident, init => $init_stream: expr, expected => $expected: expr, $($method: expr => req => $req: expr, res => $res: expr ;)*) => {
		#[test]
		fn $name() {
			use self::futures::{Future, Stream};

			let transport = $crate::MockedTransport {
				requests: Default::default(),
				mocked_responses: vec![$($res),*],
			};
			let stream = $init_stream(&transport);
			let res = stream.collect().wait().unwrap();
			let expected_requests: Vec<_> = vec![$($method),*].into_iter().zip(vec![$($req),*].into_iter()).collect();
			let requests_borrow = transport.requests.borrow();
			let requests: Vec<_> = requests_borrow.iter()
				.map(|&(ref l, ref r)| (l as &str, r as &str))
				.collect();
			assert_eq!(expected_requests, requests);
			assert_eq!($expected, res);
		}
	}
}
