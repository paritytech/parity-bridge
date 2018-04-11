use futures::{Future, Poll, Stream};
use futures::future::{join_all, JoinAll, FromErr};
use tokio_timer::Timeout;
use web3::Transport;
use web3::types::{Bytes, H256, U256, Log};
use web3::helpers::CallResult;
use ethabi::RawLog;
use log_stream::LogStream;
use error;
use contracts::{HomeBridge, ForeignBridge};
use contract_connection::ContractConnection;

/// takes `deposit_log` which must be a `HomeBridge.Deposit` event
/// and returns the payload for the call to `ForeignBridge.deposit()`
fn deposit_relay_payload(
    web3_log: Log,
) -> error::Result<Bytes> {
    let tx_hash = web3_log.transaction_hash
        .expect("log must be mined and contain `transaction_hash`");
    let raw_ethabi_log = RawLog {
        topics: web3_log.topics,
        data: web3_log.data.0,
    };
    let ethabi_log = HomeBridge::default().events().deposit().parse_log(raw_ethabi_log)?;
    let payload = ForeignBridge::default().functions().deposit().input(
        ethabi_log.recipient,
        ethabi_log.value,
        tx_hash.0,
    );
    Ok(payload.into())
}

SingleDepositRelayFactory {
    gas,
    gas_price,
    foreign: ContractConnection<T>,
}

impl<T: Transport> SingleDepositRelayFactory {
    pub fn log_to_relay(log: Log) -> SingleDepositRelay<T> {
        SingleDepositRelay::new(log, self.foreign, self.gas, self.gas_price)
    }
}

pub struct SingleDepositRelay<T: Transport> {
    future: Timeout<FromErr<CallResult<Bytes, T::Out>
}

impl<T: Transport + Clone> SingleDepositRelay<T> {
    pub fn new(
        log: Log,
        foreign: ContractConnection<T>,
        gas: U256,
        gas_price: U256,
    ) -> Result<Self> {
        let payload = deposit_relay_payload(log)?;
            // TODO annotate error

        Self {
            future: foreign.send_transaction(payload, gas, gas_price),
        }
    }
}

impl<T: Transport> Stream for DepositRelay<T> {
    type Item = ();
    type Error = error::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        info!("single_deposit_relay"
        let _ = try_ready!(future.poll());
        info!(
        Some(())
    }
}

/// State of deposits relay.
enum DepositBatchRelayState<T: Transport> {
    /// Deposit relay is waiting for logs.
    WaitForLogs,
    /// Relaying deposits in progress.
    WaitForRelays {
        future: JoinAll<Vec<Timeout<FromErr<CallResult<H256, T::Out>, error::Error>>>>,
        block: u64,
    },
    /// All deposits till given block has been relayed.
    Yield {
        block: u64,
    }
}

/// a tokio `Stream` that when polled fetches all new `HomeBridge.Deposit`
/// events from `logs` and for each of them executes a `ForeignBridge.deposit`
/// transaction and waits for the configured confirmations.
/// stream yields last block on `home` for which all `HomeBrige.Deposit`
/// events have been handled this way.
pub struct DepositRelayMany<T: Transport> {
    logs: LogStream<T>,
    log_to_relay: LogToRelay<T>
    foreign: ContractConnection<T>,
    gas: U256,
    gas_price: U256,
    state: DepositRelayState<T>,
}

impl<T: Transport + Clone> DepositRelayMany<T> {
    pub fn new(
        logs: LogStream<T>,
        log_to_relay: LogToRelay<T>
        foreign: ContractConnection<T>,
        gas: U256,
        gas_price: U256,
    ) -> Self {
        Self {
            logs,
            foreign,
            gas,
            gas_price,
            state: DepositRelayManyState::WaitForLogs,
        }
    }
}

impl<T: Transport> Stream for DepositRelay<T> {
    type Item = u64;
    type Error = error::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        loop {
            let next_state = match self.state {
                DepositRelayState::WaitForLogs => {
                    let log_range = try_stream!(self.logs.poll());

                    let foreign = self.foreign.clone();
                    let gas = self.gas;
                    let gas_price = self.gas_price;

                    let relays = log_range.logs
                        .into_iter()
                        .map(|log| DepositRelaySingle::new(
                            log, foreign
                        ))
                        .collect::<Result<Vec<_>, Self::Error>>()?;

                    DepositRelayState::WaitForRelays {
                        future: join_all(transactions),
                        block: item.to,
                    }
                }
                DepositRelayState::WaitForRelays {
                    ref mut future,
                    block,
                } => {
                    let _ = try_ready!(future.poll());
                    DepositRelayState::Yield(Some(block))
                }
                DepositRelayState::Yield(block) {
                    None => DepositRelayState::WaitForLogs,
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
    use super::deposit_relay_payload;

    #[test]
    fn test_deposit_relay_payload() {
        let data = "000000000000000000000000aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0".from_hex().unwrap();
        let log = Log {
            data: data.into(),
            topics: vec![
                "e1fffcc4923d04b559f4d29a8bfc6cda04eb5b0d3c460751c2402c5c5cc9109c".into(),
            ],
            transaction_hash: Some(
                "884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364".into(),
            ),
            ..Default::default()
        };

        let payload = deposit_relay_payload(log).unwrap();
        let expected: Bytes = "26b3293f000000000000000000000000aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364".from_hex().unwrap().into();
        assert_eq!(expected, payload);
    }
}
