use futures::{Async, Future, Poll, Stream};
use futures::future::{join_all, FromErr, Join, JoinAll};
use tokio_timer::Timeout;
use web3::Transport;
use web3::types::{Address, Bytes, H256, Log, U256, TransactionReceipt};
use web3;
use ethabi::{self, RawLog};
use log_stream::LogStream;
use contracts::foreign;
use contracts::home::HomeBridge;
use error::{self, ResultExt};
use message_to_main::MessageToMain;
use signature::Signature;
use contract_connection::ContractConnection;
use web3::helpers::CallResult;
use helpers::web3_to_ethabi_log;
use relay_stream::LogToFuture;
use side_contract::{SideContract, GetMessage, GetSignature};
use main_contract::{MainContract, IsSideToMainSignaturesRelayed};

fn log_to_collected_signatures(web3_log: &web3::types::Log) -> foreign::logs::CollectedSignatures {
    foreign::ForeignBridge::default()
        .events()
        .collected_signatures()
        .parse_log(web3_to_ethabi_log(web3_log))
        .expect("`Log` must be a from a `CollectedSignatures` event. q.e.d.")
}

/// state of the state machine that is the future responsible for
/// the SideToMain relay
enum State<T: Transport> {
    AwaitMessage(Timeout<GetMessage<T>>),
    /// authority is not responsible for relaying this. noop
    NotResponsible,
    AwaitIsRelayed {
        future: Timeout<IsSideToMainSignaturesRelayed<T>>,
        message: MessageToMain,
    },
    AwaitSignatures {
        future: Timeout<JoinAll<Vec<GetSignature<T>>>>,
        message: MessageToMain,
    },
    AwaitTxSent(Timeout<FromErr<CallResult<H256, T::Out>, error::Error>>),
    AwaitTxReceipt(Timeout<FromErr<CallResult<Option<TransactionReceipt>, T::Out>, error::Error>>),
}

pub struct SideToMainSignatures<T: Transport> {
    side_tx_hash: H256,
    main: MainContract<T>,
    side: SideContract<T>,
    state: State<T>,
}

impl<T: Transport> SideToMainSignatures<T> {
    pub fn new(log: Log, main: MainContract<T>, side: SideContract<T>) -> Self {
        let side_tx_hash = log.transaction_hash
            .expect("`log` must be mined and contain `transaction_hash`. q.e.d.");

        let parsed_log = log_to_collected_signatures(&log);

        let state = if parsed_log.authority_responsible_for_relay != main.authority_address {
            info!(
                "{:?} - this bridge node is not responsible for relaying transaction to main",
                side_tx_hash
            );
            // this bridge node is not responsible for relaying this transaction.
            // someone else will relay this transaction to home.
            State::NotResponsible
        } else {
            info!(
                "{:?} - step 1/3 - about to fetch message",
                side_tx_hash,
            );
            State::AwaitMessage(side.get_message(parsed_log.message_hash))
        };

        Self {
            side_tx_hash,
            main,
            side,
            state,
        }
    }
}

