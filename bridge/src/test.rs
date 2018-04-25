/// helpers for testing:
///
/// - mocking transports

use jsonrpc_core;
use web3;
use serde_json;
use std::cell::RefCell;
use std::rc::Rc;
use web3::Transport;
use futures;

#[derive(Debug, Clone, PartialEq)]
pub struct RequestData {
    pub method: String,
    pub params: Vec<jsonrpc_core::Value>,
}

impl From<(&'static str, serde_json::Value)> for RequestData {
    fn from(a: (&'static str, serde_json::Value)) -> Self {
        Self {
            method: a.0.to_owned(),
            params: a.1.as_array().unwrap().clone(),
        }
    }
}

/// a `Transport` that and will return the specified responses
/// `clone`d versions have the same storage
#[derive(Debug, Clone)]
pub struct MockTransport {
    pub expected_requests: Vec<RequestData>,
    pub actual_requests: Rc<RefCell<Vec<RequestData>>>,
    pub mock_responses: Vec<serde_json::Value>,
}

impl MockTransport {
    pub fn expected_requests(&self) -> Vec<RequestData> {
        self.expected_requests.clone()
    }
    pub fn actual_requests(&self) -> Vec<RequestData> {
        self.actual_requests.as_ref().borrow().clone()
    }
}

impl Transport for MockTransport {
    type Out = web3::Result<jsonrpc_core::Value>;

    fn prepare(
        &self,
        method: &str,
        params: Vec<jsonrpc_core::Value>,
    ) -> (usize, jsonrpc_core::Call) {
        let current_request_index = { self.actual_requests.as_ref().borrow().len() };
        assert_eq!(
            self.expected_requests[current_request_index]
                .method
                .as_str(),
            method,
            "invalid method called"
        );
        assert_eq!(
            self.expected_requests[current_request_index].params, params,
            "invalid method params"
        );
        self.actual_requests
            .as_ref()
            .borrow_mut()
            .push(RequestData {
                method: method.to_string(),
                params: params.clone(),
            });

        let request = web3::helpers::build_request(1, method, params);
        (current_request_index + 1, request)
    }

    fn send(&self, _id: usize, _request: jsonrpc_core::Call) -> web3::Result<jsonrpc_core::Value> {
        let current_request_index = { self.actual_requests.as_ref().borrow().len() };
        let response = self.mock_responses
            .iter()
            .nth(current_request_index - 1)
            .expect("missing response");
        let f = futures::finished(response.clone());
        Box::new(f)
    }
}

#[macro_export]
macro_rules! mock_transport {
    (
        $($method: expr => req => $req: expr, res => $res: expr ;)*
    ) => {
        $crate::MockTransport {
            actual_requests: Default::default(),
            expected_requests: vec![$($method),*]
                .into_iter()
                .zip(vec![$($req),*]
                .into_iter())
                .map(Into::into)
                .collect(),
            mock_responses: vec![$($res),*],
        }
    }
}
