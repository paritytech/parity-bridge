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
/// extraction of a pattern that occurred repeatedly in the codebase
///
/// where a "relay" is the detection of an event on chain A
/// followed by a transaction on chain B
use error::{self, ResultExt};
use futures::future::{join_all, JoinAll};
use futures::{Async, Future, Poll, Stream};
use log_stream::LogsInBlockRange;
use web3::types::Log;
use OrderedStream;

/// something that can create relay futures from logs.
/// to be called by `RelayStream` for every log.
pub trait LogToFuture {
    type Future: Future<Error = error::Error>;

    fn log_to_future(&self, log: &Log) -> Self::Future;
}

/// a tokio `Stream` that when polled fetches all new logs from `stream_of_logs`
/// calls `log_to_future` for each to obtain relay futures, waits for those
/// futures to complete and yields the block numbers for which all relay
/// futures have completed.
/// those block numbers can then be persisted since they'll never need to be
/// checked again.
pub struct RelayStream<S: Stream<Item = LogsInBlockRange, Error = error::Error>, F: LogToFuture> {
    stream_of_logs: S,
    log_to_future: F,
    /// reorders relay futures so they are yielded in block order
    /// rather than the order they complete.
    /// this is required because relay futures are not guaranteed to
    /// complete in block order.
    ordered_stream: OrderedStream<u64, F::Future>
}

impl<S: Stream<Item = LogsInBlockRange, Error = error::Error>, F: LogToFuture> RelayStream<S, F> {
    pub fn new(stream_of_logs: S, log_to_future: F) -> Self {
        Self {
            stream_of_logs,
            log_to_future,
            ordered_stream: OrderedStream::new(),
        }
    }
}

impl<S: Stream<Item = LogsInBlockRange, Error = error::Error>, F: LogToFuture> Stream
    for RelayStream<S, F>
{
    type Item = u64;
    type Error = error::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        // on each poll we loop until there are neither new logs
        // nor newly completed relays
        loop {
            let maybe_logs_in_block_range = try_maybe_stream!(
                self.stream_of_logs
                    .poll()
                    .chain_err(|| "RelayStream: fetching logs failed")
            );

            if let Some(ref logs_in_block_range) = maybe_logs_in_block_range {
                // if there are new logs, create futures from them
                // which are responsible for the relay and add them to the
                // ordered stream
                for log in &logs_in_block_range.logs {
                    let relay_future = self.log_to_future.log_to_future(log);
                    self.ordered_stream
                        .insert(logs_in_block_range.to, relay_future);
                }
            }

            let maybe_fully_relayed_until_block = try_maybe_stream!(
                self.ordered_stream
                    .poll()
                    .chain_err(|| "RelayStream: relaying logs failed")
            );

            if let Some((fully_relayed_until_block, _)) = maybe_fully_relayed_until_block {
                // all relay futures for this block or before have completed
                // we can yield the block number which can be safely
                // persisted since it doesn't need to get checked again
                return Ok(Async::Ready(Some(fully_relayed_until_block)));
            }

            if maybe_logs_in_block_range.is_none() && maybe_fully_relayed_until_block.is_none() {
                // there are neither new logs nor is there a new block number
                // until which all relays have completed
                return Ok(Async::NotReady);
            }
        }
    }
}
