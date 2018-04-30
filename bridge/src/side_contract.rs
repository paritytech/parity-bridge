use web3::Transport;
use web3::types::{Bytes, H256, Log, U256, Address};
use futures::{Async, Future, Poll, Stream};
use web3::helpers::CallResult;
use error::{self, ResultExt};
use message_to_mainnet::MessageToMainnet;

pub struct HasAuthSignedMainToSide<T: Transport> {
    future: CallResult<Bytes, T::Out>,
    authority: Address,
}

impl<T: Transport> HasAuthSignedMainToSide<T> {
    pub fn new(
        transport: T,
        contract_address: Address,
        authority: Address,
        main_tx_hash: H256
    ) -> Self {
        let payload = ForeignBridge::default().functions().deposits().input(main_tx_hash);
        let future = call(contract_address, transport, payload);
        Self { future, authority }
    }
}

impl<T: Transport> Future for HasAuthSignedMainToSide<T> {
    type Item = bool;
    type Error = error::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let response = try_ready!(future.poll);
        let authorities_that_signed_deposit = ForeignBridge::default()
            .functions()
            .deposits()
            .output(response);
        authorities_that_signed_deposit.find(|x| x == self.authority).is_some()
    }
}

pub struct GetMessage<T: Transport> {
    future: CallResult<Bytes, T::Out>
}

impl<T: Transport> GetMessage<T> {
    pub fn new(message_hash: H256) -> Self {
        let message_payload = ForeignBridge::default()
            .functions()
            .message()
            .input(message_hash);
        let future = call(contract_address, transport, payload);
        Self { future }
    }
}

impl<T: Transport> Future for GetMessage<T> {
    type Item = MessageToMainnet;
    type Error = error::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let response = try_ready!(future.poll);
        let message_bytes = ForeignBridge::default()
            .functions()
            .message()
            .output(response.0.as_slice())
            .chain_err(|| "WithdrawRelay: decoding message failed")?;
        MessageToMainnet::from_bytes(&message_bytes)
    }
}

pub struct GetSignature<T: Transport> {
    future: CallResult<Bytes, T::Out>
}

impl GetSignature<T> {
    pub fn new(transport: &T, contract_address: Address, message_hash: H256, index: u32) {
                let payload = ForeignBridge::default()
                    .functions()
                    .signature()
                    .input(message_hash, index);
                options.side.call(Bytes(payload))
            })
    }
}

impl<T: Transport> Future for GetSignature<T> {
    type Item = Signature;
    type Error = error::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let response = try_ready!(future.poll);
        let message_bytes = ForeignBridge::default()
            .functions()
            .message()
            .output(response.0.as_slice())
            .chain_err(|| "WithdrawRelay: decoding message failed")?;
        MessageToMainnet::from_bytes(&message_bytes)
        Signature::from_bytes(
            ForeignBridge::default()
                .functions()
                .signature()
                .output(signature.0.as_slice())
                .chain_err(|| "WithdrawRelay: decoding signature failed")?
                .as_slice(),
        )
    }
}

/// a more highlevel wrapper around the auto generated ethabi contract
pub struct SideContract<T> {
    pub contract_address: Address,
    pub transport: T,
    pub authority_address: Address,
    // TODO [snd] this should get fetched from the contract
    pub required_signatures: u32
}

impl<T: Transport> SideContract<T> {
    /// returns `Future` that resolves with `bool` whether `authority`
    /// has signed main to side relay for `tx_hash`
    pub fn has_auth_signed_main_to_side(&self, main_tx_hash: H256) -> HasAuthSignedMainToSide<T> {
        HasAuthSignedMainToSide::new(self.transport, self.address, authority_address, main_tx_hash)
    }

    // /// returns `Future` that resolves with `bool` whether `authority`
    // /// has signed side to main relay for `tx_hash`
    // pub fn has_auth_signed_side_to_main(&self, tx_hash: H256) -> IsRelayed<T> {
    //     IsRelayed::new(self.address, self.transport.clone(), self.authority_address, tx_hash)
    // }

    pub fn sign_main_to_side(&self, recipient: Address, value: U256, breakout_tx_hash: H256) -> Transaction<T> {
        let payload = ForeignBridge::default()
            .functions()
            .deposit()
            .input(recipient, value, breakout_tx_hash);
        Transaction::new(self.transport, self.contract_address, self.authority_address, recipient, value, main_tx_hash)
    }

    pub fn sign_side_to_main(&self, recipient: Address, value: U256, breakout_tx_hash: H256) -> Transaction<T> {
        let payload = ForeignBridge::default()
            .functions()
            .submit_signature()
            .input(signature.0.to_vec(), self.message.to_bytes());
        Transaction::new(self.transport, self.contract_address, self.authority_address, 
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
            .collect::Vec<_>();
        join_all(futures)
    }
}
