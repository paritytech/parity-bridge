pub mod deploy;
mod deposit_relay;
mod withdraw_confirm;
mod withdraw_relay;

use futures::{Poll, Stream, Async};
use web3::Transport;

use database::Database;
use config::{Config};
use log_stream::{LogStream, LogStreamOptions};
use error::Error;
use contracts::{HomeBridge, ForeignBridge};
use contract_connection::ContractConnection;
use relay_stream::RelayStream;

/// combines relays streams with the database.
/// (relay streams have no knowledge of the database.)
/// wraps the relay streams.
/// polls relay streams if polled.
/// updates the database with results returned from relay streams.
pub struct Bridge<T: Transport, D> {
    deposits_relay: RelayStream<T, deposit_relay::Options<T>>,
    withdraws_relay: RelayStream<T, withdraw_relay::Options<T>>,
    withdraws_confirm: RelayStream<T, withdraw_confirm::Options<T>>,
    database: D,
}

impl<T: Transport, D: Database> Bridge<T, D> {
    pub fn new(
        config: Config,
        home_transport: T,
        foreign_transport: T,
        database: D
    ) -> Self {
        let state = database.read();

        let home_connection = ContractConnection::new(
            config.address,
            state.home_contract_address,
            home_transport.clone(),
            config.home.request_timeout
        );

        let foreign_connection = ContractConnection::new(
            config.address,
            state.foreign_contract_address,
            foreign_transport.clone(),
            config.foreign.request_timeout
        );

        let deposit_log_stream = LogStream::new(LogStreamOptions {
            filter: HomeBridge::default().events().deposit().create_filter(),
            request_timeout: config.home.request_timeout,
            poll_interval: config.home.poll_interval,
            confirmations: config.home.required_confirmations,
            transport: home_transport.clone(),
            contract_address: state.home_contract_address,
            after: state.checked_deposit_relay,
        });

        let main_to_side_options = deposit_relay::Options {
            foreign: foreign_connection.clone(),
            gas: config.txs.deposit_relay.gas.into(),
            gas_price: config.txs.deposit_relay.gas_price.into()
        };

        let deposits_relay = RelayStream::new(deposit_log_stream, main_to_side_options);

        let withdraw_log_stream = LogStream::new(LogStreamOptions {
            filter: ForeignBridge::default().events().withdraw().create_filter(),
            request_timeout: config.foreign.request_timeout,
            poll_interval: config.foreign.poll_interval,
            confirmations: config.foreign.required_confirmations,
            transport: foreign_transport.clone(),
            contract_address: state.foreign_contract_address,
            after: state.checked_withdraw_relay,
        });

        let withdraw_confirm_options = withdraw_confirm::Options {
            side: foreign_connection.clone(),
            gas: config.txs.withdraw_confirm.gas.into(),
            gas_price: config.txs.withdraw_confirm.gas_price.into(),
            address: config.address,
        };

        let withdraws_confirm = RelayStream::new(withdraw_log_stream, withdraw_confirm_options);

        let collected_signatures_log_stream = LogStream::new(LogStreamOptions {
            filter: ForeignBridge::default().events().collected_signatures().create_filter(),
            request_timeout: config.foreign.request_timeout,
            poll_interval: config.foreign.poll_interval,
            confirmations: config.foreign.required_confirmations,
            transport: foreign_transport.clone(),
            contract_address: state.foreign_contract_address,
            after: state.checked_withdraw_relay,
        });

        let side_to_main_options = withdraw_relay::Options {
            main: home_connection.clone(),
            side: foreign_connection.clone(),
            gas: config.txs.withdraw_relay.gas.into(),
            required_signatures: config.authorities.required_signatures,
            address: config.address
        };

        let withdraws_relay = RelayStream::new(collected_signatures_log_stream, side_to_main_options);

        Self {
            deposits_relay,
            withdraws_confirm,
            withdraws_relay,
            database,
        }
    }
}

impl<T: Transport, D: Database> Stream for Bridge<T, D> {
    type Item = ();
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        // only proceed once all three streams are Async::Ready
        let deposits_relay = try_stream!(self.deposits_relay.poll());
        let withdraws_relay = try_stream!(self.withdraws_relay.poll());
        let withdraws_confirm = try_stream!(self.withdraws_confirm.poll());

        // update the state
        let mut state = self.database.read();
        state.checked_deposit_relay = deposits_relay;
        state.checked_withdraw_relay = withdraws_relay;
        state.checked_withdraw_confirm = withdraws_confirm;
        self.database.write(&state)?;

        return Ok(Async::Ready(Some(())));
    }
}
