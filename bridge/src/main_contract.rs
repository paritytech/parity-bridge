use helpers::{call, Transaction};
use contracts::home::HomeBridge;
use error;
use ethereum_types::{Address, U256, H256};
use web3::types::Bytes;
use web3::Transport;
use web3::helpers::CallResult;
use futures::{Future, Poll, Async};
use config::Config;
use database::State;
use message_to_main::MessageToMain;
use signature::Signature;
use log_stream::{LogStream, LogStreamOptions};
use std::time::Duration;

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

    // pub fn deploy
    //
    //     let data = HomeBridge::default().constructor(
    //         self.config.home.contract.bin.clone().0,
    //         self.config.authorities.required_signatures,
    //         self.config.authorities.accounts.clone(),
    //         self.config.estimated_gas_cost_of_withdraw,
    //         self.config.max_total_home_contract_balance,
    //         self.config.max_single_deposit_value,
    //     );

    pub fn is_side_to_main_relayed(&self, side_tx_hash: H256) -> IsSideToMainSignaturesRelayed<T> {
        IsSideToMainSignaturesRelayed::new(self.transport, self.contract_address, side_tx_hash)
    }

    /// `Stream` of all txs on main that need to be relayed to side
    pub fn main_to_side_log_stream(&self, after: u64) -> LogStream<T> {
        LogStream::new(LogStreamOptions {
            filter: HomeBridge::default().events().deposit().create_filter(),
            request_timeout: self.request_timeout,
            poll_interval: self.logs_poll_interval,
            confirmations: self.required_log_confirmations,
            transport: self.transport,
            contract_address: self.contract_address,
            after,
        })
    }

    /// relay a tx from side to main by submitting message and collected signatures
    pub fn relay_side_to_main(
        &self,
        message: &MessageToMain,
        signatures: &Vec<Signature>
    ) -> Transaction<T> {
        let payload = HomeBridge::default()
            .functions()
            .withdraw()
            .input(
                signatures.iter().map(|x| x.v),
                signatures.iter().map(|x| x.r),
                signatures.iter().map(|x| x.s),
                message.to_bytes()
            );
        Transaction::new(
            &self.transport,
            self.contract_address,
            self.authority_address,
            self.submit_collected_signatures_gas,
            message.main_gas_price,
            payload)
    }
}

pub struct IsSideToMainSignaturesRelayed<T: Transport> {
    future: CallResult<Bytes, T::Out>,
}

impl<T: Transport> IsSideToMainSignaturesRelayed<T> {
    pub fn new(
        transport: T,
        contract_address: Address,
        side_tx_hash: H256
    ) -> Self {
        let payload = HomeBridge::default().functions().withdraws().input(side_tx_hash);
        let future = call(&transport, contract_address, payload);
        Self { future }
    }
}

impl<T: Transport> Future for IsSideToMainSignaturesRelayed<T> {
    type Item = bool;
    type Error = error::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let response = try_ready!(self.future.poll());
        Ok(Async::Ready(HomeBridge::default().functions().withdraws().output(&response.0)?))
    }
}

