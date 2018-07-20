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
use ::OrderedStream;

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
    stream_of_logs: S,
    log_to_future: F,
    /// maps the last block
    /// if all relays for this a block have finished yield that block
    ordered_stream: OrderedStream<u64, JoinAll<Vec<F::Future>>>,
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
        // on each poll we loop until neither any logs or relays are ready
        loop {
            // if there are logs
            // fetch them and add them all to the
            // map
            let maybe_logs_in_block_range = try_maybe_stream!(
                self.stream_of_logs
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
                let futures: Vec<_> = logs_in_block_range
                    .logs
                    .iter()
                    .map(|log| log_to_future.log_to_future(log))
                    .collect();
                let joined_futures = join_all(futures);
                self.ordered_stream
                    .insert(logs_in_block_range.to, joined_futures);
            }

            let maybe_block_range_fully_relayed = try_maybe_stream!(
                self.ordered_stream
                    .poll()
                    .chain_err(|| "RelayStream: relaying logs failed")
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
