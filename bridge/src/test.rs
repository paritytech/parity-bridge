// Copyright 2017 Parity Technologies (UK) Ltd.
// This file is part of Parity-Bridge.

// Parity-Bridge is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity-Bridge is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity-Bridge.  If not, see <http://www.gnu.org/licenses/>.
use futures;
/// helpers for testing:
///
/// - mocking transports
use jsonrpc_core;
use serde_json;
use std::cell::RefCell;
use std::rc::Rc;
use web3;
use web3::Transport;

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
        assert!(
            current_request_index < self.expected_requests.len(),
            "{} requests expected but at least one more request is being executed",
            self.expected_requests.len()
        );

        assert_eq!(
            self.expected_requests[current_request_index]
                .method
                .as_str(),
            method,
            "invalid method called"
        );
        assert_eq!(
            self.expected_requests[current_request_index].params, params,
            "invalid method params at request #{}",
            current_request_index
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
        let response = self
            .mock_responses
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