impl<T: Transport> Future for SideToMainSignatures<T> {
    type Item = Option<TransactionReceipt>;
    type Error = error::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            let next_state = match self.state {
                State::NotResponsible => {
                    return Ok(Async::Ready(None));
                }
                State::AwaitMessage(ref mut future) => {
                    let message = try_ready!(future.poll().chain_err(|| "SubmitSignature: fetching message failed"));
                    State::AwaitIsRelayed {
                        future: self.main.is_side_tx_relayed(message.side_tx_hash),
                        message
                    }
                }
                State::AwaitIsRelayed { ref mut future, message } => {
                    let is_relayed = try_ready!(future.poll().chain_err(|| "SubmitSignature: fetching message failed"));

                    if is_relayed {
                        return None;
                    }

                    State::AwaitSignatures {
                        future: self.side_contract.get_signatures(message.hash()),
                        message
                    }
                }
                State::AwaitSignatures { ref mut future, message }  => {
                    let signatures = try_ready!(future.poll().chain_err(|| "WithdrawRelay: fetching message and signatures failed"));
                    info!("{:?} - step 2/3 - message and {} signatures received. about to send transaction", self.tx_hash, signatures.len());
                    State::AwaitTxSent(self.main.relay_side_tx(message, signatures))
                }
                State::AwaitTxSent(ref mut future) => {
                    let main_tx_hash = try_ready!(
                        future
                            .poll()
                            .chain_err(|| "WithdrawRelay: sending transaction failed")
                    );
                    info!(
                        "{:?} - step 3/3 - DONE - transaction sent {:?}",
                        self.tx_hash, main_tx_hash
                    );
                    State::AwaitTxReceipt(web3::api::Eth::new(self.side.transport)
                        .transaction_receipt(main_tx_hash))
                }
                State::AwaitTxReceipt(ref mut future) => {
                    let receipt = try_ready!(future.poll().chain_err(|| "WithdrawRelay: sending transaction failed"));
                    return Ok(Async::Ready(Some(receipt)));
                }
            };
            self.state = next_state;
        }
    }
}

/// options for relays from side to main
pub struct LogToSideToMainSignatures<T> {
    pub main: MainContract<T>,
    pub side: SideContract<T>,
}

/// from the options and a log a relay future can be made
impl<T: Transport> LogToFuture for LogToSideToMainSignatures<T> {
    type Future = SideToMainSignatures<T>;

    fn log_to_future(&self, log: Log) -> Self::Future {
        SideToMainSignatures::new(log, self.main.clone(), self.side.clone())
    }
}

// #[cfg(test)]
// mod tests {
//     use rustc_hex::FromHex;
//     use web3::types::{Bytes, Log};
//     use super::signatures_payload;
//
//     #[test]
//     fn test_signatures_payload() {
//         let my_address = "aff3454fce5edbc8cca8697c15331677e6ebcccc".into();
//
//         let data = "000000000000000000000000aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0".from_hex().unwrap();
//
//         let log = Log {
//             data: data.into(),
//             topics: vec![
//                 "eb043d149eedb81369bec43d4c3a3a53087debc88d2525f13bfaa3eecda28b5c".into(),
//             ],
//             transaction_hash: Some(
//                 "884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364".into(),
//             ),
//             ..Default::default()
//         };
//
//         let assignment = signatures_payload(2, my_address, log)
//             .unwrap()
//             .unwrap();
//         let expected_message: Bytes =
//             "490a32c600000000000000000000000000000000000000000000000000000000000000f0"
//                 .from_hex()
//                 .unwrap()
//                 .into();
//         let expected_signatures: Vec<Bytes> = vec![
// 			"1812d99600000000000000000000000000000000000000000000000000000000000000f00000000000000000000000000000000000000000000000000000000000000000".from_hex().unwrap().into(),
// 			"1812d99600000000000000000000000000000000000000000000000000000000000000f00000000000000000000000000000000000000000000000000000000000000001".from_hex().unwrap().into(),
// 		];
//         assert_eq!(expected_message, assignment.message_payload);
//         assert_eq!(expected_signatures, assignment.signature_payloads);
//     }
//
//     #[test]
//     fn test_signatures_payload_not_ours() {
//         let my_address = "aff3454fce5edbc8cca8697c15331677e6ebcccd".into();
//
//         let data = "000000000000000000000000aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0".from_hex().unwrap();
//
//         let log = Log {
//             data: data.into(),
//             topics: vec![
//                 "eb043d149eedb81369bec43d4c3a3a53087debc88d2525f13bfaa3eecda28b5c".into(),
//             ],
//             transaction_hash: Some(
//                 "884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364".into(),
//             ),
//             ..Default::default()
//         };
//
//         let assignment = signatures_payload(2, my_address, log).unwrap();
//         assert_eq!(None, assignment);
//     }
// }
