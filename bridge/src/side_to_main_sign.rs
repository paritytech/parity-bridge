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
use contracts;
use error::{self, ResultExt};
use futures::future::FromErr;
use futures::{Async, Future, Poll};
use helpers::{AsyncCall, AsyncTransaction};
use message_to_main::{MessageToMain, MESSAGE_LENGTH};
use relay_stream::LogToFuture;
use side_contract::SideContract;
use signature::Signature;
use tokio_timer::{Timeout, Timer};
use web3;
use web3::api::Namespace;
use web3::helpers::CallResult;
use web3::types::{Bytes, H256, H520, Log};
use web3::Transport;

enum State<T: Transport> {
    AwaitCheckAlreadySigned(AsyncCall<T, contracts::side::HasAuthoritySignedSideToMainWithInput>),
    AwaitSignature(Timeout<FromErr<CallResult<H520, T::Out>, error::Error>>),
    AwaitTransaction(AsyncTransaction<T>),
}

/// `Future` that is responsible for calling `sideContract.submitSignature`
/// for a single `sideContract.Withdraw` event.
/// these get created by the `side_to_main_sign` `RelayStream` that's part
/// of the `Bridge`.
pub struct SideToMainSign<T: Transport> {
    tx_hash: H256,
    side: SideContract<T>,
    message: MessageToMain,
    state: State<T>,
}

impl<T: Transport> SideToMainSign<T> {
    pub fn new(log: &Log, side: SideContract<T>) -> Self {
        let tx_hash = log.transaction_hash
            .expect("`log` must be mined and contain `transaction_hash`. q.e.d.");

        let message =
            MessageToMain::from_log(log).expect("`log` must contain valid message. q.e.d.");
        let message_bytes = message.to_bytes();

        assert_eq!(
            message_bytes.len(),
            MESSAGE_LENGTH,
            "SideBridge never accepts messages with len != {} bytes; qed",
            MESSAGE_LENGTH
        );

        let future = side.is_side_to_main_signed_on_side(&message);
        let state = State::AwaitCheckAlreadySigned(future);
        info!("{:?} - step 1/3 - about to sign message", tx_hash);

        Self {
            side,
            tx_hash,
            message,
            state,
        }
    }
}

impl<T: Transport> Future for SideToMainSign<T> {
    /// transaction hash
    type Item = Option<H256>;
    type Error = error::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            let next_state = match self.state {
                State::AwaitCheckAlreadySigned(ref mut future) => {
                    let is_already_signed = try_ready!(
                        future
                            .poll()
                            .chain_err(|| "WithdrawConfirm: message signing failed")
                    );
                    if is_already_signed {
                        return Ok(Async::Ready(None));
                    }

                    let inner_future = web3::api::Eth::new(self.side.transport.clone())
                        .sign(self.side.authority_address, Bytes(self.message.to_bytes()))
                        .from_err();
                    let timeout_future =
                        Timer::default().timeout(inner_future, self.side.request_timeout);
                    State::AwaitSignature(timeout_future)
                }
                State::AwaitSignature(ref mut future) => {
                    let signature_bytes = try_ready!(
                        future
                            .poll()
                            .chain_err(|| "WithdrawConfirm: message signing failed")
                    );
                    info!(
                        "{:?} - step 2/3 - message signed. about to send transaction",
                        self.tx_hash
                    );

                    let signature = Signature::from_bytes(&signature_bytes)?;

                    let future = self.side
                        .submit_side_to_main_signature(&self.message, &signature);
                    State::AwaitTransaction(future)
                }
                State::AwaitTransaction(ref mut future) => {
                    let tx_hash = try_ready!(
                        future
                            .poll()
                            .chain_err(|| "WithdrawConfirm: sending transaction failed")
                    );
                    info!(
                        "{:?} - step 3/3 - DONE - transaction sent {:?}",
                        self.tx_hash, tx_hash
                    );
                    return Ok(Async::Ready(Some(tx_hash)));
                }
            };
            self.state = next_state;
        }
    }
}

pub struct LogToSideToMainSign<T: Transport> {
    pub side: SideContract<T>,
}

/// from the options and a log a relay future can be made
impl<T: Transport> LogToFuture for LogToSideToMainSign<T> {
    type Future = SideToMainSign<T>;

