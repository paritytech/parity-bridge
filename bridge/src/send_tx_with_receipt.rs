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

use error::{self, ResultExt};
use futures::future::FromErr;
use futures::{Future, Poll};
use std::time::Duration;
use tokio_timer::{Timeout, Timer};
use web3::api::Namespace;
use web3::helpers::CallFuture;
use web3::types::{TransactionReceipt, TransactionRequest, U64};
use web3::{self, Transport};

mod inner {
    use block_number_stream::{BlockNumberStream, BlockNumberStreamOptions};
    use error::{self, ResultExt};
    use futures::future::FromErr;
    use futures::{Async, Future, Poll, Stream};
    use std::time::Duration;
    use tokio_timer::{Timeout, Timer};
    use web3::api::Namespace;
    use web3::helpers::CallFuture;
    use web3::types::{TransactionReceipt, TransactionRequest, H256};
    use web3::{self, Transport};

    enum State<T: Transport> {
        AwaitSendTransaction(Timeout<FromErr<CallFuture<H256, T::Out>, error::Error>>),
        AwaitBlockNumber(H256),
        AwaitTransactionReceipt {
            future: Timeout<FromErr<CallFuture<Option<TransactionReceipt>, T::Out>, error::Error>>,
            transaction_hash: H256,
            last_block: u64,
        },
    }

    pub struct SendTransactionWithReceiptOptions<T: Transport> {
        pub transport: T,
        pub request_timeout: Duration,
        pub poll_interval: Duration,
        pub confirmations: u32,
        pub transaction: TransactionRequest,
        pub after: u64,
    }

    pub struct SendTransactionWithReceipt<T: Transport> {
        transport: T,
        state: State<T>,
        block_number_stream: BlockNumberStream<T>,
        request_timeout: Duration,
        timer: Timer,
    }

    impl<T: Transport> SendTransactionWithReceipt<T> {
        pub fn new(options: SendTransactionWithReceiptOptions<T>) -> Self {
            let timer = Timer::default();

            let block_number_stream_options = BlockNumberStreamOptions {
                request_timeout: options.request_timeout,
                poll_interval: options.poll_interval,
                confirmations: options.confirmations,
                transport: options.transport.clone(),
                after: options.after,
            };
            let block_number_stream = BlockNumberStream::new(block_number_stream_options);
            let future =
                web3::api::Eth::new(&options.transport).send_transaction(options.transaction);
            let future = timer.timeout(future.from_err(), options.request_timeout);

            SendTransactionWithReceipt {
                transport: options.transport,
                state: State::AwaitSendTransaction(future),
                block_number_stream,
                request_timeout: options.request_timeout,
                timer,
            }
        }
    }

    impl<T: Transport> Future for SendTransactionWithReceipt<T> {
        type Item = TransactionReceipt;
        type Error = error::Error;

        fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
            loop {
                let next_state = match self.state {
                    State::AwaitSendTransaction(ref mut future) => {
                        let hash = try_ready!(future.poll().chain_err(|| {
                            "SendTransactionWithReceipt: sending transaction failed"
                        }));
                        info!("SendTransactionWithReceipt: sent transaction {}", hash);
                        State::AwaitBlockNumber(hash)
                    }
                    State::AwaitBlockNumber(transaction_hash) => {
                        let last_block = match try_ready!(
                            self.block_number_stream
                                .poll()
                                .chain_err(|| "SendTransactionWithReceipt: fetching of last confirmed block failed")
                            ) {
                            Some(last_block) => last_block,
                            None => bail!("SendTransactionWithReceipt: fetching of last confirmed block failed"),
                        };

                        info!(
                            "SendTransactionWithReceipt: fetched confirmed block number {}",
                            last_block
                        );
                        let future = web3::api::Eth::new(&self.transport)
                            .transaction_receipt(transaction_hash);
                        State::AwaitTransactionReceipt {
                            future: self.timer.timeout(future.from_err(), self.request_timeout),
                            transaction_hash,
                            last_block,
                        }
                    }
                    State::AwaitTransactionReceipt {
                        ref mut future,
                        transaction_hash,
                        last_block,
                    } => {
                        let maybe_receipt = try_ready!(future.poll().chain_err(|| {
                            "SendTransactionWithReceipt: getting transaction receipt failed"
                        }));

                        match maybe_receipt {
                            // transaction hasn't been mined yet
                            None => State::AwaitBlockNumber(transaction_hash),
                            Some(receipt) => {
                                info!(
                                    "SendTransactionWithReceipt: got transaction receipt: {}",
                                    transaction_hash
                                );
                                match receipt.block_number {
                                    // receipt comes from pending block
                                    None => State::AwaitBlockNumber(transaction_hash),
                                    Some(receipt_block_number) => {
                                        if last_block < receipt_block_number.as_u64() {
                                            // transaction does not have enough confirmations
                                            State::AwaitBlockNumber(transaction_hash)
                                        } else {
                                            return Ok(Async::Ready(receipt));
                                        }
                                    }
                                }
                            }
                        }
                    }
                };

                self.state = next_state;
            }
        }
    }
}

