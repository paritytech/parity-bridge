use error::{self, ResultExt};
use ethabi;
use futures::future::FromErr;
use futures::{Async, Future, Poll, Stream};
use std::time::Duration;
use tokio_timer::{Interval, Timeout, Timer};
use web3;
use web3::api::Namespace;
use web3::helpers::CallResult;
use web3::types::{Address, FilterBuilder, H256, Log, U256};
use web3::Transport;

fn ethabi_topic_to_web3(topic: &ethabi::Topic<ethabi::Hash>) -> Option<Vec<H256>> {
    match topic {
        ethabi::Topic::Any => None,
        ethabi::Topic::OneOf(options) => Some(options.clone()),
        ethabi::Topic::This(hash) => Some(vec![hash.clone()]),
    }
}

fn filter_to_builder(filter: &ethabi::TopicFilter, address: Address) -> FilterBuilder {
    let t0 = ethabi_topic_to_web3(&filter.topic0);
    let t1 = ethabi_topic_to_web3(&filter.topic1);
    let t2 = ethabi_topic_to_web3(&filter.topic2);
    let t3 = ethabi_topic_to_web3(&filter.topic3);
    FilterBuilder::default()
        .address(vec![address])
        .topics(t0, t1, t2, t3)
}

/// options for creating a `LogStream`. passed to `LogStream::new`
pub struct LogStreamOptions<T> {
    pub filter: ethabi::TopicFilter,
    pub request_timeout: Duration,
    pub poll_interval: Duration,
    pub confirmations: u32,
    pub transport: T,
    pub contract_address: Address,
    pub after: u64,
}

/// Contains all logs matching `LogStream` filter in inclusive block range `[from, to]`.
#[derive(Debug, PartialEq)]
pub struct LogsInBlockRange {
    pub from: u64,
    pub to: u64,
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
        from: u64,
        to: u64,
        future: Timeout<FromErr<CallResult<Vec<Log>, T::Out>, error::Error>>,
    },
}

/// `futures::Stream` that fetches logs from `contract_address` matching `filter`
/// with adjustable `poll_interval` and `request_timeout`.
/// yields new logs that are `confirmations` blocks deep
pub struct LogStream<T: Transport> {
    request_timeout: Duration,
    confirmations: u32,
    transport: T,
    last_checked_block: u64,
    timer: Timer,
    poll_interval: Interval,
    state: State<T>,
    filter_builder: FilterBuilder,
    topic: Vec<H256>,
}

impl<T: Transport> LogStream<T> {
    /// creates a `LogStream`
    pub fn new(options: LogStreamOptions<T>) -> Self {
        let timer = Timer::default();

        let topic = ethabi_topic_to_web3(&options.filter.topic0)
            .expect("filter must have at least 1 topic. q.e.d.");
        let filter_builder = filter_to_builder(&options.filter, options.contract_address);
        LogStream {
            request_timeout: options.request_timeout,
            confirmations: options.confirmations,
            poll_interval: timer.interval(options.poll_interval),
            transport: options.transport,
            last_checked_block: options.after,
            timer: timer,
            state: State::AwaitInterval,
            filter_builder,
            topic,
        }
    }
}

