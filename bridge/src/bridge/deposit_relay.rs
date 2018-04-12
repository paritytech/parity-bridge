use futures::{Future, Poll, Stream, Async};
use futures::future::{join_all, JoinAll, FromErr};
use tokio_timer::Timeout;
use web3::Transport;
use web3::types::{Bytes, H256, U256, Log};
use web3::helpers::CallResult;
use ethabi::RawLog;
use error;
use contracts::{HomeBridge, ForeignBridge};
use contract_connection::ContractConnection;
use relay_stream::RelayFactory;

/// takes `deposit_log` which must be a `HomeBridge.Deposit` event
/// and returns the payload for the call to `ForeignBridge.deposit()`
fn deposit_relay_payload(
    web3_log: Log,
) -> Vec<u8> {
    let tx_hash = web3_log.transaction_hash
        .expect("log must be mined and contain `transaction_hash`. q.e.d.");
    let raw_ethabi_log = RawLog {
        topics: web3_log.topics,
        data: web3_log.data.0,
    };
    let ethabi_log = HomeBridge::default()
        .events()
        .deposit()
        .parse_log(raw_ethabi_log)
        .expect("log must be a from a Deposit event. q.e.d.");
    ForeignBridge::default().functions().deposit().input(
        ethabi_log.recipient,
        ethabi_log.value,
        tx_hash.0,
    )
}

/// `Future` that relays a single deposit
pub struct DepositRelay<T: Transport> {
    tx_hash: H256,
    future: Timeout<FromErr<CallResult<H256, T::Out>, error::Error>>,
}

impl<T: Transport DepositRelay<T> {
    pub fn new(
        log: Log,
        foreign: ContractConnection<T>,
        gas: U256,
        gas_price: U256,
    ) -> Self {
        let tx_hash = log.transaction_hash
            .expect("`log` must be mined and contain `transaction_hash`. q.e.d.");
        let payload = deposit_relay_payload(log);

        Self {
            tx_hash,
            future: foreign.send_transaction(Bytes(payload), gas, gas_price),
        }
    }
}

impl<T: Transport> Future for DepositRelay<T> {
    type Item = ();
    type Error = error::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        info!("sending relay transaction for {:?}", self.tx_hash);
        let _ = try_ready!(self.future.poll());
        info!("finished relaying {:?}", self.tx_hash);
        Ok(Async::Ready(()))
    }
}

/// a thing that contains configuration for `DepositRelay`s
/// so it can take a `Log` and return a `DepositRelay`
pub struct DepositRelayFactory<T> {
    pub gas: U256,
    pub gas_price: U256,
    pub foreign: ContractConnection<T>,
}

impl<T: Transport> RelayFactory for DepositRelayFactory<T> {
    type Relay = DepositRelay<T>;
    fn log_to_relay(&self, log: Log) -> Self::Relay {
        DepositRelay::new(log, self.foreign.clone(), self.gas, self.gas_price)
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
