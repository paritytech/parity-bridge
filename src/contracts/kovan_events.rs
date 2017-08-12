use web3::types::{Address, U256};

pub struct KovanDeposit {
	pub recipient: Address,
	pub value: U256,
}
