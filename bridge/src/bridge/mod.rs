pub mod deploy;
mod deposit_relay;
mod withdraw_confirm;
mod withdraw_relay;

use futures::{Poll, Stream, Async};
use web3::Transport;

use database::{Database, State};
use config::{Config};
use log_stream::{LogStream, LogStreamOptions};
use error::{self, ResultExt};
use contracts::{HomeBridge, ForeignBridge};
use contract_connection::ContractConnection;
use relay_stream::RelayStream;

/// combines relays streams with the database.
/// (relay streams have no knowledge of the database.)
/// wraps the relay streams.
/// if polled polls all relay streams which causes them fetch
/// all pending relays and relay them
/// updates the database with results returned from relay streams.
pub struct Bridge<T: Transport> {
    deposits_relay: RelayStream<LogStream<T>, deposit_relay::Options<T>>,
    withdraws_relay: RelayStream<LogStream<T>, withdraw_relay::Options<T>>,
    withdraws_confirm: RelayStream<LogStream<T>, withdraw_confirm::Options<T>>,
    state: State
}

impl<T: Transport> Bridge<T> {
    pub fn new(
        config: Config,
        initial_state: State,
        home_transport: T,
        foreign_transport: T
    ) -> Self {

        let home_connection = ContractConnection::new(
            config.address,
            initial_state.home_contract_address,
            home_transport.clone(),
            config.home.request_timeout
        );

        let foreign_connection = ContractConnection::new(
            config.address,
            initial_state.foreign_contract_address,
            foreign_transport.clone(),
            config.foreign.request_timeout
        );

        let deposit_log_stream = LogStream::new(LogStreamOptions {
            filter: HomeBridge::default().events().deposit().create_filter(),
            request_timeout: config.home.request_timeout,
            poll_interval: config.home.poll_interval,
            confirmations: config.home.required_confirmations,
            transport: home_transport.clone(),
            contract_address: initial_state.home_contract_address,
            after: initial_state.checked_deposit_relay,
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
            contract_address: initial_state.foreign_contract_address,
            after: initial_state.checked_withdraw_confirm,
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
            contract_address: initial_state.foreign_contract_address,
            after: initial_state.checked_withdraw_relay,
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
            state: initial_state
        }
    }
}

impl<T: Transport> Stream for Bridge<T> {
    type Item = State;
    type Error = error::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        loop {
            let maybe_deposits_relay = try_maybe_stream!(self.deposits_relay.poll()
                .chain_err(|| "Bridge: polling deposits relay failed"));
            let maybe_withdraws_relay = try_maybe_stream!(self.withdraws_relay.poll()
                .chain_err(|| "Bridge: polling withdraws relay failed"));
            let maybe_withdraws_confirm = try_maybe_stream!(self.withdraws_confirm.poll()
                .chain_err(|| "Bridge: polling withdraws confirm failed"));

            let mut has_state_changed = false;

            if let Some(deposits_relay) = maybe_deposits_relay {
                info!("last block checked for deposit relay is now {}", deposits_relay);
                self.state.checked_deposit_relay = deposits_relay;
                has_state_changed = true;
            }
            if let Some(withdraws_relay) = maybe_withdraws_relay {
                info!("last block checked for withdraw relay is now {}", withdraws_relay);
                self.state.checked_withdraw_relay = withdraws_relay;
                has_state_changed = true;
            }
            if let Some(withdraws_confirm) = maybe_withdraws_confirm {
                info!("last block checked for withdraw confirm is now {}", withdraws_confirm);
                self.state.checked_withdraw_confirm = withdraws_confirm;
                has_state_changed = true;
            }

            if has_state_changed {
                return Ok(Async::Ready(Some(self.state.clone())));
            } else {
                return Ok(Async::NotReady);
            }
        }
    }
}
