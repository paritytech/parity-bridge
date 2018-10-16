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
use config::Config;
use contracts;
use database::State;
use ethabi::FunctionOutputDecoder;
use futures::future::{join_all, JoinAll};
use helpers::{AsyncCall, AsyncTransaction};
use log_stream::{LogStream, LogStreamOptions};
use message_to_main::MessageToMain;
use signature::Signature;
use std::time::Duration;
use web3::types::{Address, H256, U256};
use web3::Transport;

/// highlevel wrapper around the auto generated ethabi contract `bridge_contracts::side`
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
    pub sign_side_to_main_gas: U256,
    pub sign_side_to_main_gas_price: U256,
}

impl<T: Transport> SideContract<T> {
    pub fn new(transport: T, config: &Config, state: &State) -> Self {
        Self {
            transport,
            contract_address: state.side_contract_address,
            authority_address: config.address,
            required_signatures: config.authorities.required_signatures,
            request_timeout: config.side.request_timeout,
            logs_poll_interval: config.side.poll_interval,
            required_log_confirmations: config.side.required_confirmations,
            sign_main_to_side_gas: config.txs.deposit_relay.gas,
            sign_main_to_side_gas_price: config.txs.deposit_relay.gas_price,
            sign_side_to_main_gas: config.txs.withdraw_confirm.gas,
            sign_side_to_main_gas_price: config.txs.withdraw_confirm.gas_price,
        }
    }

    pub fn call<F: FunctionOutputDecoder>(&self, payload: Vec<u8>, output_decoder: F) -> AsyncCall<T, F> {
        AsyncCall::new(
            &self.transport,
            self.contract_address,
            self.request_timeout,
            payload,
            output_decoder,
        )
    }

    pub fn is_side_contract(&self) -> AsyncCall<T, contracts::side::functions::is_side_bridge_contract::Decoder> {
        let (payload, decoder) = contracts::side::functions::is_side_bridge_contract::call();
        self.call(payload, decoder)
    }

    /// returns `Future` that resolves with `bool` whether `authority`
    /// has signed side to main relay for `tx_hash`
    pub fn is_side_to_main_signed_on_side(
        &self,
        message: &MessageToMain,
    ) -> AsyncCall<T, contracts::side::functions::has_authority_signed_side_to_main::Decoder> {
        let (payload, decoder) = contracts::side::functions::has_authority_signed_side_to_main::call(
            self.authority_address,
            message.to_bytes(),
        );

        self.call(payload, decoder)
    }

    pub fn is_main_to_side_signed_on_side(
        &self,
        recipient: Address,
        value: U256,
        main_tx_hash: H256,
    ) -> AsyncCall<T, contracts::side::functions::has_authority_signed_main_to_side::Decoder> {
        let (payload, decoder) = contracts::side::functions::has_authority_signed_main_to_side::call(
            self.authority_address,
            recipient,
            value,
            main_tx_hash,
        );

        self.call(payload, decoder)
    }

    pub fn sign_main_to_side(
        &self,
        recipient: Address,
        value: U256,
        breakout_tx_hash: H256,
    ) -> AsyncTransaction<T> {
        let payload = contracts::side::functions::deposit::encode_input(recipient, value, breakout_tx_hash);

        AsyncTransaction::new(
            &self.transport,
            self.contract_address,
            self.authority_address,
            self.sign_main_to_side_gas,
            self.sign_main_to_side_gas_price,
            self.request_timeout,
            payload,
        )
    }

    pub fn side_to_main_sign_log_stream(&self, after: u64) -> LogStream<T> {
        LogStream::new(LogStreamOptions {
            filter: contracts::side::events::withdraw::filter(),
            request_timeout: self.request_timeout,
            poll_interval: self.logs_poll_interval,
            confirmations: self.required_log_confirmations,
            transport: self.transport.clone(),
            contract_address: self.contract_address,
            after,
        })
    }

    pub fn side_to_main_signatures_log_stream(&self, after: u64, address: Address) -> LogStream<T> {
        LogStream::new(LogStreamOptions {
            filter: contracts::side::events::collected_signatures::filter(address),
            request_timeout: self.request_timeout,
            poll_interval: self.logs_poll_interval,
            confirmations: self.required_log_confirmations,
            transport: self.transport.clone(),
            contract_address: self.contract_address,
            after,
        })
    }

    pub fn submit_side_to_main_signature(
        &self,
        message: &MessageToMain,
        signature: &Signature,
    ) -> AsyncTransaction<T> {
        let payload = contracts::side::functions::submit_signature::encode_input(signature.to_bytes(), message.to_bytes());
        AsyncTransaction::new(
            &self.transport,
            self.contract_address,
            self.authority_address,
            self.sign_side_to_main_gas,
            self.sign_side_to_main_gas_price,
            self.request_timeout,
            payload,
        )
    }

    pub fn get_signatures(
        &self,
        message_hash: H256,
    ) -> JoinAll<Vec<AsyncCall<T, contracts::side::functions::signature::Decoder>>> {
        let futures = (0..self.required_signatures)
            .into_iter()
            .map(|index| {
                let (payload, decoder) = contracts::side::functions::signature::call(message_hash, index);
                self.call(payload, decoder)
            })
            .collect::<Vec<_>>();
        join_all(futures)
    }
}
