use std::ops;
use futures::{Future, Poll, Stream};
use futures::future::{join_all, JoinAll, FromErr};
use tokio_timer::Timeout;
use web3::Transport;
use web3::types::{Bytes, H256, H520, U256};
use log_stream::LogStream;
use contracts::foreign::ForeignBridge;
use error;
use message_to_mainnet::{MessageToMainnet, MESSAGE_LENGTH};
use contract_connection::ContractConnection;
use web3::helpers::CallResult;

fn withdraw_submit_signature_payload(
    withdraw_message: Vec<u8>,
    signature: H520,
) -> Bytes {
    assert_eq!(
        withdraw_message.len(),
        MESSAGE_LENGTH,
        "ForeignBridge never accepts messages with len != {} bytes; qed",
        MESSAGE_LENGTH
    );
    ForeignBridge::default()
        .functions()
        .submit_signature()
        .input(signature.0.to_vec(), withdraw_message)
        .into()
}

/// State of withdraw confirmation.
enum WithdrawConfirmState<T: Transport> {
    /// Withdraw confirm is waiting for logs.
    WaitForLogs,
    /// Signing withdraws.
    SignWithdraws {
        messages: Vec<Vec<u8>>,
        future: JoinAll<Vec<Timeout<FromErr<CallResult<H520, T::Out>, error::Error>>>>,
        block: u64,
    },
    /// Confirming withdraws.
    ConfirmWithdraws {
        future: JoinAll<Vec<Timeout<FromErr<CallResult<H256, T::Out>, error::Error>>>>,
        block: u64,
    },
    /// All withdraws till given block has been confirmed.
    Yield(Option<u64>),
}

pub struct WithdrawConfirm<T: Transport> {
    logs: LogStream<T>,
    foreign: ContractConnection<T>,
    gas: U256,
    gas_price: U256,
    state: WithdrawConfirmState<T>,
}

impl<T: Transport> WithdrawConfirm<T> {
    pub fn new(
        logs: LogStream<T>,
        foreign: ContractConnection<T>,
        gas: U256,
        gas_price: U256,
    ) -> Self {
        Self {
            logs,
            foreign,
            gas,
            gas_price,
            state: WithdrawConfirmState::WaitForLogs,
        }
    }
}

impl<T: Transport> Stream for WithdrawConfirm<T> {
    type Item = u64;
    type Error = error::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        loop {
            let next_state = match self.state {
                WithdrawConfirmState::WaitForLogs => {
                    let item = try_stream!(self.logs.poll());
                    info!("got {} new withdraws to sign", item.logs.len());
                    let withdraw_messages = item.logs
                        .into_iter()
                        .map(|log| {
                            info!(
                                "withdraw is ready for signature submission. tx hash {}",
                                log.transaction_hash.unwrap()
                            );
                            Ok(MessageToMainnet::from_log(log)?.to_bytes())
                        })
                        .collect::<Result<Vec<_>, Self::Error>>()?;

                    // borrow checker...
                    let foreign = &self.foreign;
                    let sign_requests = withdraw_messages
                        .clone()
                        .into_iter()
                        .map(|message| foreign.sign(Bytes(message)))
                        .collect::<Vec<_>>();

                    info!("signing");
                    WithdrawConfirmState::SignWithdraws {
                        future: join_all(sign_requests),
                        messages: withdraw_messages,
                        block: item.to,
                    }
                }
                WithdrawConfirmState::SignWithdraws {
                    ref mut future,
                    ref mut messages,
                    block,
                } => {
                    let signatures = try_ready!(future.poll());
                    info!("signing complete");

                    // borrow checker...
                    let foreign = &self.foreign;
                    let gas = self.gas;
                    let gas_price = self.gas_price;
                    let requests = messages
                        .drain(ops::RangeFull)
                        .zip(signatures.into_iter())
                        .map(|(withdraw_message, signature)| {
                            withdraw_submit_signature_payload(
                                withdraw_message,
                                signature,
                            )
                        })
                        .map(|payload| {
                            info!("submitting signature");
                            foreign.send_transaction(payload, gas, gas_price)
                        })
                        .collect::<Vec<_>>();

                    info!("submitting {} signatures", requests.len());
                    WithdrawConfirmState::ConfirmWithdraws {
                        future: join_all(requests),
                        block,
                    }
                }
                WithdrawConfirmState::ConfirmWithdraws {
                    ref mut future,
                    block,
                } => {
                    let _ = try_ready!(future.poll());
                    info!("submitting signatures complete");
                    WithdrawConfirmState::Yield(Some(block))
                }
                WithdrawConfirmState::Yield(ref mut block) => match block.take() {
                    None => {
                        info!("waiting for new withdraws that should get signed");
                        WithdrawConfirmState::WaitForLogs
                    }
                    some => return Ok(some.into()),
                },
            };
            self.state = next_state;
        }
    }
}
