use web3::Transport;
use web3::types::{Bytes, H256, Log, U256, Address};
use futures::{Async, Future, Poll, Stream};
use web3::helpers::CallResult;
use error::{self, ResultExt};
use message_to_main::MessageToMain;
use futures::future::{join_all, JoinAll};
use helpers::{call, Transaction};
use signature::Signature;
use contracts::foreign::ForeignBridge;
use log_stream::{LogStream, LogStreamOptions};
use std::time::Duration;

/// a more highlevel wrapper around the auto generated ethabi contract
#[derive(Clone)]
pub struct SideContract<T> {
    pub contract_address: Address,
    pub transport: T,
    pub authority_address: Address,
    // TODO [snd] this should get fetched from the contract
    pub required_signatures: u32,
    pub request_timeout: Duration,
    pub poll_interval: Duration,
    pub required_log_confirmations: u32,
}

impl<T: Transport> SideContract<T> {
    /// returns `Future` that resolves with `bool` whether `authority`
    /// has signed main to side relay for `tx_hash`
    pub fn is_main_to_side_signed_on_side(&self, main_tx_hash: H256) -> IsMainToSideSignedOnSide<T> {
        IsMainToSideSignedOnSide::new(self.transport, self.contract_address, self.authority_address, main_tx_hash)
    }

    /// returns `Future` that resolves with `bool` whether `authority`
    /// has signed side to main relay for `tx_hash`
    pub fn is_side_to_main_signed_on_side(&self, side_tx_hash: H256) -> IsSideToMainSignedOnSide<T> {
        IsSideToMainSignedOnSide::new(self.transport, self.authority_address, side_tx_hash);
    }

    pub fn sign_main_to_side(&self, recipient: Address, value: U256, breakout_tx_hash: H256) -> Transaction<T> {
        let payload = ForeignBridge::default()
            .functions()
            .deposit()
            .input(recipient, value, breakout_tx_hash);
        Transaction::new(self.transport, self.contract_address, self.authority_address, payload)
    }

    pub fn side_to_main_sign_log_stream(&self, after: u64) -> LogStream<T> {
        LogStream::new(LogStreamOptions {
            filter: ForeignBridge::default().events().withdraw().create_filter(),
            request_timeout: self.request_timeout,
            poll_interval: self.poll_interval,
            confirmations: self.required_confirmations,
            transport: self.transport,
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
            poll_interval: self.poll_interval,
            confirmations: self.required_log_confirmations,
            transport: self.transport,
            contract_address: self.contract_address,
            after,
        })
    }

    pub fn submit_side_to_main_signature(&self, message: &MessageToMain, signature: &Signature) -> Transaction<T> {
        let payload = ForeignBridge::default()
            .functions()
            .submit_signature()
            .input(signature.to_bytes(), message.to_bytes());
        Transaction::new(self.transport, self.contract_address, self.authority_address, payload)
    }

    pub fn get_message(&self, message_hash: H256) -> GetMessage<T> {
        GetMessage::new(self.transport, self.contract_address, message_hash)
    }

    pub fn get_signatures(&self, message_hash: H256) -> JoinAll<Vec<GetSignature<T>>> {
        let futures = (0..self.required_signatures)
            .into_iter()
            .map(|index| {
                GetSignature::new(self.transport, self.contract_address, message_hash, index)
            })
            .collect::<Vec<_>>();
        join_all(futures)
    }
}

pub struct IsMainToSideSignedOnSide<T: Transport> {
    future: CallResult<Bytes, T::Out>,
    authority: Address,
}

impl<T: Transport> IsMainToSideSignedOnSide<T> {
    pub fn new(transport: T, contract_address: Address, authority: Address, main_tx_hash: H256) -> Self {
        let payload = ForeignBridge::default().functions().deposits().input(main_tx_hash);
        let future = call(contract_address, transport, payload);
        Self { future, authority }
    }
}

impl<T: Transport> Future for IsMainToSideSignedOnSide<T> {
    type Item = bool;
    type Error = error::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let response = try_ready!(self.0.poll());
        let authorities_that_signed_deposit = ForeignBridge::default()
            .functions()
            .deposits()
            .output(response);
        authorities_that_signed_deposit.find(|x| x == self.authority).is_some()
    }
}

pub struct IsSideToMainSignedOnSide<T: Transport> {
    future: CallResult<Bytes, T::Out>,
    authority: Address,
}

impl<T: Transport> IsSideToMainSignedOnSide<T> {
    pub fn new(transport: T, contract_address: Address, authority: Address, main_tx_hash: H256) -> Self {
        let payload = ForeignBridge::default().functions().deposits().input(main_tx_hash);
        let future = call(contract_address, transport, payload);
        Self { future, authority }
    }
}

impl<T: Transport> Future for IsSideToMainSignedOnSide<T> {
    type Item = bool;
    type Error = error::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let response = try_ready!(self.0.poll());
        let authorities_that_signed_deposit = ForeignBridge::default()
            .functions()
            .deposits()
            .output(response);
        authorities_that_signed_deposit.find(|x| x == self.authority).is_some()
    }
}

pub struct GetMessage<T: Transport>(CallResult<Bytes, T::Out>);

impl<T: Transport> GetMessage<T> {
    pub fn new(transport: T, contract_address: Address, message_hash: H256) -> Self {
        let payload = ForeignBridge::default().functions().message().input(message_hash);
        GetMessage(call(transport, contract_address, payload))
    }
}

impl<T: Transport> Future for GetMessage<T> {
    type Item = MessageToMain;
    type Error = error::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let response = try_ready!(self.0.poll());
        let message_bytes = ForeignBridge::default()
            .functions()
            .message()
            .output(response.0.as_slice())
            .chain_err(|| "WithdrawRelay: decoding message failed")?;
        MessageToMain::from_bytes(&message_bytes)
    }
}

pub struct GetSignature<T: Transport>(CallResult<Bytes, T::Out>);

impl<T: Transport> GetSignature<T> {
    pub fn new(transport: &T, contract_address: Address, message_hash: H256, index: u32) -> Self {
        let payload = ForeignBridge::default()
            .functions()
            .signature()
            .input(message_hash, index);
        GetSignature(call(contract_address, transport, payload))
    }
}

impl<T: Transport> Future for GetSignature<T> {
    type Item = Signature;
    type Error = error::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let response = try_ready!(self.0.poll);
        Signature::from_bytes(
            ForeignBridge::default()
                .functions()
                .signature()
                .output(response)
                .chain_err(|| "WithdrawRelay: decoding signature failed")?
                .as_slice(),
        )
    }
}

