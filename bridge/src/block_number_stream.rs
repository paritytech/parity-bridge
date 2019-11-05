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
use futures::{Async, Future, Poll, Stream};
use std::time::Duration;
use tokio_timer::{Interval, Timeout, Timer};
use web3;
use web3::api::Namespace;
use web3::helpers::CallFuture;
use web3::types::U64;
use web3::Transport;

/// Block Number Stream state.
enum State<T: Transport> {
	AwaitInterval,
	AwaitBlockNumber(Timeout<FromErr<CallFuture<U64, T::Out>, error::Error>>),
}

pub struct BlockNumberStreamOptions<T> {
	pub request_timeout: Duration,
	pub poll_interval: Duration,
	pub confirmations: u32,
	pub transport: T,
	pub after: u64,
}

/// `Stream` that repeatedly polls `eth_blockNumber` and yields new block numbers.
pub struct BlockNumberStream<T: Transport> {
	request_timeout: Duration,
	confirmations: u32,
	transport: T,
	last_checked_block: u64,
	timer: Timer,
	poll_interval: Interval,
	state: State<T>,
}

impl<T: Transport> BlockNumberStream<T> {
	pub fn new(options: BlockNumberStreamOptions<T>) -> Self {
		let timer = Timer::default();

		BlockNumberStream {
			request_timeout: options.request_timeout,
			confirmations: options.confirmations,
			poll_interval: timer.interval(options.poll_interval),
			transport: options.transport,
			last_checked_block: options.after,
			timer,
			state: State::AwaitInterval,
		}
	}
}

impl<T: Transport> Stream for BlockNumberStream<T> {
	type Item = u64;
	type Error = error::Error;

	fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
		loop {
			let (next_state, value_to_yield) = match self.state {
				State::AwaitInterval => {
					// wait until `interval` has passed
					let _ = try_stream!(self
						.poll_interval
						.poll()
						.chain_err(|| format!("BlockNumberStream polling interval failed",)));
					info!("BlockNumberStream polling last block number");
					let future = web3::api::Eth::new(&self.transport).block_number();
					let next_state = State::AwaitBlockNumber(
						self.timer.timeout(future.from_err(), self.request_timeout),
					);
					(next_state, None)
				}
				State::AwaitBlockNumber(ref mut future) => {
					let last_block = try_ready!(future
						.poll()
						.chain_err(|| "BlockNumberStream: fetching of last block number failed"))
					.as_u64();
					info!(
						"BlockNumberStream: fetched last block number {}",
						last_block
					);
					// subtraction that saturates at zero
					let last_confirmed_block = last_block.saturating_sub(self.confirmations as u64);

					if self.last_checked_block < last_confirmed_block {
						self.last_checked_block = last_confirmed_block;
						(State::AwaitInterval, Some(last_confirmed_block))
					} else {
						info!("BlockNumberStream: no blocks confirmed since we last checked. waiting some more");
						(State::AwaitInterval, None)
					}
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
	use tokio_core::reactor::Core;

	#[test]
	fn test_block_number_stream() {
		let transport = mock_transport!(
			"eth_blockNumber" =>
				req => json!([]),
				res => json!("0x1011");
			"eth_blockNumber" =>
				req => json!([]),
				res => json!("0x1011");
			"eth_blockNumber" =>
				req => json!([]),
				res => json!("0x1012");
			"eth_blockNumber" =>
				req => json!([]),
				res => json!("0x1015");
		);

		let block_number_stream = BlockNumberStream::new(BlockNumberStreamOptions {
			request_timeout: Duration::from_secs(1),
			poll_interval: Duration::from_secs(0),
			confirmations: 12,
			transport: transport.clone(),
			after: 3,
		});

		let mut event_loop = Core::new().unwrap();
		let block_numbers = event_loop
			.run(block_number_stream.take(3).collect())
			.unwrap();

		assert_eq!(block_numbers, vec![0x1011 - 12, 0x1012 - 12, 0x1015 - 12]);
		assert_eq!(transport.actual_requests(), transport.expected_requests());
	}
}
