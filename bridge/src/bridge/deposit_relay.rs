use futures::{Async, Future, Poll, Stream};
use futures::future::{join_all, FromErr, JoinAll};
use tokio_timer::Timeout;
use web3::Transport;
use web3::types::{Bytes, H256, Log, U256};
use web3::helpers::CallResult;
use ethabi::RawLog;
use error::{self, ResultExt};
use contracts::{ForeignBridge, HomeBridge};
use contract_connection::ContractConnection;
use relay_stream::RelayFactory;

/// takes `deposit_log` which must be a `HomeBridge.Deposit` event
/// and returns the payload for the call to `ForeignBridge.deposit()`
fn deposit_relay_payload(web3_log: Log) -> Vec<u8> {
    let tx_hash = web3_log
        .transaction_hash
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

/// `Future` responsible for doing a single relay from `main` to `side`
pub struct MainToSideRelay<T: Transport> {
    tx_hash: H256,
    future: Timeout<FromErr<CallResult<H256, T::Out>, error::Error>>,
}

impl<T: Transport> MainToSideRelay<T> {
    pub fn new(log: Log, options: Options<T>) -> Self {
        let tx_hash = log.transaction_hash
            .expect("`log` must be mined and contain `transaction_hash`. q.e.d.");
        let payload = deposit_relay_payload(log);
        info!("{:?} - step 1/2 - about to send transaction", tx_hash);
        let future =
            options
                .foreign
                .send_transaction(Bytes(payload), options.gas, options.gas_price);

        Self { tx_hash, future }
    }
}

impl<T: Transport> Future for MainToSideRelay<T> {
    /// transaction hash
    type Item = H256;
    type Error = error::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let tx_hash = try_ready!(
            self.future
                .poll()
                .chain_err(|| "DepositRelay: sending transaction failed")
        );
        info!(
            "{:?} - step 2/2 - DONE - transaction sent {:?}",
            self.tx_hash, tx_hash
        );
        Ok(Async::Ready(tx_hash))
    }
}

/// options for relays from side to main
#[derive(Clone)]
pub struct Options<T> {
    pub gas: U256,
    pub gas_price: U256,
    pub foreign: ContractConnection<T>,
}

/// from the options and a log a relay future can be made
impl<T: Transport> RelayFactory for Options<T> {
    type Relay = MainToSideRelay<T>;

    fn log_to_relay(&self, log: Log) -> Self::Relay {
        MainToSideRelay::new(log, self.clone())
    }
}

#[cfg(test)]
mod tests {
    use rustc_hex::FromHex;
    use web3::types::{Bytes, Log};
    use super::*;
    use tokio_core::reactor::Core;
    use contracts;
    use ethabi;
    use rustc_hex::ToHex;

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

        let payload = deposit_relay_payload(log);
        let expected: Vec<u8> = "26b3293f000000000000000000000000aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364".from_hex().unwrap();
        assert_eq!(expected, payload);
    }

    #[test]
    fn test_deposit_relay_future() {
        let deposit_topic = HomeBridge::default()
            .events()
            .deposit()
            .create_filter()
            .topic0;

        let log = contracts::home::logs::Deposit {
            recipient: "aff3454fce5edbc8cca8697c15331677e6ebcccc".into(),
            value: 1000.into(),
        };

        // TODO [snd] would be great if there were a way to automate this
        let log_data = ethabi::encode(&[
            ethabi::Token::Address(log.recipient),
            ethabi::Token::Uint(log.value)
        ]);

        let log_tx_hash = "0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364".into();

        let raw_log = Log {
            address: "0000000000000000000000000000000000000001".into(),
            topics: deposit_topic.into(),
            data: Bytes(log_data),
            transaction_hash: Some(log_tx_hash),
            ..Default::default()
        };

        let authority_address = "0000000000000000000000000000000000000001".into();

        let tx_hash = "0x1db8f385535c0d178b8f40016048f3a3cffee8f94e68978ea4b277f57b638f0b";
        let foreign_contract_address = "0000000000000000000000000000000000000dd1".into();

        let tx_data = ForeignBridge::default().functions().deposit().input(
            log.recipient,
            log.value,
            log_tx_hash
        );

        let transport = mock_transport!(
            "eth_sendTransaction" =>
                req => json!([{
                    "data": format!("0x{}", tx_data.to_hex()),
                    "from": "0x0000000000000000000000000000000000000001",
                    "gas": "0xfd",
                    "gasPrice": "0xa0",
                    "to": foreign_contract_address,
                }]),
            res => json!(tx_hash);
        );

        let connection = ContractConnection::new(
            authority_address,
            foreign_contract_address,
            transport.clone(),
            ::std::time::Duration::from_secs(1)
        );

        let options = Options {
            foreign: connection,
            gas: 0xfd.into(),
            gas_price: 0xa0.into(),
        };

        let future = MainToSideRelay::new(raw_log, options);

        let mut event_loop = Core::new().unwrap();
        let result = event_loop.run(future).unwrap();
        assert_eq!(result, tx_hash.into());

        assert_eq!(transport.actual_requests(), transport.expected_requests());
    }
}
