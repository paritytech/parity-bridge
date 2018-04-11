use futures::{Future, Poll, Stream};
use futures::future::{join_all, Join, JoinAll, FromErr};
use tokio_timer::Timeout;
use web3::Transport;
use web3::types::{Address, Bytes, U256, H256, Log};
use ethabi::{self, RawLog};
use log_stream::LogStream;
use contracts::{ForeignBridge, HomeBridge};
use error;
use message_to_mainnet::MessageToMainnet;
use signature::Signature;
use contract_connection::ContractConnection;
use web3::helpers::CallResult;

/// payloads for calls to `ForeignBridge.signature` and `ForeignBridge.message`
/// to retrieve the signatures (v, r, s) and messages
/// which the withdraw relay process should later relay to `HomeBridge`
/// by calling `HomeBridge.withdraw(v, r, s, message)`
#[derive(Debug, PartialEq)]
struct RelayAssignment {
    signature_payloads: Vec<Bytes>,
    message_payload: Bytes,
}

fn signatures_payload(
    required_signatures: u32,
    my_address: Address,
    log: Log,
) -> error::Result<Option<RelayAssignment>> {
    // convert web3::Log to ethabi::RawLog since ethabi events can
    // only be parsed from the latter
    let raw_log = RawLog {
        topics: log.topics.into_iter().map(|t| t.0.into()).collect(),
        data: log.data.0,
    };
    let collected_signatures = ForeignBridge::default().events().collected_signatures().parse_log(raw_log)?;
    if collected_signatures.authority_responsible_for_relay != my_address.0.into() {
        info!(
            "bridge not responsible for relaying transaction to home. tx hash: {}",
            log.transaction_hash.unwrap()
        );
        // this authority is not responsible for relaying this transaction.
        // someone else will relay this transaction to home.
        return Ok(None);
    }
    let signature_payloads = (0..required_signatures)
        .into_iter()
        .map(|index| {
            ForeignBridge::default()
                .functions()
                .signature()
                .input(collected_signatures.message_hash, index)
        })
        .map(Into::into)
        .collect();
    let message_payload = ForeignBridge::default()
        .functions()
        .message()
        .input(collected_signatures.message_hash)
        .into();

    Ok(Some(RelayAssignment {
        signature_payloads,
        message_payload,
    }))
}

/// state of the withdraw relay state machine
pub enum WithdrawsRelayState<T: Transport> {
    WaitForLogs,
    WaitForMessagesSignatures {
        future: Join<
            JoinAll<Vec<Timeout<FromErr<CallResult<Bytes, T::Out>, error::Error>>>>,
            JoinAll<Vec<JoinAll<Vec<Timeout<FromErr<CallResult<Bytes, T::Out>, error::Error>>>>>>,
        >,
        block: u64,
    },
    RelayWithdraws {
        future: JoinAll<Vec<Timeout<FromErr<CallResult<H256, T::Out>, error::Error>>>>,
        block: u64,
    },
    Yield(Option<u64>),
}

/// a tokio `Stream` that when polled fetches all new `ForeignBridge.CollectedSignatures`
/// events from `logs` and, if the node is responsible for the relay,
/// executes a `HomeBridge.withdraw` transaction and waits for the configured
/// confirmations.
/// stream yields last block on `foreign` for which all `ForeignBridge.CollectedSignatures`
/// events have been handled this way.
pub struct WithdrawsRelay<T: Transport> {
    logs: LogStream<T>,
    home: ContractConnection<T>,
    foreign: ContractConnection<T>,
    required_signatures: u32,
    gas: U256,
    state: WithdrawsRelayState<T>,
}

impl<T: Transport + Clone> WithdrawsRelay<T> {
    pub fn new(
        logs: LogStream<T>,
        home: ContractConnection<T>,
        foreign: ContractConnection<T>,
        required_signatures: u32,
        gas: U256,
    ) -> Self {
        Self {
            logs,
            home,
            foreign,
            required_signatures,
            gas,
            state: WithdrawsRelayState::WaitForLogs,
        }
    }
}

