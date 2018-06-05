/// extraction of a pattern that occurred repeatedly in the codebase
///
/// where a "relay" is the detection of an event on chain A
/// followed by a transaction on chain B

use web3::types::Log;
use futures::{Async, Future, Poll, Stream};
use futures::future::{join_all, JoinAll};
use web3::Transport;
use log_stream::LogsInBlockRange;
use error::{self, ResultExt};
use std::collections::BTreeMap;
use std::collections::HashSet;
use future_heap::FutureHeap;

/// something that can create relay futures from logs.
/// to be called by `RelayStream` for every log.
pub trait LogToFuture {
    type Future: Future<Error = error::Error>;

    fn log_to_future(&self, log: &Log) -> Self::Future;
}

/// a tokio `Stream` that when polled fetches all new logs from `logs`
/// executes a `ForeignBridge.deposit`
/// stream yields last block number on `home` for which all `HomeBrige.Deposit`
/// events have been handled this way.
pub struct RelayStream<S: Stream<Item = LogsInBlockRange, Error = error::Error>, F: LogToFuture> {
    log_stream: S,
    log_to_future: F,
    /// maps the last block
    /// if all relays for this a block have finished yield that block
    future_heap: FutureHeap<u64, JoinAll<Vec<F::Future>>>
}

impl<S: Stream<Item = LogsInBlockRange, Error = error::Error>, F: LogToFuture> RelayStream<S, F> {
    pub fn new(log_stream: S, log_to_future: F) -> Self {
        Self {
            log_stream,
            log_to_future,
            future_heap: FutureHeap::new(),
        }
    }
}

impl<S: Stream<Item = LogsInBlockRange, Error = error::Error>, F: LogToFuture> Stream
    for RelayStream<S, F>
{
    type Item = u64;
    type Error = error::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        // on each poll we loop until neither any logs or relays are ready
        loop {
            // if there are logs
            // fetch them and add them all to the
            // map
            let maybe_logs_in_block_range = try_maybe_stream!(
                self.log_stream
                    .poll()
                    .chain_err(|| "RelayStream: fetching logs failed")
            );

            if let Some(ref logs_in_block_range) = maybe_logs_in_block_range {
                // keep track of the min number of block
                // where all logs have been relayed
                // and yield that number if it has changed

                // borrow checker
                let log_to_future = &self.log_to_future;

                // only after all Logs in the LogsInBlockRange have
                // been relayed can we safely mark the number
                // as done
                let futures: Vec<_> = logs_in_block_range.logs
                    .iter()
                    .map(|log| log_to_future.log_to_future(log))
                    .collect();
                let joined_futures = join_all(futures);
                self.future_heap.insert(logs_in_block_range.to, joined_futures);
            }

            let maybe_block_range_fully_relayed = try_maybe_stream!(
                self.future_heap
                    .poll()
                    .chain_err(|| "RelayStream: fetching logs failed")
            );

            if let Some((last_block, _)) = maybe_block_range_fully_relayed {
                return Ok(Async::Ready(Some(last_block)));
            }

            if maybe_logs_in_block_range.is_none() && maybe_block_range_fully_relayed.is_none() {
                // there are neither new logs nor any block range has been fully relayed
                return Ok(Async::NotReady);
            }
        }
    }
}
