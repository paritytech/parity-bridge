use ethereum_types::{Address, U256, H256};
use contracts::foreign::events::Withdraw;
use web3::types::Log;
use ethabi;
use error::Error;

pub struct MessageToMainnet {
	pub recipient: Address,
	pub value: U256,
	pub sidenet_transaction_hash: H256,
	pub mainnet_gas_price: U256,
}

pub const MESSAGE_LENGTH: usize = 116;

impl MessageToMainnet {
	pub fn from_bytes(bytes: &[u8]) -> Self {
		assert_eq!(bytes.len(), MESSAGE_LENGTH);

		MessageToMainnet {
			recipient: bytes[0..20].into(),
			value: bytes[20..52].into(),
			sidenet_transaction_hash: bytes[52..84].into(),
			mainnet_gas_price: bytes[84..MESSAGE_LENGTH].into(),
		}
	}

	/// construct a message from a `Withdraw` that was logged on `foreign`
	pub fn from_log(web3_log: Log) -> Result<Self, Error> {
		let ethabi_raw_log = ethabi::RawLog {
			topics: web3_log.topics,
			data: web3_log.data.0,
		};
		let withdraw_log = Withdraw::default().parse_log(ethabi_raw_log)?;
		// TODO [snd] replace expect by result
		let hash = web3_log.transaction_hash.expect("log to be mined and contain `transaction_hash`");
		Ok(Self {
			recipient: withdraw_log.recipient.0.into(),
			value: U256(withdraw_log.value.0),
			sidenet_transaction_hash: hash.to_vec().as_slice().into(),
			mainnet_gas_price: U256(withdraw_log.home_gas_price.0),
		})
	}

	/// mainly used to construct the message to be passed to
	/// `submitSignature`
	pub fn to_bytes(&self) -> Vec<u8> {
		let mut result = vec![0u8; MESSAGE_LENGTH];
		result[0..20].copy_from_slice(&self.recipient.0[..]);
		self.value.to_little_endian(&mut result[20..52]);
		result[52..84].copy_from_slice(&self.sidenet_transaction_hash.0[..]);
		self.mainnet_gas_price.to_little_endian(&mut result[84..MESSAGE_LENGTH]);
		return result;
	}

	pub fn to_payload(&self) -> Vec<u8> {
		ethabi::encode(&[ethabi::Token::Bytes(self.to_bytes())])
	}
}
