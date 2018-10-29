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
use futures::{Async, Future, Poll};
use helpers::{self, AsyncCall, AsyncTransaction};
use relay_stream::LogToFuture;
use side_contract::SideContract;
use main_contract::MainContract;
use web3::types::{Address, H256, Log, U256};
use web3::Transport;

#[derive(Clone)]
pub struct LogToAcceptMessageFromMain<T> {
    pub main: MainContract<T>,
    pub side: SideContract<T>,
}

impl<T: Transport> LogToFuture for LogToAcceptMessageFromMain<T> {
    type Future = AcceptMessageFromMain<T>;

    fn log_to_future(&self, log: &Log) -> Self::Future {
        AcceptMessageFromMain::new(log, self.side.clone(), self.main.clone())
    }
}

enum State<T: Transport> {
    AwaitMessage(AsyncCall<T, contracts::new_main::functions::relayed_messages::Decoder>),
    AwaitAlreadyAccepted {
        message: Vec<u8>,
        future: AsyncCall<T, contracts::new_side::functions::has_authority_accepted_message_from_main::Decoder>
    },
    AwaitTxSent(AsyncTransaction<T>),
}

pub struct AcceptMessageFromMain<T: Transport> {
    state: State<T>,
    main_tx_hash: H256,
    message_id: H256,
    sender: Address,
    recipient: Address,
    side: SideContract<T>,
}

impl<T: Transport> AcceptMessageFromMain<T> {
    pub fn new(raw_log: &Log, side: SideContract<T>, main: MainContract<T>) -> Self {
        let main_tx_hash = raw_log
            .transaction_hash
            .expect("`log` must be mined and contain `transaction_hash`. q.e.d.");


        let log = helpers::parse_log(contracts::new_main::events::relay_message::parse_log, raw_log)
            .expect("`log` must be for a relay message. q.e.d.");

        let sender = log.sender;
        let recipient = log.recipient;

        info!("{:?} - step 1/4 - fetch message using message_id", main_tx_hash);
        let future = main.relayed_message_by_id(log.message_id);
        let state = State::AwaitMessage(future);

        AcceptMessageFromMain {
            state,
            main_tx_hash,
            message_id: log.message_id,
            sender,
            recipient,
            side,
        }
    }
}

impl<T: Transport> Future for AcceptMessageFromMain<T> {
    type Item = Option<H256>;
    type Error = error::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            let next_state = match self.state {
                State::AwaitMessage(ref mut future) => {
                    let message = try_ready!(
                        future
                            .poll()
                            .chain_err(|| "AcceptMessageFromMain: failed to fetch the message")
                    );

                    info!("{:?} - 2/4 - checking if the message is already signed", self.main_tx_hash);
                    State::AwaitAlreadyAccepted {
                        message: message.clone(),
                        future: self.side.is_message_accepted_from_main(
                            self.main_tx_hash,
                            message,
                            self.sender,
                            self.recipient,
                        )
                    }
                },
                State::AwaitAlreadyAccepted { ref message, ref mut future } => {
                    let has_already_accepted = try_ready!(
                        future
                            .poll()
                            .chain_err(|| "AcceptMessageFromMain: failed to check if already accepted")
                    );
                    if has_already_accepted {
                        info!("{:?} - DONE - already accepted", self.main_tx_hash);
                        return Ok(Async::Ready(None));
                    }

                    info!("{:?} - 3/4 - accepting the meessage", self.main_tx_hash);
                    State::AwaitTxSent(self.side.accept_message_from_main(
                        self.main_tx_hash,
                        message.clone(),
                        self.sender,
                        self.recipient,
                    ))
                },
                State::AwaitTxSent(ref mut future) => {
                    let main_tx_hash = self.main_tx_hash;
                    let side_tx_hash = try_ready!(
                        future
                            .poll()
                            .chain_err(|| format!(
                                "AcceptMessageFromMain: checking whether {} already was relayed failed",
                                main_tx_hash
                            ))
                    );
                    info!("{:?} - DONE - accepted", self.main_tx_hash);
                    return Ok(Async::Ready(Some(side_tx_hash)));
                },
            };
            self.state = next_state;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use contracts;
    use ethabi;
    use rustc_hex::ToHex;
    use tokio_core::reactor::Core;
    use web3::types::{Bytes, Log};

