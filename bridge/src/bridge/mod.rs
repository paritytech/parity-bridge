mod deploy;
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

pub use self::deploy::{DeployForeign, DeployHome};
pub use self::deposit_relay::{DepositsRelay, LogToDepositRelay};
pub use self::withdraw_relay::WithdrawsRelay;
pub use self::withdraw_confirm::WithdrawsConfirm;

/// combines relays streams with the database.
/// (relay streams have no knowledge of the database.)
/// wraps the relay streams.
/// polls relay streams if polled.
/// updates the database with results returned from relay streams.
pub struct Bridge<T: Transport, D> {
    deposits_relay: DepositsRelay<T>,
    withdraws_relay: WithdrawsRelay<T>,
    withdraws_confirm: WithdrawsConfirm<T>,
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

        let log_to_deposit_relay = LogToDepositRelay {
            foreign: foreign_connection.clone(),
            gas: config.txs.deposit_relay.gas.into(),
            gas_price: config.txs.deposit_relay.gas_price.into()
        };

        let deposits_relay = DepositsRelay::new(deposit_log_stream, log_to_deposit_relay);

        let withdraw_log_stream = LogStream::new(LogStreamOptions {
            filter: ForeignBridge::default().events().withdraw().create_filter(),
            request_timeout: config.foreign.request_timeout,
            poll_interval: config.foreign.poll_interval,
            confirmations: config.foreign.required_confirmations,
            transport: foreign_transport.clone(),
            contract_address: state.foreign_contract_address,
            after: state.checked_withdraw_relay,
        });

        let withdraws_confirm = WithdrawsConfirm::new(
            withdraw_log_stream,
            foreign_connection.clone(),
            config.txs.withdraw_confirm.gas.into(),
            config.txs.withdraw_confirm.gas_price.into()
        );

        let collected_signatures_log_stream = LogStream::new(LogStreamOptions {
            filter: ForeignBridge::default().events().collected_signatures().create_filter(),
            request_timeout: config.foreign.request_timeout,
            poll_interval: config.foreign.poll_interval,
            confirmations: config.foreign.required_confirmations,
            transport: foreign_transport.clone(),
            contract_address: state.foreign_contract_address,
            after: state.checked_withdraw_relay,
        });

        let withdraws_relay = WithdrawsRelay::new(
            collected_signatures_log_stream,
            home_connection.clone(),
            foreign_connection.clone(),
            config.authorities.required_signatures,
            config.txs.withdraw_relay.gas.into(),
        );

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
