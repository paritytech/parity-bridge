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
use ethabi::ContractFunction;
use ethereum_types::{Address, U256};
use futures::Future;
use helpers::{AsyncCall, AsyncTransaction};
use log_stream::{LogStream, LogStreamOptions};
use message_to_main::MessageToMain;
use signature::Signature;
use std::time::Duration;
use web3::Transport;

/// a more highlevel wrapper around the auto generated ethabi contract
#[derive(Clone)]
pub struct MainContract<T> {
    pub transport: T,
    pub contract_address: Address,
    pub authority_address: Address,
    pub submit_collected_signatures_gas: U256,
    pub request_timeout: Duration,
    pub logs_poll_interval: Duration,
    pub required_log_confirmations: u32,
}

impl<T: Transport> MainContract<T> {
    pub fn new(transport: T, config: &Config, state: &State) -> Self {
        Self {
            transport,
            contract_address: state.main_contract_address,
            authority_address: config.address,
            submit_collected_signatures_gas: config.estimated_gas_cost_of_withdraw,
            request_timeout: config.home.request_timeout,
            logs_poll_interval: config.home.poll_interval,
            required_log_confirmations: config.home.required_confirmations,
        }
    }

    pub fn call<F: ContractFunction>(&self, f: F) -> AsyncCall<T, F> {
        AsyncCall::new(
            &self.transport,
            self.contract_address,
            self.request_timeout,
            f,
        )
    }

    pub fn is_main_contract(&self) -> AsyncCall<T, contracts::home::IsHomeBridgeContractWithInput> {
        self.call(contracts::home::functions::is_home_bridge_contract())
    }

    /// `Stream` of all txs on main that need to be relayed to side
    pub fn main_to_side_log_stream(&self, after: u64) -> LogStream<T> {
        LogStream::new(LogStreamOptions {
            filter: contracts::home::events::deposit().filter(),
            request_timeout: self.request_timeout,
            poll_interval: self.logs_poll_interval,
            confirmations: self.required_log_confirmations,
            transport: self.transport.clone(),
            contract_address: self.contract_address,
            after,
        })
    }

    /// relay a tx from side to main by submitting message and collected signatures
    pub fn relay_side_to_main(
        &self,
        message: &MessageToMain,
        signatures: &Vec<Signature>,
    ) -> AsyncTransaction<T> {
        AsyncTransaction::new(
            &self.transport,
            self.contract_address,
            self.authority_address,
            self.submit_collected_signatures_gas,
            message.main_gas_price,
            self.request_timeout,
            contracts::home::functions::withdraw(
                signatures.iter().map(|x| x.v),
                signatures.iter().map(|x| x.r),
                signatures.iter().map(|x| x.s),
                message.to_bytes(),
            ),
        )
    }
}
