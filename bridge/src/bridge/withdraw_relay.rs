use futures::{Future, Poll, Stream, Async};
use futures::future::{join_all, Join, JoinAll, FromErr};
use tokio_timer::Timeout;
use web3::Transport;
use web3::types::{Address, Bytes, U256, H256, Log};
use web3;
use ethabi::{self, RawLog};
use log_stream::LogStream;
use contracts::{foreign, ForeignBridge, HomeBridge};
use error::{self, ResultExt};
use message_to_mainnet::MessageToMainnet;
use signature::Signature;
use contract_connection::ContractConnection;
use web3::helpers::CallResult;
use relay_stream::RelayFactory;

/// convert web3::Log to ethabi::RawLog since ethabi events can
/// only be parsed from the latter
fn web3_to_ethabi_log(web3_log: &web3::types::Log) -> ethabi::RawLog {
    RawLog {
        topics: web3_log.topics.iter().map(|t| t.0.into()).collect(),
        data: web3_log.data.0.clone(),
    }
}

fn log_to_collected_signatures(web3_log: &web3::types::Log) -> foreign::logs::CollectedSignatures {
    ForeignBridge::default()
        .events()
        .collected_signatures()
        .parse_log(web3_to_ethabi_log(web3_log))
        .expect("`Log` must be a from a `CollectedSignatures` event. q.e.d.")
}

/// state of the state machine that is the future responsible for
/// the SideToMain relay
enum State<T: Transport> {
    /// authority is not responsible for relaying this. noop.
    NotResponsible,
    AwaitMessageAndSignatures(Join<
        Timeout<FromErr<CallResult<Bytes, T::Out>, error::Error>>,
        JoinAll<Vec<Timeout<FromErr<CallResult<Bytes, T::Out>, error::Error>>>>,
    >),
    AwaitTransaction(Timeout<FromErr<CallResult<H256, T::Out>, error::Error>>),
}

pub struct SideToMainRelay<T: Transport> {
    tx_hash: H256,
    options: Options<T>,
    state: State<T>,
}

impl<T: Transport> SideToMainRelay<T> {
    pub fn new(log: Log, options: Options<T>) -> Self {
        let tx_hash = log.transaction_hash
            .expect("`log` must be mined and contain `transaction_hash`. q.e.d.");

        let event = log_to_collected_signatures(&log);

        let state = if event.authority_responsible_for_relay != options.address {
            info!("{:?} - this bridge node is not responsible for relaying transaction to main", tx_hash);
            // this bridge node is not responsible for relaying this transaction.
            // someone else will relay this transaction to home.
            State::NotResponsible
        } else {
            // fetch the actual message
            let message_payload = ForeignBridge::default()
                .functions()
                .message()
                .input(event.message_hash);
            let message_call = options.side.call(Bytes(message_payload));

            // fetch all the signatures on the message
            let signature_calls = (0..options.required_signatures)
                .into_iter()
                .map(|index| {
                    let payload = ForeignBridge::default()
                        .functions()
                        .signature()
                        .input(event.message_hash, index);
                    options.side.call(Bytes(payload))
                })
                .collect::<Vec<_>>();

            let future = message_call.join(join_all(signature_calls));

            info!("{:?} - step 1/3 - about to request message and signatures", tx_hash);
            State::AwaitMessageAndSignatures(future)
        };

        Self { tx_hash, options, state }
    }
}

impl<T: Transport> Future for SideToMainRelay<T> {
    /// transaction hash (if this authority is responsible for the relay)
    type Item = Option<H256>;
    type Error = error::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            let next_state = match self.state {
                State::NotResponsible => {
                    return Ok(Async::Ready(None));
                }
                State::AwaitMessageAndSignatures(ref mut future) => {
                    let (message_raw, signatures_raw) = try_ready!(future.poll()
                        .chain_err(|| "WithdrawRelay: fetching message and signatures failed"));

                    let message = ForeignBridge::default()
                        .functions()
                        .message()
                        .output(message_raw.0.as_slice())
                        .chain_err(|| "WithdrawRelay: decoding message failed")?;

                    let signatures = signatures_raw
                        .iter()
                        .map(|signature| {
                            Signature::from_bytes(
                                ForeignBridge::default()
                                    .functions()
                                    .signature()
                                    .output(signature.0.as_slice())
                                    .chain_err(|| "WithdrawRelay: decoding signature failed")?
                                    .as_slice(),
                            )
                        })
                        .collect::<error::Result<Vec<_>>>()?;

                    let payload: Bytes = HomeBridge::default()
                        .functions()
                        .withdraw()
                        .input(
                            signatures.iter().map(|x| x.v),
                            signatures.iter().map(|x| x.r),
                            signatures.iter().map(|x| x.s),
                            message.clone()
                        )
                        .into();

                    let gas_price = MessageToMainnet::from_bytes(&message)
                        .mainnet_gas_price;

                    info!("{:?} - step 2/3 - message and signatures received. about to send transaction", self.tx_hash);

                    let future = self.options.main.send_transaction(payload, self.options.gas, gas_price);

                    State::AwaitTransaction(future)
                },
                State::AwaitTransaction(ref mut future) => {
                    let tx_hash = try_ready!(future.poll()
                        .chain_err(|| "WithdrawRelay: sending transaction failed"));
                    info!("{:?} - step 3/3 - DONE - transaction sent {:?}", self.tx_hash, tx_hash);
                    return Ok(Async::Ready(Some(tx_hash)));
                }
            };
            self.state = next_state;
        }
    }
}

/// options for relays from side to main
#[derive(Clone)]
pub struct Options<T> {
    pub gas: U256,
    pub required_signatures: u32,
    pub address: Address,
    pub main: ContractConnection<T>,
    pub side: ContractConnection<T>,
}

/// from the options and a log a relay future can be made
impl<T: Transport> RelayFactory for Options<T> {
    type Relay = SideToMainRelay<T>;

    fn log_to_relay(&self, log: Log) -> Self::Relay {
        SideToMainRelay::new(log, self.clone())
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
