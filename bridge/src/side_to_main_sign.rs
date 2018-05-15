/// concerning the collection of signatures on `side`

use std::ops;
use futures::{Async, Future, Poll, Stream};
use futures::future::{join_all, FromErr, JoinAll};
use tokio_timer::Timeout;
use web3::Transport;
use web3::types::{Address, Bytes, H256, H520, Log, U256};
use log_stream::LogStream;
use contracts::foreign::ForeignBridge;
use error::{self, ResultExt};
use message_to_main::{MessageToMain, MESSAGE_LENGTH};
use web3::helpers::CallResult;
use relay_stream::LogToFuture;
use side_contract::{SideContract, IsSideToMainSignedOnSide};

enum State<T: Transport> {
    AwaitCallHasSubmittedSignature(Timeout<IsSideToMainSignedOnSide<T>>),
    AwaitSignature(Timeout<FromErr<CallResult<H520, T::Out>, error::Error>>),
    AwaitTransaction(Timeout<FromErr<CallResult<H256, T::Out>, error::Error>>),
}

pub struct SideToMainSign<T: Transport> {
    tx_hash: H256,
    side: SideContract<T>,
    message: MessageToMain,
    state: State<T>,
}


impl<T: Transport> SideToMainSign<T> {
    pub fn new(log: Log, side: SideContract<T>) -> Self {
        let tx_hash = log.transaction_hash
            .expect("`log` must be mined and contain `transaction_hash`. q.e.d.");

        let message =
            MessageToMain::from_log(log).expect("`log` must contain valid message. q.e.d.");
        let message_bytes = message.to_bytes();

        assert_eq!(
            message_bytes.len(),
            MESSAGE_LENGTH,
            "ForeignBridge never accepts messages with len != {} bytes; qed",
            MESSAGE_LENGTH
        );

        let future = side.sign(Bytes(message_bytes));
        let state = State::AwaitSignature(future);
        info!("{:?} - step 1/3 - about to sign message", tx_hash);

        Self {
            side,
            tx_hash,
            message,
            state,
        }
    }
}

impl<T: Transport> Future for SideToMainSign<T> {
    /// transaction hash
    type Item = H256;
    type Error = error::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            let next_state = match self.state {
                State::AwaitSignature(ref mut future) => {
                    let signature = try_ready!(
                        future
                            .poll()
                            .chain_err(|| "WithdrawConfirm: message signing failed")
                    );
                    info!(
                        "{:?} - step 2/3 - message signed. about to send transaction",
                        self.tx_hash
                    );

                    State::AwaitTransaction(future)
                }
                State::AwaitTransaction(ref mut future) => {
                    let tx_hash = try_ready!(
                        future
                            .poll()
                            .chain_err(|| "WithdrawConfirm: sending transaction failed")
                    );
                    info!(
                        "{:?} - step 3/3 - DONE - transaction sent {:?}",
                        self.tx_hash, tx_hash
                    );
                    return Ok(Async::Ready(tx_hash));
                }
            };
            self.state = next_state;
        }
    }
}

pub struct LogToSideToMainSign<T: Transport> {
    pub side: SideContract<T>
}

/// from the options and a log a relay future can be made
impl<T: Transport> LogToFuture for LogToSideToMainSign<T> {
    type Future = SideToMainSign<T>;

    fn log_to_future(&self, log: Log) -> Self::Future {
        SideToMainSign::new(log, self.side.clone())
    }
}