impl<T: Transport> Stream for LogStream<T> {
    type Item = LogsInBlockRange;
    type Error = error::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        loop {
            let (next_state, value_to_yield) = match self.state {
                State::AwaitInterval => {
                    // wait until `interval` has passed
                    let _ = try_stream!(self.poll_interval.poll().chain_err(|| format!(
                        "LogStream (topic: #{:?}): polling interval failed",
                        self.topic
                    )));
                    info!(
                        "LogStream (topic: #{:?}): polling last block number",
                        self.topic
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
                            .chain_err(|| "LogStream: fetching of last block number failed")
                    ).as_u64();
                    info!("LogStream: fetched last block number {}", last_block);
                    // subtraction that saturates at zero
                    let last_confirmed_block = last_block.saturating_sub(self.confirmations as u64);

                    let next_state = if self.last_checked_block < last_confirmed_block {
                        let from = self.last_checked_block + 1;
                        let filter = self.filter_builder
                            .clone()
                            .from_block(from.into())
                            .to_block(last_confirmed_block.into())
                            .build();
                        let future = web3::api::Eth::new(&self.transport).logs(filter);

                        info!(
                            "LogStream: fetching logs in blocks {} to {}",
                            from, last_confirmed_block
                        );
                        State::AwaitLogs {
                            from: from,
                            to: last_confirmed_block,
                            future: self.timer.timeout(future.from_err(), self.request_timeout),
                        }
                    } else {
                        info!("LogStream: no blocks confirmed since we last checked. waiting some more");
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
                    info!(
                        "LogStream (topic: {:?}): fetched {} logs from {} to {}",
                        self.topic,
                        logs.len(),
                        from,
                        to
                    );
                    let log_range_to_yield = LogsInBlockRange { from, to, logs };

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

#[cfg(test)]
mod tests {
    use super::*;
    use contracts::home::HomeBridge;
    use helpers::StreamExt;
    use rustc_hex::FromHex;
    use tokio_core::reactor::Core;
    use web3::types::{Bytes, Log};

    #[test]
    fn test_log_stream_twice_no_logs() {
        let deposit_topic = HomeBridge::default()
            .events()
            .deposit()
            .create_filter()
            .topic0;

        let transport = mock_transport!(
            "eth_blockNumber" =>
                req => json!([]),
                res => json!("0x1011");
            "eth_getLogs" =>
                req => json!([{
                    "address": "0x0000000000000000000000000000000000000001",
                    "fromBlock": "0x4",
                    "toBlock": "0x1005",
                    "topics": [deposit_topic]
                }]),
                res => json!([]);
            "eth_blockNumber" =>
                req => json!([]),
                res => json!("0x1012");
            "eth_getLogs" =>
                req => json!([{
                    "address": "0x0000000000000000000000000000000000000001",
                    "fromBlock": "0x1006",
                    "toBlock": "0x1006",
                    "topics": [deposit_topic]
                }]),
                res => json!([]);
        );

        let log_stream = LogStream::new(LogStreamOptions {
            request_timeout: Duration::from_secs(1),
            poll_interval: Duration::from_secs(1),
            confirmations: 12,
            transport: transport.clone(),
            contract_address: "0000000000000000000000000000000000000001".into(),
            after: 3,
            filter: HomeBridge::default().events().deposit().create_filter(),
        });

        let mut event_loop = Core::new().unwrap();
        let log_ranges = event_loop.run(log_stream.take(2).collect()).unwrap();

        assert_eq!(
            log_ranges,
            vec![
                LogsInBlockRange {
                    from: 4,
                    to: 4101,
                    logs: vec![],
                },
                LogsInBlockRange {
                    from: 4102,
                    to: 4102,
                    logs: vec![],
                },
            ]
        );
        assert_eq!(transport.actual_requests(), transport.expected_requests());
    }

    #[test]
    fn test_log_stream_once_one_log() {
        let deposit_topic = HomeBridge::default()
            .events()
            .deposit()
            .create_filter()
            .topic0;

        let transport = mock_transport!(
            "eth_blockNumber" =>
                req => json!([]),
                res => json!("0x1011");
            "eth_getLogs" =>
                req => json!([{
                    "address": "0x0000000000000000000000000000000000000001",
                    "fromBlock": "0x4",
                    "toBlock": "0x1005",
                    "topics": [deposit_topic],
                }]),
                res => json!([{
                    "address": "0x0000000000000000000000000000000000000cc1",
                    "topics": [deposit_topic],
                    "data": "0x000000000000000000000000aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0",
                    "type": "",
                    "transactionHash": "0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"
                }]);
        );

        let log_stream = LogStream::new(LogStreamOptions {
            request_timeout: Duration::from_secs(1),
            poll_interval: Duration::from_secs(1),
            confirmations: 12,
            transport: transport.clone(),
            contract_address: "0000000000000000000000000000000000000001".into(),
            after: 3,
            filter: HomeBridge::default().events().deposit().create_filter(),
        });

        let mut event_loop = Core::new().unwrap();
        let log_ranges = event_loop.run(log_stream.take(1).collect()).unwrap();

        assert_eq!(
            log_ranges,
            vec![
                LogsInBlockRange { from: 4, to: 4101, logs: vec![
                    Log {
                        address: "0x0000000000000000000000000000000000000cc1".into(),
                        topics: deposit_topic.into(),
                        data: Bytes("000000000000000000000000aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0".from_hex().unwrap()),
                        transaction_hash: Some("0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364".into()),
                        block_hash: None,
                        block_number: None,
                        transaction_index: None,
                        log_index: None,
                        transaction_log_index: None,
                        log_type: None,
                        removed: None,
                    }
                ] },
            ]);
        assert_eq!(transport.actual_requests(), transport.expected_requests());
    }
}
