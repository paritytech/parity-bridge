use web3::types::{Address, U256, H256, Bytes};

pub struct KovanDeposit {
	pub recipient: Address,
	pub value: U256,
}

pub struct KovanWithdraw {
	pub recipient: Address,
	pub value: U256,
	pub hash: H256,
}

impl KovanWithdraw {
	pub fn bytes(&self) -> Bytes {
		let mut bytes = vec![0u8; 84];
		bytes[0..20].copy_from_slice(&self.recipient);
		bytes[20..52].copy_from_slice(&self.value);
		bytes[52..84].copy_from_slice(&self.hash);
		bytes.into()
	}
}

pub struct KovanCollectSignatures;