enum State<T: Transport> {
    AwaitBlockNumber {
        future: Timeout<FromErr<CallFuture<U64, T::Out>, error::Error>>,
        transaction: Option<TransactionRequest>,
    },
    AwaitReceipt(inner::SendTransactionWithReceipt<T>),
}

pub struct SendTransactionWithReceiptOptions<T> {
    pub transport: T,
    pub request_timeout: Duration,
    pub poll_interval: Duration,
    pub confirmations: u32,
    pub transaction: TransactionRequest,
}

pub struct SendTransactionWithReceipt<T: Transport> {
    request_timeout: Duration,
    poll_interval: Duration,
    transport: T,
    state: State<T>,
    confirmations: u32,
}

impl<T: Transport> SendTransactionWithReceipt<T> {
    pub fn new(options: SendTransactionWithReceiptOptions<T>) -> Self {
        let timer = Timer::default();

        let future = web3::api::Eth::new(&options.transport).block_number();

        let state = State::AwaitBlockNumber {
            future: timer.timeout(future.from_err(), options.request_timeout),
            transaction: Some(options.transaction),
        };

        SendTransactionWithReceipt {
            request_timeout: options.request_timeout,
            poll_interval: options.poll_interval,
            transport: options.transport,
            state,
            confirmations: options.confirmations,
        }
    }
}

impl<T: Transport> Future for SendTransactionWithReceipt<T> {
    type Item = TransactionReceipt;
    type Error = error::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            let next_state = match self.state {
                State::AwaitBlockNumber {
                    ref mut future,
                    ref mut transaction,
                } => {
                    let block_number = try_ready!(future.poll().chain_err(|| {
                        "SendTransactionWithReceipt: fetching last block number failed"
                    }));
                    info!(
                        "SendTransactionWithReceipt: got last block number {}",
                        block_number
                    );
                    let transaction = transaction.take().expect(
                        "SendTransactionWithReceipt is always created with
                             State::AwaitBlockNumber with transaction set to Some; qed",
                    );

                    let inner_options = inner::SendTransactionWithReceiptOptions {
                        transport: self.transport.clone(),
                        request_timeout: self.request_timeout,
                        poll_interval: self.poll_interval,
                        confirmations: self.confirmations,
                        transaction,
                        after: block_number.as_u64(),
                    };

                    let future = inner::SendTransactionWithReceipt::new(inner_options);
                    State::AwaitReceipt(future)
                }
                State::AwaitReceipt(ref mut future) => return future.poll(),
            };

            self.state = next_state;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_core::reactor::Core;

