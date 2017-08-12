use web3::types::{H256, U256, Address};

pub struct EthereumDeposit {
	pub recipient: Address,
	pub value: U256,
	pub hash: H256,
}
