use contracts;
use error::{self, ResultExt};
use futures::{Async, Future, Poll, Stream};
use helpers::{self, AsyncCall, AsyncTransaction};
use relay_stream::LogToFuture;
use side_contract::SideContract;
use web3::api::Namespace;
use web3::types::{Address, H256, Log, U256};
use web3::Transport;

enum State<T: Transport> {
    AwaitAlreadySigned(AsyncCall<T, contracts::foreign::HasAuthoritySignedMainToSideWithInput>),
    AwaitTxSent(AsyncTransaction<T>),
}

/// `Future` responsible for doing a single relay from `main` to `side`
pub struct MainToSideSign<T: Transport> {
    main_tx_hash: H256,
    recipient: Address,
    value: U256,
    state: State<T>,
    side: SideContract<T>,
}

impl<T: Transport> MainToSideSign<T> {
    pub fn new(raw_log: &Log, side: SideContract<T>) -> Self {
        let main_tx_hash = raw_log
            .transaction_hash
            .expect("`log` must be mined and contain `transaction_hash`. q.e.d.");
        info!(
            "{:?} - step 1/3 - about to check whether already signed",
            main_tx_hash
        );

        let log = helpers::parse_log(&contracts::home::events::deposit(), raw_log)
            .expect("`log` must be for a deposit event. q.e.d.");

        let recipient = log.recipient;
        let value = log.value;

        let future = side.is_main_to_side_signed_on_side(recipient, value, main_tx_hash);
        let state = State::AwaitAlreadySigned(future);

        Self {
            main_tx_hash,
            side,
            state,
            recipient,
            value,
        }
    }
}

impl<T: Transport> Future for MainToSideSign<T> {
    type Item = Option<H256>;
    type Error = error::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            let next_state = match self.state {
                State::AwaitAlreadySigned(ref mut future) => {
                    let has_already_signed = try_ready!(
                        future
                            .poll()
                            .chain_err(|| "MainToSideSign: failed to check if already signed")
                    );
                    if has_already_signed {
                        info!("{:?} - DONE - already signed", self.main_tx_hash);
                        return Ok(Async::Ready(None));
                    }

                    info!("{:?} - 2/3 - signing", self.main_tx_hash);
                    State::AwaitTxSent(self.side.sign_main_to_side(
                        self.recipient,
                        self.value,
                        self.main_tx_hash,
                    ))
                }
                State::AwaitTxSent(ref mut future) => {
                    let main_tx_hash = self.main_tx_hash;
                    let side_tx_hash = try_ready!(future.poll().chain_err(|| format!(
                        "MainToSideSign: checking whether {} already was relayed failed",
                        main_tx_hash
                    )));
                    info!("{:?} - DONE - signed", self.main_tx_hash);
                    return Ok(Async::Ready(Some(side_tx_hash)));
                }
            };
            self.state = next_state;
        }
    }
}

/// options for relays from side to main
#[derive(Clone)]
pub struct LogToMainToSideSign<T> {
    pub side: SideContract<T>,
}

/// from the options and a log a relay future can be made
impl<T: Transport> LogToFuture for LogToMainToSideSign<T> {
    type Future = MainToSideSign<T>;

    fn log_to_future(&self, log: &Log) -> Self::Future {
        MainToSideSign::new(log, self.side.clone())
    }
}

// #[cfg(test)]
// mod tests {
//     use rustc_hex::FromHex;
//     use web3::types::{Bytes, Log};
//     use super::*;
//     use tokio_core::reactor::Core;
//     use contracts;
//     use ethabi;
//     use rustc_hex::ToHex;
//
//     #[test]
//     fn test_deposit_relay_payload() {
//         let data = "000000000000000000000000aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0".from_hex().unwrap();
//         let log = Log {
//             data: data.into(),
//             topics: vec![
//                 "e1fffcc4923d04b559f4d29a8bfc6cda04eb5b0d3c460751c2402c5c5cc9109c".into(),
//             ],
//             transaction_hash: Some(
//                 "884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364".into(),
//             ),
//             ..Default::default()
//         };
//
//         let payload = deposit_relay_payload(log);
//         let expected: Vec<u8> = "26b3293f000000000000000000000000aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364".from_hex().unwrap();
//         assert_eq!(expected, payload);
//     }
//
//     #[test]
//     fn test_deposit_relay_future() {
//         let deposit_topic = HomeBridge::default()
//             .events()
//             .deposit()
//             .create_filter()
//             .topic0;
//
//         let log = contracts::home::logs::Deposit {
//             recipient: "aff3454fce5edbc8cca8697c15331677e6ebcccc".into(),
//             value: 1000.into(),
//         };
//
//         // TODO [snd] would be great if there were a way to automate this
//         let log_data = ethabi::encode(&[
//             ethabi::Token::Address(log.recipient),
//             ethabi::Token::Uint(log.value),
//         ]);
//
//         let log_tx_hash =
//             "0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364".into();
//
//         let raw_log = Log {
//             address: "0000000000000000000000000000000000000001".into(),
//             topics: deposit_topic.into(),
//             data: Bytes(log_data),
//             transaction_hash: Some(log_tx_hash),
//             ..Default::default()
//         };
//
//         let authority_address = "0000000000000000000000000000000000000001".into();
//
//         let tx_hash = "0x1db8f385535c0d178b8f40016048f3a3cffee8f94e68978ea4b277f57b638f0b";
//         let foreign_contract_address = "0000000000000000000000000000000000000dd1".into();
//
//         let tx_data = ForeignBridge::default().functions().deposit().input(
//             log.recipient,
//             log.value,
//             log_tx_hash,
//         );
//
//         let transport = mock_transport!(
//             "eth_sendTransaction" =>
//                 req => json!([{
//                     "data": format!("0x{}", tx_data.to_hex()),
//                     "from": "0x0000000000000000000000000000000000000001",
//                     "gas": "0xfd",
//                     "gasPrice": "0xa0",
//                     "to": foreign_contract_address,
//                 }]),
//             res => json!(tx_hash);
//         );
//
//         let connection = ContractConnection::new(
//             authority_address,
//             foreign_contract_address,
//             transport.clone(),
//             ::std::time::Duration::from_secs(1),
//         );
//
//         let options = Options {
//             foreign: connection,
//             gas: 0xfd.into(),
//             gas_price: 0xa0.into(),
//         };
//
//         let future = MainToSideSign::new(raw_log, options);
//
//         let mut event_loop = Core::new().unwrap();
//         let result = event_loop.run(future).unwrap();
//         assert_eq!(result, tx_hash.into());
//
//         assert_eq!(transport.actual_requests(), transport.expected_requests());
//     }
// }
