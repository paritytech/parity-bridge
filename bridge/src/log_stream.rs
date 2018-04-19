/// concerning logs

use std::time::Duration;
use tokio_timer::{Interval, Timeout, Timer};
use web3;
use web3::api::Namespace;
use web3::types::{Address, FilterBuilder, H256, Log, U256};
use web3::helpers::CallResult;
use futures::{Async, Future, Poll, Stream};
use futures::future::FromErr;
use web3::Transport;
use error::{self, ResultExt};
use ethabi;

fn web3_topic(topic: ethabi::Topic<ethabi::Hash>) -> Option<Vec<H256>> {
    let t: Vec<ethabi::Hash> = topic.into();
    // parity does not conform to an ethereum spec
    if t.is_empty() {
        None
    } else {
        Some(t)
    }
}

pub fn web3_filter(filter: ethabi::TopicFilter, address: Address) -> FilterBuilder {
    let t0 = web3_topic(filter.topic0);
    let t1 = web3_topic(filter.topic1);
    let t2 = web3_topic(filter.topic2);
    let t3 = web3_topic(filter.topic3);
    FilterBuilder::default()
        .address(vec![address])
        .topics(t0, t1, t2, t3)
}

/// passed to `LogStream::new`
pub struct LogStreamOptions<T> {
    pub filter: ethabi::TopicFilter,
    pub request_timeout: Duration,
    pub poll_interval: Duration,
    pub confirmations: usize,
    pub transport: T,
    pub contract_address: Address,
    pub after: U256,
}

/// Contains all logs matching `LogStream` filter in inclusive block range `[from, to]`.
#[derive(Debug, PartialEq)]
pub struct LogRange {
    pub from: U256,
    pub to: U256,
    pub logs: Vec<Log>,
}

/// Log Stream state.
enum State<T: Transport> {
    /// Log Stream is waiting for timer to poll.
    AwaitInterval,
    /// Fetching best block number.
    AwaitBlockNumber(Timeout<FromErr<CallResult<U256, T::Out>, error::Error>>),
    /// Fetching logs for new best block.
    AwaitLogs {
        from: U256,
        to: U256,
        future: Timeout<FromErr<CallResult<Vec<Log>, T::Out>, error::Error>>,
    },
}

/// Stream of confirmed logs.
pub struct LogStream<T: Transport> {
    request_timeout: Duration,
    confirmations: usize,
    transport: T,
    last_checked_block: U256,
    timer: Timer,
    interval: Interval,
    state: State<T>,
    filter: FilterBuilder,
}

impl<T: Transport> LogStream<T> {
    /// creates a new LogStream
    pub fn new(options: LogStreamOptions<T>) -> Self {
        let timer = Timer::default();
        LogStream {
            request_timeout: options.request_timeout,
            confirmations: options.confirmations,
            interval: timer.interval(options.poll_interval),
            transport: options.transport,
            last_checked_block: options.after,
            timer: timer,
            state: State::AwaitInterval,
            filter: web3_filter(options.filter, options.contract_address),
        }
    }
}

impl<T: Transport> Stream for LogStream<T> {
    type Item = LogRange;
    type Error = error::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        loop {
            let (next_state, value_to_yield) = match self.state {
                State::AwaitInterval => {
                    // wait until `interval` has passed
                    let _ = try_stream!(
                        self.interval
                            .poll()
                            .chain_err(|| "LogStream: polling interval failed")
                    );
                    let future = web3::api::Eth::new(&self.transport).block_number();
                    let next_state = State::AwaitBlockNumber(
                        self.timer.timeout(future.from_err(), self.request_timeout),
                    );
                    (next_state, None)
                }
                State::AwaitBlockNumber(ref mut future) => {
                    let last_block = try_ready!(
                        future
                            .poll()
                            .chain_err(|| "LogStream: fetching of block number failed")
                    );
                    // subtraction that saturates at zero
                    let last_confirmed_block = last_block.saturating_sub(self.confirmations.into());

                    let next_state = if self.last_checked_block < last_confirmed_block {
                        let from = self.last_checked_block + 1.into();
                        let filter = self.filter
                            .clone()
                            .from_block(from.as_u64().into())
                            .to_block(last_confirmed_block.as_u64().into())
                            .build();
                        let future = web3::api::Eth::new(&self.transport).logs(&filter);

                        State::AwaitLogs {
                            from: from,
                            to: last_confirmed_block,
                            future: self.timer.timeout(future.from_err(), self.request_timeout),
                        }
                    } else {
                        trace!("LogStream: no blocks confirmed since we last checked. waiting some more");
                        State::AwaitInterval
                    };

                    (next_state, None)
                }
                State::AwaitLogs {
                    ref mut future,
                    from,
                    to,
                } => {
                    let logs = try_ready!(
                        future
                            .poll()
                            .chain_err(|| "LogStream: polling web3 logs failed")
                    );
                    let log_range_to_yield = LogRange { from, to, logs };

                    self.last_checked_block = to;
                    (State::AwaitInterval, Some(log_range_to_yield))
                }
            };

            self.state = next_state;

            if value_to_yield.is_some() {
                return Ok(Async::Ready(value_to_yield));
            }
        }
    }
}
