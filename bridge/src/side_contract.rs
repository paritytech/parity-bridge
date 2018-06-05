use web3::Transport;
use web3::types::{Bytes, H256, Log, U256, Address};
use futures::{Async, Future, Poll, Stream};
use web3::helpers::CallResult;
use error::{self, ResultExt};
use message_to_main::MessageToMain;
use futures::future::{join_all, JoinAll};
use helpers::{AsyncCall, AsyncTransaction};
use signature::Signature;
use contracts::foreign::{self, ForeignBridge};
use contracts;
use log_stream::{LogStream, LogStreamOptions};
use std::time::Duration;
use config::Config;
use database::State;
use ethabi::ContractFunction;

/// a more highlevel wrapper around the auto generated ethabi contract
#[derive(Clone)]
pub struct SideContract<T> {
    pub transport: T,
    pub contract_address: Address,
    pub authority_address: Address,
    // TODO [snd] this should get fetched from the contract
    pub required_signatures: u32,
    pub request_timeout: Duration,
    pub logs_poll_interval: Duration,
    pub required_log_confirmations: u32,
    pub sign_main_to_side_gas: U256,
    pub sign_main_to_side_gas_price: U256,
    pub submit_side_to_main_gas: U256,
}

impl<T: Transport> SideContract<T> {
    pub fn new(transport: T, config: &Config, state: &State) -> Self {
        Self {
            transport,
            contract_address: state.main_contract_address,
            authority_address: config.address,
            required_signatures: config.authorities.required_signatures,
            request_timeout: config.foreign.request_timeout,
            logs_poll_interval: config.foreign.poll_interval,
            required_log_confirmations: config.foreign.required_confirmations,
            sign_main_to_side_gas: config.txs.deposit_relay.gas,
            sign_main_to_side_gas_price: config.txs.deposit_relay.gas_price,
            submit_side_to_main_gas: config.txs.withdraw_relay.gas,
        }
    }

    pub fn call<F: ContractFunction>(&self, f: F) -> AsyncCall<T, F> {
        AsyncCall::new(&self.transport, self.contract_address, self.request_timeout, f)
    }

    /// returns `Future` that resolves with `bool` whether `authority`
    /// has signed side to main relay for `tx_hash`
    pub fn is_side_to_main_signed_on_side(&self, message: &MessageToMain) -> AsyncCall<T, contracts::foreign::HasAuthoritySignedSideToMainWithInput> {
        self.call(ForeignBridge::default()
            .functions()
            .has_authority_signed_side_to_main(self.authority_address, message.keccak256()))
    }

    pub fn is_main_to_side_signed_on_side(&self, recipient: Address, value: U256, main_tx_hash: H256) -> AsyncCall<T, contracts::foreign::HasAuthoritySignedMainToSideWithInput> {
        self.call(ForeignBridge::default()
            .functions()
            .has_authority_signed_main_to_side(self.authority_address, recipient, value, main_tx_hash))
    }

    pub fn sign_main_to_side(&self, recipient: Address, value: U256, breakout_tx_hash: H256) -> AsyncTransaction<T> {
        AsyncTransaction::new(
            &self.transport,
            self.contract_address,
            self.authority_address,
            self.sign_main_to_side_gas,
            self.sign_main_to_side_gas_price,
            self.request_timeout,
            ForeignBridge::default()
                .functions()
                .deposit(recipient, value, breakout_tx_hash))
    }

    pub fn side_to_main_sign_log_stream(&self, after: u64) -> LogStream<T> {
        LogStream::new(LogStreamOptions {
            filter: ForeignBridge::default().events().withdraw().create_filter(),
            request_timeout: self.request_timeout,
            poll_interval: self.logs_poll_interval,
            confirmations: self.required_log_confirmations,
            transport: self.transport.clone(),
            contract_address: self.contract_address,
            after,
        })
    }

    pub fn side_to_main_signatures_log_stream(&self, after: u64) -> LogStream<T> {
        LogStream::new(LogStreamOptions {
            filter: ForeignBridge::default()
                .events()
                .collected_signatures()
                .create_filter(),
            request_timeout: self.request_timeout,
            poll_interval: self.logs_poll_interval,
            confirmations: self.required_log_confirmations,
            transport: self.transport.clone(),
            contract_address: self.contract_address,
            after,
        })
    }

    pub fn submit_side_to_main_signature(&self, message: &MessageToMain, signature: &Signature) -> AsyncTransaction<T> {
        AsyncTransaction::new(
            &self.transport,
            self.contract_address,
            self.authority_address,
            self.submit_side_to_main_gas,
            message.main_gas_price,
            self.request_timeout,
            ForeignBridge::default()
                .functions()
                .submit_signature(signature.to_bytes(), message.to_bytes()))
    }

    pub fn get_signatures(&self, message_hash: H256) -> JoinAll<Vec<AsyncCall<T, foreign::SignatureWithInput>>> {
        let futures = (0..self.required_signatures)
            .into_iter()
            .map(|index| {
                self.call(
                    ForeignBridge::default().functions().signature(message_hash, index)
                )
            })
            .collect::<Vec<_>>();
        join_all(futures)
    }
}