    fn log_to_future(&self, log: &Log) -> Self::Future {
        SideToMainSign::new(log, self.side.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use contracts;
    use ethabi;
    use ethabi::ContractFunction;
    use rustc_hex::FromHex;
    use rustc_hex::ToHex;
    use tokio_core::reactor::Core;
    use web3::types::{Address, Bytes, Log};

    #[test]
    fn test_side_to_main_sign_relay_future_not_relayed() {
        let topic = contracts::side::events::withdraw().filter().topic0;

        let log = contracts::side::logs::Withdraw {
            recipient: "aff3454fce5edbc8cca8697c15331677e6ebcccc".into(),
            value: 1000.into(),
            main_gas_price: 100.into(),
        };

        // TODO [snd] would be nice if ethabi derived log structs implemented `encode`
        let log_data = ethabi::encode(&[
            ethabi::Token::Address(log.recipient),
            ethabi::Token::Uint(log.value),
            ethabi::Token::Uint(log.main_gas_price),
        ]);

        let log_tx_hash =
            "0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364".into();

        let raw_log = Log {
            address: "0000000000000000000000000000000000000001".into(),
            topics: topic.into(),
            data: Bytes(log_data),
            transaction_hash: Some(log_tx_hash),
            block_hash: None,
            block_number: None,
            transaction_index: None,
            log_index: None,
            transaction_log_index: None,
            log_type: None,
            removed: None,
        };

        let authority_address: Address = "0000000000000000000000000000000000000001".into();

        let tx_hash = "0x1db8f385535c0d178b8f40016048f3a3cffee8f94e68978ea4b277f57b638f0b";
        let side_contract_address = "0000000000000000000000000000000000000dd1".into();

        let message = MessageToMain {
            recipient: log.recipient,
            value: log.value,
            side_tx_hash: log_tx_hash,
            main_gas_price: log.main_gas_price,
        };

        let call_data = contracts::side::functions::has_authority_signed_side_to_main(
            authority_address,
            message.to_bytes(),
        );

        let signature = "8697c15331677e6ebccccaff3454fce5edbc8cca8697c15331677aff3454fce5edbc8cca8697c15331677e6ebccccaff3454fce5edbc8cca8697c15331677e6ebc";

        let tx_data = contracts::side::functions::submit_signature(
            signature.from_hex().unwrap(),
            message.to_bytes(),
        );

        let transport = mock_transport!(
            "eth_call" =>
                req => json!([{
                    "data": format!("0x{}", call_data.encoded().to_hex()),
                    "to": side_contract_address,
                }, "latest"]),
                res => json!(format!("0x{}", ethabi::encode(&[ethabi::Token::Bool(false)]).to_hex()));
            "eth_sign" =>
                req => json!([
                    authority_address,
                    format!("0x{}", message.to_bytes().to_hex())
                ]),
                res => json!(format!("0x{}", signature));
            "eth_sendTransaction" =>
                req => json!([{
                    "data": format!("0x{}", tx_data.encoded().to_hex()),
                    "from": format!("0x{}", authority_address.to_hex()),
                    "gas": "0xfd",
                    "gasPrice": "0xa0",
                    "to": side_contract_address,
                }]),
                res => json!(tx_hash);
        );

        let side_contract = SideContract {
            transport: transport.clone(),
            contract_address: side_contract_address,
            authority_address,
            required_signatures: 1,
            request_timeout: ::std::time::Duration::from_millis(0),
            logs_poll_interval: ::std::time::Duration::from_millis(0),
            required_log_confirmations: 0,
            sign_main_to_side_gas: 0.into(),
            sign_main_to_side_gas_price: 0.into(),
            sign_side_to_main_gas: 0xfd.into(),
            sign_side_to_main_gas_price: 0xa0.into(),
        };

        let future = SideToMainSign::new(&raw_log, side_contract);

        let mut event_loop = Core::new().unwrap();
        let result = event_loop.run(future).unwrap();
        assert_eq!(result, Some(tx_hash.into()));

        assert_eq!(transport.actual_requests(), transport.expected_requests());
    }

    #[test]
    fn test_side_to_main_sign_relay_future_already_relayed() {
        let topic = contracts::side::events::withdraw().filter().topic0;

        let log = contracts::side::logs::Withdraw {
            recipient: "aff3454fce5edbc8cca8697c15331677e6ebcccc".into(),
            value: 1000.into(),
            main_gas_price: 100.into(),
        };

        // TODO [snd] would be nice if ethabi derived log structs implemented `encode`
        let log_data = ethabi::encode(&[
            ethabi::Token::Address(log.recipient),
            ethabi::Token::Uint(log.value),
            ethabi::Token::Uint(log.main_gas_price),
        ]);

        let log_tx_hash =
            "0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364".into();

        let raw_log = Log {
            address: "0000000000000000000000000000000000000001".into(),
            topics: topic.into(),
            data: Bytes(log_data),
            transaction_hash: Some(log_tx_hash),
            block_hash: None,
            block_number: None,
            transaction_index: None,
            log_index: None,
            transaction_log_index: None,
            log_type: None,
            removed: None,
        };

        let authority_address: Address = "0000000000000000000000000000000000000001".into();

        let tx_hash = "0x1db8f385535c0d178b8f40016048f3a3cffee8f94e68978ea4b277f57b638f0b";
        let side_contract_address = "0000000000000000000000000000000000000dd1".into();

        let message = MessageToMain {
            recipient: log.recipient,
            value: log.value,
            side_tx_hash: log_tx_hash,
            main_gas_price: log.main_gas_price,
        };

        let call_data = contracts::side::functions::has_authority_signed_side_to_main(
            authority_address,
            message.to_bytes(),
        );

        let signature = "8697c15331677e6ebccccaff3454fce5edbc8cca8697c15331677aff3454fce5edbc8cca8697c15331677e6ebccccaff3454fce5edbc8cca8697c15331677e6ebc";

        let tx_data = contracts::side::functions::submit_signature(
            signature.from_hex().unwrap(),
            message.to_bytes(),
        );

        let transport = mock_transport!(
            "eth_call" =>
                req => json!([{
                    "data": format!("0x{}", call_data.encoded().to_hex()),
                    "to": side_contract_address,
                }, "latest"]),
                res => json!(format!("0x{}", ethabi::encode(&[ethabi::Token::Bool(true)]).to_hex()));
        );

        let side_contract = SideContract {
            transport: transport.clone(),
            contract_address: side_contract_address,
            authority_address,
            required_signatures: 1,
            request_timeout: ::std::time::Duration::from_millis(0),
            logs_poll_interval: ::std::time::Duration::from_millis(0),
            required_log_confirmations: 0,
            sign_main_to_side_gas: 0.into(),
            sign_main_to_side_gas_price: 0.into(),
            sign_side_to_main_gas: 0xfd.into(),
            sign_side_to_main_gas_price: 0xa0.into(),
        };

        let future = SideToMainSign::new(&raw_log, side_contract);

        let mut event_loop = Core::new().unwrap();
        let result = event_loop.run(future).unwrap();
        assert_eq!(result, None);

        assert_eq!(transport.actual_requests(), transport.expected_requests());
    }
}