impl<T: Transport> Stream for WithdrawsRelay<T> {
    type Item = u64;
    type Error = error::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        loop {
            let next_state = match self.state {
                WithdrawsRelayState::WaitForLogs => {
                    let item = try_stream!(self.logs.poll());
                    info!("got {} new signed withdraws to relay", item.logs.len());
                    let assignments = item.logs
                        .into_iter()
                        .map(|log| {
                            info!(
                                "collected signature is ready for relay: tx hash: {}",
                                log.transaction_hash.unwrap()
                            );
                            signatures_payload(
                                self.required_signatures,
                                self.home.authority_address,
                                log,
                            )
                        })
                        .collect::<error::Result<Vec<_>>>()?;
                        // .map_err(|err|
                        //     error::annotate(err, "polling logs from home chain"))?;

                    let (signatures, messages): (Vec<_>, Vec<_>) = assignments
                        .into_iter()
                        .filter_map(|a| a)
                        .map(|assignment| {
                            (assignment.signature_payloads, assignment.message_payload)
                        })
                        .unzip();

                    let foreign = &self.foreign;
                    let message_calls = messages
                        .into_iter()
                        .map(|payload| foreign.call(payload))
                        .collect::<Vec<_>>();

                    let signature_calls = signatures
                        .into_iter()
                        .map(|payloads| {
                            payloads
                                .into_iter()
                                .map(|payload| foreign.call(payload))
                                .collect::<Vec<_>>()
                        })
                        .map(|calls| join_all(calls))
                        .collect::<Vec<_>>();

                    info!("fetching messages and signatures");
                    WithdrawsRelayState::WaitForMessagesSignatures {
                        future: join_all(message_calls).join(join_all(signature_calls)),
                        block: item.to,
                    }
                }
                WithdrawsRelayState::WaitForMessagesSignatures {
                    ref mut future,
                    block,
                } => {
                    let (messages_raw, signatures_raw) = try_ready!(future.poll());
                    info!("fetching messages and signatures complete");
                    assert_eq!(messages_raw.len(), signatures_raw.len());

                    let messages = messages_raw
                        .iter()
                        .map(|message| {
                            ForeignBridge::default()
                                .functions()
                                .message()
                                .output(message.0.as_slice())
                                .map(Bytes)
                        })
                        .collect::<ethabi::Result<Vec<_>>>()
                        .map_err(error::Error::from)?;
                    info!("messages decoded");

                    let signatures = signatures_raw
                        .iter()
                        .map(|signatures| {
                            signatures
                                .iter()
                                .map(|signature| {
                                    Signature::from_bytes(
                                        ForeignBridge::default()
                                            .functions()
                                            .signature()
                                            .output(signature.0.as_slice())?
                                            .as_slice(),
                                    )
                                })
                                .collect::<error::Result<Vec<_>>>()
                        })
                        .collect::<error::Result<Vec<_>>>()?;
                    info!("signatures decoded");

                    // borrow checker...
                    let home = &self.home;
                    let gas = self.gas;
                    let relays = messages
                        .into_iter()
                        .zip(signatures.into_iter())
                        .map(|(message, signatures)| {
                            let payload: Bytes = HomeBridge::default()
                                .functions()
                                .withdraw()
                                .input(
                                    signatures.iter().map(|x| x.v),
                                    signatures.iter().map(|x| x.r),
                                    signatures.iter().map(|x| x.s),
                                    message.clone().0,
                                )
                                .into();
                            let gas_price = MessageToMainnet::from_bytes(message.0.as_slice())
                                .mainnet_gas_price;
                            home.send_transaction(payload, gas, gas_price)
                        })
                        .collect::<Vec<_>>();

                    info!("relaying {} withdraws", relays.len());
                    WithdrawsRelayState::RelayWithdraws {
                        future: join_all(relays),
                        block,
                    }
                }
                WithdrawsRelayState::RelayWithdraws {
                    ref mut future,
                    block,
                } => {
                    let _ = try_ready!(future.poll());
                    info!("relaying withdraws complete");
                    WithdrawsRelayState::Yield(Some(block))
                }
                WithdrawsRelayState::Yield(ref mut block) => match block.take() {
                    None => {
                        info!("waiting for signed withdraws to relay");
                        WithdrawsRelayState::WaitForLogs
                    }
                    some => return Ok(some.into()),
                },
            };
            self.state = next_state;
        }
    }
}

#[cfg(test)]
mod tests {
    use rustc_hex::FromHex;
    use web3::types::{Bytes, Log};
    use super::signatures_payload;

    #[test]
    fn test_signatures_payload() {
        let my_address = "aff3454fce5edbc8cca8697c15331677e6ebcccc".into();

        let data = "000000000000000000000000aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0".from_hex().unwrap();

        let log = Log {
            data: data.into(),
            topics: vec![
                "eb043d149eedb81369bec43d4c3a3a53087debc88d2525f13bfaa3eecda28b5c".into(),
            ],
            transaction_hash: Some(
                "884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364".into(),
            ),
            ..Default::default()
        };

        let assignment = signatures_payload(2, my_address, log)
            .unwrap()
            .unwrap();
        let expected_message: Bytes =
            "490a32c600000000000000000000000000000000000000000000000000000000000000f0"
                .from_hex()
                .unwrap()
                .into();
        let expected_signatures: Vec<Bytes> = vec![
			"1812d99600000000000000000000000000000000000000000000000000000000000000f00000000000000000000000000000000000000000000000000000000000000000".from_hex().unwrap().into(),
			"1812d99600000000000000000000000000000000000000000000000000000000000000f00000000000000000000000000000000000000000000000000000000000000001".from_hex().unwrap().into(),
		];
        assert_eq!(expected_message, assignment.message_payload);
        assert_eq!(expected_signatures, assignment.signature_payloads);
    }

    #[test]
    fn test_signatures_payload_not_ours() {
        let my_address = "aff3454fce5edbc8cca8697c15331677e6ebcccd".into();

        let data = "000000000000000000000000aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0".from_hex().unwrap();

        let log = Log {
            data: data.into(),
            topics: vec![
                "eb043d149eedb81369bec43d4c3a3a53087debc88d2525f13bfaa3eecda28b5c".into(),
            ],
            transaction_hash: Some(
                "884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364".into(),
            ),
            ..Default::default()
        };

        let assignment = signatures_payload(2, my_address, log).unwrap();
        assert_eq!(None, assignment);
    }
}
