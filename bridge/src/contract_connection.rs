/// extracts a pattern that is used commonly throughout the bridge

use futures::future::{FromErr, Future};
use std::time::Duration;
use tokio_timer::{Timeout, Timer};
use web3::{self, Transport};
use web3::api::Namespace;
use web3::types::{Address, Bytes, CallRequest, H256, H520, U256,
                  TransactionRequest};
use web3::helpers::CallResult;
use error;

/// easy interaction with a specific contract with timeout
#[derive(Clone)]
pub struct ContractConnection<T> {
    pub authority_address: Address,
    pub contract_address: Address,
    pub transport: T,
    pub timeout_duration: Duration,
    pub timer: Timer,
}

type CallResult

impl<T: Transport> ContractConnection<T> {
    pub fn new(
        authority_address: Address,
        contract_address: Address,
        transport: T,
        timeout_duration: Duration,
    ) -> Self {
        Self {
            authority_address,
            contract_address,
            timeout_duration,
            transport,
            timer: Timer::default()
        }
    }

    pub fn call(&self, payload: Bytes) -> Timeout<FromErr<CallResult<Bytes, T::Out>, error::Error>> {
        let call_request = CallRequest {
            from: None,
            to: self.contract_address,
            gas: None,
            gas_price: None,
            value: None,
            data: Some(payload),
        };
        let future = web3::api::Eth::new(&self.transport).call(call_request, None);
        self.timer.timeout(future.from_err(), self.timeout_duration)
    }

    pub fn sign(&self, data: Bytes) -> Timeout<FromErr<CallResult<H520, T::Out>, error::Error>> {
        let future = web3::api::Eth::new(&self.transport).sign(self.authority_address, data);
        self.timer.timeout(future.from_err(), self.timeout_duration)
    }

    pub fn send_transaction(&self, payload: Bytes, gas: U256, gas_price: U256) -> Timeout<FromErr<CallResult<H256, T::Out>, error::Error>> {
        let tx_request = TransactionRequest {
            from: self.authority_address,
            to: Some(self.contract_address),
            gas: Some(gas),
            gas_price: Some(gas_price),
            value: None,
            data: Some(payload),
            nonce: None,
            condition: None,
        };
        let future = web3::api::Eth::new(&self.transport).send_transaction(tx_request);
        self.timer.timeout(future.from_err(), self.timeout_duration)
    }
}
