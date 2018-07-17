// Copyright 2017 Parity Technologies (UK) Ltd.
// This file is part of Parity-Bridge.

// Parity-Bridge is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity-Bridge is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity-Bridge.  If not, see <http://www.gnu.org/licenses/>.
use contracts;
use error::{self, ResultExt};
use futures::future::JoinAll;
use futures::{Async, Future, Poll, Stream};
use helpers;
use helpers::{AsyncCall, AsyncTransaction};
use main_contract::MainContract;
use message_to_main::MessageToMain;
use relay_stream::LogToFuture;
use side_contract::SideContract;
use signature::Signature;
use web3::api::Namespace;
use web3::types::{H256, Log};
use web3::Transport;

/// state of the state machine that is the future responsible for
/// the SideToMain relay
enum State<T: Transport> {
    AwaitMessage(AsyncCall<T, contracts::side::MessageWithInput>),
    /// authority is not responsible for relaying this. noop
    NotResponsible,
    AwaitIsRelayed {
        future: AsyncCall<T, contracts::main::WithdrawsWithInput>,
        message: MessageToMain,
    },
    AwaitSignatures {
        future: JoinAll<Vec<AsyncCall<T, contracts::side::SignatureWithInput>>>,
        message: MessageToMain,
    },
    AwaitTxSent(AsyncTransaction<T>),
}

pub struct SideToMainSignatures<T: Transport> {
    side_tx_hash: H256,
    main: MainContract<T>,
    side: SideContract<T>,
    state: State<T>,
}

impl<T: Transport> SideToMainSignatures<T> {
    pub fn new(raw_log: &Log, main: MainContract<T>, side: SideContract<T>) -> Self {
        let side_tx_hash = raw_log
            .transaction_hash
            .expect("`log` must be mined and contain `transaction_hash`. q.e.d.");

        let log = helpers::parse_log(&contracts::side::events::collected_signatures(), raw_log)
            .expect("`Log` must be a from a `CollectedSignatures` event. q.e.d.");

        let state = if log.authority_responsible_for_relay != main.authority_address {
            info!(
                "{:?} - this bridge node is not responsible for relaying transaction to main",
                side_tx_hash
            );
            // this bridge node is not responsible for relaying this transaction.
            // someone else will relay this transaction to main.
            State::NotResponsible
        } else {
            info!("{:?} - step 1/3 - about to fetch message", side_tx_hash,);
            State::AwaitMessage(side.call(contracts::side::functions::message(log.message_hash)))
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
    type Item = Option<H256>;
    type Error = error::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            let next_state = match self.state {
                State::NotResponsible => {
                    return Ok(Async::Ready(None));
                }
                State::AwaitMessage(ref mut future) => {
                    let message_bytes = try_ready!(
                        future
                            .poll()
                            .chain_err(|| "SubmitSignature: fetching message failed")
                    );
                    let message = MessageToMain::from_bytes(&message_bytes)?;
                    State::AwaitIsRelayed {
                        future: self.main
                            .call(contracts::main::functions::withdraws(message.side_tx_hash)),
                        message,
                    }
                }
                State::AwaitIsRelayed {
                    ref mut future,
                    ref message,
                } => {
                    let is_relayed = try_ready!(
                        future
                            .poll()
                            .chain_err(|| "SubmitSignature: fetching message failed")
                    );

                    if is_relayed {
                        return Ok(Async::Ready(None));
                    }

                    State::AwaitSignatures {
                        future: self.side.get_signatures(message.keccak256()),
                        message: message.clone(),
                    }
                }
                State::AwaitSignatures {
                    ref mut future,
                    ref message,
                } => {
                    let raw_signatures = try_ready!(
                        future
                            .poll()
                            .chain_err(|| "WithdrawRelay: fetching message and signatures failed")
                    );
                    let signatures: Vec<Signature> = raw_signatures
                        .iter()
                        .map(|x| Signature::from_bytes(x))
                        .collect::<Result<_, _>>()?;
                    info!("{:?} - step 2/3 - message and {} signatures received. about to send transaction", self.side_tx_hash, signatures.len());
                    State::AwaitTxSent(self.main.relay_side_to_main(&message, &signatures))
                }
                State::AwaitTxSent(ref mut future) => {
                    let main_tx_hash = try_ready!(
                        future
                            .poll()
                            .chain_err(|| "WithdrawRelay: sending transaction failed")
                    );
                    info!(
                        "{:?} - step 3/3 - DONE - transaction sent {:?}",
                        self.side_tx_hash, main_tx_hash
                    );
                    return Ok(Async::Ready(Some(main_tx_hash)));
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

    fn log_to_future(&self, log: &Log) -> Self::Future {
        SideToMainSignatures::new(log, self.main.clone(), self.side.clone())
    }
}
