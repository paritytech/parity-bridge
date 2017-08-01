use std::sync::Arc;
use std::time::Duration;
use futures::{Future, Poll};
use web3::{self, Web3, Transport};
use web3::types::{H256, BlockId, Block, Transaction, TransactionReceipt, TransactionRequest};
use web3::confirm::{self, SendTransactionWithConfirmation};
use web3::helpers::CallResult;
use error::{Error, AppError};

/// Download block if there are matching events.
struct DownloadBlock;

/// Sends signed transaction
struct SendSignedTransaction;

/// Forwards transaction to mainnet.
struct ForwardTransactionToMainnet;

enum WithdrawLoop<T: Transport> {
	DownloadBlock(CallResult<Block<Transaction>, T::Out>),
	SendSignedTransaction(SendTransactionWithConfirmation<T>),
	ForwardTransactionToMainnet,
}

enum DepositLoop {
	DownloadBlock,
	ConfirmDeposit,
}

struct WithdrawHandler<T: Transport> {
	block_hash: H256,
	web3: Arc<Web3<T>>,
	state: WithdrawLoop<T>,
	transport: T,
}

impl<T: Transport> WithdrawHandler<T> {
	pub fn new(block_hash: H256, web3: Arc<Web3<T>>, transport: T) -> Self where T::Out: Send + 'static {
		let future = web3.eth().block_with_txs(BlockId::Hash(block_hash.clone()));
		WithdrawHandler {
			block_hash,
			web3,
			transport,
			state: WithdrawLoop::DownloadBlock(future),
		}
	}
}

impl<T: Transport + Clone> Future for WithdrawHandler<T> {
	// TODO: this should be a state to update db
	type Item = ();
	type Error = AppError;

	fn poll(&mut self) -> Poll<(), Self::Error> {
		loop {
			let next_state = match self.state {
				WithdrawLoop::DownloadBlock(ref mut future) => {
					let block = try_ready!(future.poll());
					let tx_request: TransactionRequest = { unimplemented!() };
					let tx_future = confirm::send_transaction_with_confirmation(self.transport.clone(), tx_request, Duration::from_secs(10), 12);
					WithdrawLoop::SendSignedTransaction(tx_future)
				},
				WithdrawLoop::SendSignedTransaction(ref mut future) => {
					//let confirmation 
					unimplemented!();
				},
				WithdrawLoop::ForwardTransactionToMainnet => {
					unimplemented!();
				},
			};
			self.state = next_state;
		}
		//unimplemented!();
	}
}