    #[test]
    fn test_send_tx_with_receipt() {
        let transport = mock_transport!(
            "eth_blockNumber" =>
                req => json!([]),
                res => json!("0x1010");
            "eth_sendTransaction" =>
                req => json!([{
                    "data": "0x60",
                    "from": "0x006b5dda44dc2606f07ad86c9190fb54fd905f6d",
                    "gas": "0xf4240",
                    "gasPrice": "0x0"
                }]),
                res => json!("0x36efc16910ea67a2425a1e75f7e39e3c6a94f5763c68a47258f552481e20cd34");
            "eth_blockNumber" =>
                req => json!([]),
                res => json!("0x1011");
            "eth_blockNumber" =>
                req => json!([]),
                res => json!("0x1012");
            "eth_blockNumber" =>
                req => json!([]),
                res => json!("0x1013");
            "eth_getTransactionReceipt" =>
                req => json!(["0x36efc16910ea67a2425a1e75f7e39e3c6a94f5763c68a47258f552481e20cd34"]),
                res => json!(null);
            "eth_blockNumber" =>
                req => json!([]),
                res => json!("0x1014");
            "eth_getTransactionReceipt" =>
                req => json!(["0x36efc16910ea67a2425a1e75f7e39e3c6a94f5763c68a47258f552481e20cd34"]),
                res => json!({
                    "blockHash": null,
                    "blockNumber": null,
                    "contractAddress": "0xb1ac3a5584519119419a8e56422d912c782d8e5b",
                    "cumulativeGasUsed": "0x1c1999",
                    "gasUsed": "0xcdb5d",
                    "logs": [],
                    "logsBloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
                    "root": null,
                    "status": "0x1",
                    "transactionHash": "0x36efc16910ea67a2425a1e75f7e39e3c6a94f5763c68a47258f552481e20cd34",
                    "transactionIndex":"0x4"
                });
            "eth_blockNumber" =>
                req => json!([]),
                res => json!("0x1014");
            "eth_blockNumber" =>
                req => json!([]),
                res => json!("0x1015");
            "eth_getTransactionReceipt" =>
                req => json!(["0x36efc16910ea67a2425a1e75f7e39e3c6a94f5763c68a47258f552481e20cd34"]),
                res => json!({
                    "blockHash": "0xe0bdcf35b14a292d2998308d9b3fdea93a8c3d9c0b6c824c633fb9b15f9c3919",
                    "blockNumber": "0x1015",
                    "contractAddress": "0xb1ac3a5584519119419a8e56422d912c782d8e5b",
                    "cumulativeGasUsed": "0x1c1999",
                    "gasUsed": "0xcdb5d",
                    "logs": [],
                    "logsBloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
                    "root": null,
                    "status": "0x1",
                    "transactionHash": "0x36efc16910ea67a2425a1e75f7e39e3c6a94f5763c68a47258f552481e20cd34",
                    "transactionIndex":"0x4"
                });
            "eth_blockNumber" =>
                req => json!([]),
                res => json!("0x1017");
            "eth_getTransactionReceipt" =>
                req => json!(["0x36efc16910ea67a2425a1e75f7e39e3c6a94f5763c68a47258f552481e20cd34"]),
                res => json!({
                    "blockHash": "0xe0bdcf35b14a292d2998308d9b3fdea93a8c3d9c0b6c824c633fb9b15f9c3919",
                    "blockNumber": "0x1015",
                    "contractAddress": "0xb1ac3a5584519119419a8e56422d912c782d8e5b",
                    "cumulativeGasUsed": "0x1c1999",
                    "gasUsed": "0xcdb5d",
                    "logs": [],
                    "logsBloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
                    "root": null,
                    "status": "0x1",
                    "transactionHash": "0x36efc16910ea67a2425a1e75f7e39e3c6a94f5763c68a47258f552481e20cd34",
                    "transactionIndex":"0x4"
                });
        );

        let send_transaction_with_receipt =
            SendTransactionWithReceipt::new(SendTransactionWithReceiptOptions {
                transport: transport.clone(),
                request_timeout: Duration::from_secs(1),
                poll_interval: Duration::from_secs(0),
                confirmations: 2,
                transaction: TransactionRequest {
                    from: "006b5dda44dc2606f07ad86c9190fb54fd905f6d".parse().unwrap(),
                    to: None,
                    gas: Some(0xf4240.into()),
                    gas_price: Some(0.into()),
                    value: None,
                    data: Some(vec![0x60].into()),
                    nonce: None,
                    condition: None,
                },
            });

        let mut event_loop = Core::new().unwrap();
        let receipt = event_loop.run(send_transaction_with_receipt).unwrap();
        assert_eq!(
            receipt,
            TransactionReceipt {
                transaction_hash:
                    "36efc16910ea67a2425a1e75f7e39e3c6a94f5763c68a47258f552481e20cd34"
                        .parse()
                        .unwrap(),
                transaction_index: 0x4.into(),
                block_hash: Some(
                    "e0bdcf35b14a292d2998308d9b3fdea93a8c3d9c0b6c824c633fb9b15f9c3919"
                        .parse()
                        .unwrap()
                ),
                block_number: Some(0x1015.into()),
                cumulative_gas_used: 0x1c1999.into(),
                gas_used: "cdb5d".parse().ok(),
                contract_address: Some("b1ac3a5584519119419a8e56422d912c782d8e5b".parse().unwrap()),
                logs: vec![],
                status: Some(1.into()),
                logs_bloom: Default::default(),
            }
        );
        assert_eq!(transport.actual_requests(), transport.expected_requests());
    }
}
