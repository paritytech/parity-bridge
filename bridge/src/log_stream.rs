/// concerning logs

use std::time::Duration;
use tokio_timer::{Timer, Interval, Timeout};
use web3;
use web3::api::Namespace;
use web3::types::{Address, FilterBuilder, H256, Log, U256};
use web3::helpers::CallResult;
use futures::{Future, Poll, Stream};
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
    pub after: u64,
}

/// Stream of confirmed logs.
pub struct LogStream<T: Transport> {
    request_timeout: Duration,
    confirmations: usize,
    transport: T,
    after: u64,
    timer: Timer,
    interval: Interval,
    state: LogStreamState<T>,
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
            after: options.after,
            timer: timer,
            state: LogStreamState::Wait,
            filter: web3_filter(options.filter, options.contract_address),
        }
    }
}

/// Contains all logs matching `LogStream` filter in inclusive block range `[from, to]`.
#[derive(Debug, PartialEq)]
pub struct LogRange {
    pub from: u64,
    pub to: u64,
    pub logs: Vec<Log>,
}

/// Log Stream state.
enum LogStreamState<T: Transport> {
    /// Log Stream is waiting for timer to poll.
    Wait,
    /// Fetching best block number.
    FetchBlockNumber(Timeout<FromErr<CallResult<U256, T::Out>, error::Error>>),
    /// Fetching logs for new best block.
    FetchLogs {
        from: u64,
        to: u64,
        future: Timeout<FromErr<CallResult<Vec<Log>, T::Out>, error::Error>>,
    },
    /// All logs has been fetched.
    NextItem(Option<LogRange>),
}

impl<T: Transport> Stream for LogStream<T> {
    type Item = LogRange;
    type Error = error::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        loop {
            let next_state = match self.state {
                LogStreamState::Wait => {
                    // wait until `interval` has passed
                    let _ = try_stream!(self.interval.poll());
                    let future = web3::api::Eth::new(&self.transport)
                        .block_number();
                    LogStreamState::FetchBlockNumber(
                        self.timer.timeout(future.from_err(), self.request_timeout),
                    )
                }
                LogStreamState::FetchBlockNumber(ref mut future) => {
                    let last_block = try_ready!(future.poll()).low_u64();
                    let last_confirmed_block = last_block.saturating_sub(self.confirmations as u64);
                    if last_confirmed_block > self.after {
                        let from = self.after + 1;
                        let filter = self.filter
                            .clone()
                            .from_block(from.into())
                            .to_block(last_confirmed_block.into())
                            .build();
                        let future = web3::api::Eth::new(&self.transport)
                            .logs(&filter);
                        LogStreamState::FetchLogs {
                            from: from,
                            to: last_confirmed_block,
                            future: self.timer.timeout(future.from_err(), self.request_timeout),
                        }
                    } else {
                        LogStreamState::Wait
                    }
                }
                LogStreamState::FetchLogs {
                    ref mut future,
                    from,
                    to,
                } => {
                    let logs = try_ready!(future.poll());
                    let item = LogRange { from, to, logs };

                    self.after = to;
                    LogStreamState::NextItem(Some(item))
                }
                LogStreamState::NextItem(ref mut item) => match item.take() {
                    None => LogStreamState::Wait,
                    some => return Ok(some.into()),
                },
            };

            self.state = next_state;
        }
    }
}