    #[test]
    fn test_accept_message_from_main() {
        let topic = contracts::new_main::events::relay_message::filter().topic0;

        let log = contracts::new_main::logs::RelayMessage {
            message_id: "0x1db8f385535c0d178b8f40016048f3a3cffee8f94e68978ea4b277f57b638f0b".into(),
            sender: "aff3454fce5edbc8cca8697c15331677e6ebdddd".into(),
            recipient: "aff3454fce5edbc8cca8697c15331677e6ebcccc".into(),
        };

        let log_data = ethabi::encode(&[
            ethabi::Token::FixedBytes(log.message_id.to_vec()),
            ethabi::Token::Address(log.sender),
            ethabi::Token::Address(log.recipient),
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

        let authority_address = "0000000000000000000000000000000000000001".into();

        let tx_hash = "0x1db8f385535c0d178b8f40016048f3a3cffee8f94e68978ea4b277f57b638f0b";
        let side_contract_address = "0000000000000000000000000000000000000dd1".into();
        let main_contract_address = "0000000000000000000000000000000000000dd2".into();

        let data: Vec<u8> = vec![0x12, 0x34];

        let encoded_message = ethabi::encode(&[ethabi::Token::Bytes(data.clone())]);

        let get_message_call_data = contracts::new_main::functions::messages::encode_input(log.message_id);

        let has_accepted_call_data = contracts::new_side::functions::has_authority_accepted_message_from_main::encode_input(
            log_tx_hash,
            data.clone(),
            log.sender,
            log.recipient,
            authority_address,
        );

        let accept_message_call_data = contracts::new_side::functions::accept_message::encode_input(log_tx_hash, data, log.sender, log.recipient);

        let main_transport = mock_transport!(
            "eth_call" =>
                req => json!([{
                    "data": format!("0x{}", get_message_call_data.to_hex()),
                    "to": main_contract_address,
                }, "latest"]),
                res => json!(format!("0x{}", encoded_message.to_hex()));
        );

        let side_transport = mock_transport!(
            "eth_call" =>
                req => json!([{
                    "data": format!("0x{}", has_accepted_call_data.to_hex()),
                    "to": side_contract_address,
                }, "latest"]),
                res => json!(format!("0x{}", ethabi::encode(&[ethabi::Token::Bool(false)]).to_hex()));
            "eth_sendTransaction" =>
                req => json!([{
                    "data": format!("0x{}", accept_message_call_data.to_hex()),
                    "from": "0x0000000000000000000000000000000000000001",
                    "gas": "0xfd",
                    "gasPrice": "0xa0",
                    "to": side_contract_address,
                }]),
                res => json!(tx_hash);
        );

        let main_contract = MainContract {
            transport: main_transport.clone(),
            contract_address: main_contract_address,
            authority_address,
            submit_collected_signatures_gas: 0.into(),
            request_timeout: ::std::time::Duration::from_millis(0),
            logs_poll_interval: ::std::time::Duration::from_millis(0),
            required_log_confirmations: 0,
        };

        let side_contract = SideContract {
            transport: side_transport.clone(),
            contract_address: side_contract_address,
            authority_address,
            required_signatures: 1,
            request_timeout: ::std::time::Duration::from_millis(0),
            logs_poll_interval: ::std::time::Duration::from_millis(0),
            required_log_confirmations: 0,
            sign_main_to_side_gas: 0xfd.into(),
            sign_main_to_side_gas_price: 0xa0.into(),
            sign_side_to_main_gas: 0.into(),
            sign_side_to_main_gas_price: 0.into(),
        };

        let future = AcceptMessageFromMain::new(&raw_log, side_contract, main_contract);

        let mut event_loop = Core::new().unwrap();
        let result = event_loop.run(future).unwrap();
        assert_eq!(result, Some(tx_hash.into()));

        assert_eq!(side_transport.actual_requests(), side_transport.expected_requests());
        assert_eq!(main_transport.actual_requests(), main_transport.expected_requests());
    }
}

    //#[test]
    //fn test_main_to_side_sign_relay_future_not_relayed() {
        //let topic = contracts::main::events::deposit::filter().topic0;

        //let log = contracts::main::logs::Deposit {
            //recipient: "aff3454fce5edbc8cca8697c15331677e6ebcccc".into(),
            //value: 1000.into(),
        //};

        //let log_data = ethabi::encode(&[
            //ethabi::Token::Address(log.recipient),
            //ethabi::Token::Uint(log.value),
        //]);

        //let log_tx_hash =
            //"0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364".into();

        //let raw_log = Log {
            //address: "0000000000000000000000000000000000000001".into(),
            //topics: topic.into(),
            //data: Bytes(log_data),
            //transaction_hash: Some(log_tx_hash),
            //block_hash: None,
            //block_number: None,
            //transaction_index: None,
            //log_index: None,
            //transaction_log_index: None,
            //log_type: None,
            //removed: None,
        //};

        //let authority_address = "0000000000000000000000000000000000000001".into();

        //let tx_hash = "0x1db8f385535c0d178b8f40016048f3a3cffee8f94e68978ea4b277f57b638f0b";
        //let side_contract_address = "0000000000000000000000000000000000000dd1".into();

        //let call_data = contracts::side::functions::has_authority_signed_main_to_side::encode_input(
            //authority_address,
            //log.recipient,
            //log.value,
            //log_tx_hash,
        //);

        //let tx_data = contracts::side::functions::deposit::encode_input(log.recipient, log.value, log_tx_hash);

        //let transport = mock_transport!(
            //"eth_call" =>
                //req => json!([{
                    //"data": format!("0x{}", call_data.to_hex()),
                    //"to": side_contract_address,
                //}, "latest"]),
                //res => json!(format!("0x{}", ethabi::encode(&[ethabi::Token::Bool(false)]).to_hex()));
            //"eth_sendTransaction" =>
                //req => json!([{
                    //"data": format!("0x{}", tx_data.to_hex()),
                    //"from": "0x0000000000000000000000000000000000000001",
                    //"gas": "0xfd",
                    //"gasPrice": "0xa0",
                    //"to": side_contract_address,
                //}]),
                //res => json!(tx_hash);
        //);

        //let side_contract = SideContract {
            //transport: transport.clone(),
            //contract_address: side_contract_address,
            //authority_address,
            //required_signatures: 1,
            //request_timeout: ::std::time::Duration::from_millis(0),
            //logs_poll_interval: ::std::time::Duration::from_millis(0),
            //required_log_confirmations: 0,
            //sign_main_to_side_gas: 0xfd.into(),
            //sign_main_to_side_gas_price: 0xa0.into(),
            //sign_side_to_main_gas: 0.into(),
            //sign_side_to_main_gas_price: 0.into(),
        //};

        //let future = MainToSideSign::new(&raw_log, side_contract);

        //let mut event_loop = Core::new().unwrap();
        //let result = event_loop.run(future).unwrap();
        //assert_eq!(result, Some(tx_hash.into()));

        //assert_eq!(transport.actual_requests(), transport.expected_requests());
    //}

    //#[test]
    //fn test_main_to_side_sign_relay_future_already_relayed() {
        //let topic = contracts::main::events::deposit::filter().topic0;

        //let log = contracts::main::logs::Deposit {
            //recipient: "aff3454fce5edbc8cca8697c15331677e6ebcccc".into(),
            //value: 1000.into(),
        //};

        //let log_data = ethabi::encode(&[
            //ethabi::Token::Address(log.recipient),
            //ethabi::Token::Uint(log.value),
        //]);

        //let log_tx_hash =
            //"0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364".into();

        //let raw_log = Log {
            //address: "0000000000000000000000000000000000000001".into(),
            //topics: topic.into(),
            //data: Bytes(log_data),
            //transaction_hash: Some(log_tx_hash),
            //block_hash: None,
            //block_number: None,
            //transaction_index: None,
            //log_index: None,
            //transaction_log_index: None,
            //log_type: None,
            //removed: None,
        //};

        //let authority_address = "0000000000000000000000000000000000000001".into();

        //let side_contract_address = "0000000000000000000000000000000000000dd1".into();

        //let call_data = contracts::side::functions::has_authority_signed_main_to_side::encode_input(
            //authority_address,
            //log.recipient,
            //log.value,
            //log_tx_hash,
        //);

        //let transport = mock_transport!(
            //"eth_call" =>
                //req => json!([{
                    //"data": format!("0x{}", call_data.to_hex()),
                    //"to": side_contract_address,
                //}, "latest"]),
                //res => json!(format!("0x{}", ethabi::encode(&[ethabi::Token::Bool(true)]).to_hex()));
        //);

        //let side_contract = SideContract {
            //transport: transport.clone(),
            //contract_address: side_contract_address,
            //authority_address,
            //required_signatures: 1,
            //request_timeout: ::std::time::Duration::from_millis(0),
            //logs_poll_interval: ::std::time::Duration::from_millis(0),
            //required_log_confirmations: 0,
            //sign_main_to_side_gas: 0xfd.into(),
            //sign_main_to_side_gas_price: 0xa0.into(),
            //sign_side_to_main_gas: 0.into(),
            //sign_side_to_main_gas_price: 0.into(),
        //};

        //let future = MainToSideSign::new(&raw_log, side_contract);

        //let mut event_loop = Core::new().unwrap();
        //let result = event_loop.run(future).unwrap();
        //assert_eq!(result, None);

        //assert_eq!(transport.actual_requests(), transport.expected_requests());
    //}
//}
