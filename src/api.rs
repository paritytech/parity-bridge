use std::vec;
use std::time::Duration;
use futures::{Future, Stream, Poll};
use futures_after::{After, AfterStream};
use web3::{self, api, Transport};
use web3::api::{Namespace, FilterStream, CreateFilter};
use web3::types::{Log, Filter, H256, Block, BlockId, BlockNumber};
use web3::helpers::CallResult;
use error::Error;

pub use web3::confirm::send_transaction_with_confirmation;

pub fn logs<T: Transport>(transport: T, filter: &Filter) -> CallResult<Vec<Log>, T::Out> {
	api::Eth::new(transport).logs(filter)
}

pub fn block<T: Transport>(transport: T, id: BlockId) -> CallResult<Block<H256>, T::Out> {
	api::Eth::new(transport).block(id)
}

pub fn create_blocks_filter<T: Transport + Clone>(transport: T) -> CreateFilter<T, H256> {
	api::EthFilter::new(transport).create_blocks_filter()
}

pub enum BlockNumbersStreamState<T: Transport> {
	WaitForNextBlock,
	FetchBlock(CallResult<Block<H256>, T::Out>),
	NextItem(Option<BlockNumber>),
}

pub struct BlockNumbersStream<T: Transport> {
	transport: T,
	stream: FilterStream<T, H256>,
	state: BlockNumbersStreamState<T>,
}

impl<T: Transport> BlockNumbersStream<T> {
	fn new(transport: T, stream: FilterStream<T, H256>) -> Self {
		BlockNumbersStream {
			transport,
			stream,
			state: BlockNumbersStreamState::WaitForNextBlock,
		}
	}
}

impl<T: Transport> Stream for BlockNumbersStream<T> {
	type Item = BlockNumber;
	type Error = web3::Error;

	fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
		loop {
			let next_state = match self.state {
				BlockNumbersStreamState::WaitForNextBlock => match try_ready!(self.stream.poll()) {
					Some(hash) => BlockNumbersStreamState::FetchBlock(block(&self.transport, hash.into())),
					None => return Ok(None.into()),
				},
				BlockNumbersStreamState::FetchBlock(ref mut future) => {
					let block = try_ready!(future.poll());
					let block_number = block.number.expect("block number to exist for mined block");
					BlockNumbersStreamState::NextItem(Some(BlockNumber::Number(block_number.low_u64())))
				},
				BlockNumbersStreamState::NextItem(ref mut item) => match item.take() {
					None => BlockNumbersStreamState::WaitForNextBlock,
					some => return Ok(some.into()),
				}
			};

			self.state = next_state;
		}
	}
}

pub enum LogsStreamState<T: Transport> {
	WaitForNextBlock,
	FetchLogs(CallResult<Vec<Log>, T::Out>),
	NextLog(vec::IntoIter<Log>),
}

pub struct LogsStream<T: Transport> {
	transport: T,
	state: LogsStreamState<T>,
	stream: After<BlockNumbersStream<T>>,
	filter: Filter,
}

impl<T: Transport + Clone> LogsStream<T> {
	fn new(transport: T, stream: FilterStream<T, H256>, filter: Filter, confirmations: usize) -> Self {
		LogsStream {
			stream: BlockNumbersStream::new(transport.clone(), stream).after(confirmations),
			state: LogsStreamState::WaitForNextBlock,
			transport,
			filter,
		}
	}
}

impl<T: Transport> Stream for LogsStream<T> {
	type Item = Log;
	type Error = web3::Error;

	fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
		loop {
			let next_state = match self.state {
				LogsStreamState::WaitForNextBlock => match try_ready!(self.stream.poll()) {
					Some(number) => {
						self.filter.from_block = Some(number.clone());
						self.filter.to_block = Some(number);
						LogsStreamState::FetchLogs(logs(&self.transport, &self.filter))
					},
					None => return Ok(None.into()),
				},
				LogsStreamState::FetchLogs(ref mut future) => {
					let logs = try_ready!(future.poll());
					LogsStreamState::NextLog(logs.into_iter())
				},
				LogsStreamState::NextLog(ref mut iter) => match iter.next() {
					None => LogsStreamState::WaitForNextBlock,
					some => return Ok(some.into()),
				},
			};
			self.state = next_state;
		}
	}
}

pub struct CreateLogsStream<T: Transport> {
	create_filter: CreateFilter<T, H256>,
	transport: T,
	log_filter: Filter,
	poll_interval: Duration,
	confirmations: usize,
}

impl<T: Transport + Clone> Future for CreateLogsStream<T> {
	type Item = LogsStream<T>;
	type Error = web3::Error;

	fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
		let filter = try_ready!(self.create_filter.poll());
		let stream = LogsStream::new(self.transport.clone(), filter.stream(self.poll_interval), self.log_filter.clone(), self.confirmations);
		Ok(stream.into())
	}
}

pub fn create_logs_stream_with_confirmations<T: Transport + Clone>(transport: T, log_filter: Filter, poll_interval: Duration, confirmations: usize) -> CreateLogsStream<T> {
	CreateLogsStream {
		create_filter: create_blocks_filter(transport.clone()),
		transport,
		log_filter,
		poll_interval,
		confirmations,
	}
}
