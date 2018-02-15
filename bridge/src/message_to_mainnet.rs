use ethereum_types::{Address, U256, H256};
use contracts::foreign::events::Withdraw;
use web3::types::Log;
use ethabi;
use error::{Error, ErrorKind};

/// the message that is relayed from side to main.
/// contains all the information required for the relay.
/// validators sign off on this message.
#[derive(PartialEq, Debug)]
pub struct MessageToMainnet {
	pub recipient: Address,
	pub value: U256,
	pub sidenet_transaction_hash: H256,
	pub mainnet_gas_price: U256,
}

/// length of a `MessageToMainnet.to_bytes()` in bytes
pub const MESSAGE_LENGTH: usize = 116;

impl MessageToMainnet {
	/// parses message from a byte slice
	pub fn from_bytes(bytes: &[u8]) -> Self {
		assert_eq!(bytes.len(), MESSAGE_LENGTH);

		Self {
			recipient: bytes[0..20].into(),
			value: U256::from_little_endian(&bytes[20..52]),
			sidenet_transaction_hash: bytes[52..84].into(),
			mainnet_gas_price: U256::from_little_endian(&bytes[84..MESSAGE_LENGTH]),
		}
	}

	/// construct a message from a `Withdraw` event that was logged on `foreign`
	pub fn from_log(web3_log: Log) -> Result<Self, Error> {
		let ethabi_raw_log = ethabi::RawLog {
			topics: web3_log.topics,
			data: web3_log.data.0,
		};
		let withdraw_log = Withdraw::default().parse_log(ethabi_raw_log)?;
		let hash = web3_log.transaction_hash
			.ok_or(Error::from_kind(ErrorKind::Msg("`log` must be mined and contain `transaction_hash`".into())))?;
		Ok(Self {
			recipient: withdraw_log.recipient,
			value: withdraw_log.value,
			sidenet_transaction_hash: hash,
			mainnet_gas_price: withdraw_log.home_gas_price,
		})
	}

	/// serializes message to a byte vector.
	/// mainly used to construct the message byte vector that is then signed
	/// and passed to `ForeignBridge.submitSignature`
	pub fn to_bytes(&self) -> Vec<u8> {
		let mut result = vec![0u8; MESSAGE_LENGTH];
		result[0..20].copy_from_slice(&self.recipient.0[..]);
		self.value.to_little_endian(&mut result[20..52]);
		result[52..84].copy_from_slice(&self.sidenet_transaction_hash.0[..]);
		self.mainnet_gas_price.to_little_endian(&mut result[84..MESSAGE_LENGTH]);
		return result;
	}

	/// serializes message to an ethabi payload
	pub fn to_payload(&self) -> Vec<u8> {
		ethabi::encode(&[ethabi::Token::Bytes(self.to_bytes())])
	}
}

#[cfg(test)]
mod test {
	use quickcheck::TestResult;
	use super::*;

	quickcheck! {
		fn quickcheck_message_to_mainnet_roundtrips_to_bytes(
			recipient_raw: Vec<u8>,
			value_raw: u64,
			sidenet_transaction_hash_raw: Vec<u8>,
			mainnet_gas_price_raw: u64
		) -> TestResult {
			if recipient_raw.len() != 20 || sidenet_transaction_hash_raw.len() != 32 {
				return TestResult::discard();
			}

			let recipient: Address = recipient_raw.as_slice().into();
			let value: U256 = value_raw.into();
			let sidenet_transaction_hash: H256 = sidenet_transaction_hash_raw.as_slice().into();
			let mainnet_gas_price: U256 = mainnet_gas_price_raw.into();

			let message = MessageToMainnet {
				recipient,
				value,
				sidenet_transaction_hash,
				mainnet_gas_price
			};

			let bytes = message.to_bytes();
			assert_eq!(message, MessageToMainnet::from_bytes(bytes.as_slice()));

			let payload = message.to_payload();
			let mut tokens = ethabi::decode(&[ethabi::ParamType::Bytes], payload.as_slice())
				.unwrap();
			let decoded = tokens.pop().unwrap().to_bytes().unwrap();
			assert_eq!(message, MessageToMainnet::from_bytes(decoded.as_slice()));

			TestResult::passed()
		}
	}
}
