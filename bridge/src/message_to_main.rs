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

use contracts;
use error::Error;
use ethabi;
use ethereum_types::{Address, H256};
use helpers;
use tiny_keccak;
use web3::types::Log;

/// the message that is relayed from `side` to  `main`.
/// contains all the information required for the relay.
/// bridge nodes sign off on this message in `SideToMainSign`.
/// one node submits this message and signatures in `SideToMainSignatures`.
#[derive(PartialEq, Debug, Clone)]
pub struct MessageToMain {
	pub side_tx_hash: H256,
	pub message_id: H256,
	pub sender: Address,
	pub recipient: Address,
}

/// length of a `MessageToMain.to_bytes()` in bytes
pub const MESSAGE_LENGTH: usize = 32 + 32 + 20 + 20;

impl MessageToMain {
	/// parses message from a byte slice
	pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
		if bytes.len() != MESSAGE_LENGTH {
			bail!("`bytes`.len() must be {}", MESSAGE_LENGTH);
		}

		Ok(Self {
			side_tx_hash: H256::from_slice(&bytes[0..32]),
			message_id: H256::from_slice(&bytes[32..64]),
			sender: Address::from_slice(&bytes[64..84]),
			recipient: Address::from_slice(&bytes[84..104]),
		})
	}

	pub fn keccak256(&self) -> H256 {
		H256::from_slice(&tiny_keccak::keccak256(&self.to_bytes()))
	}

	/// construct a message from a `Withdraw` event that was logged on `side`
	pub fn from_log(raw_log: &Log) -> Result<Self, Error> {
		let hash = raw_log
			.transaction_hash
			.ok_or_else(|| "`log` must be mined and contain `transaction_hash`")?;
		let log = helpers::parse_log(contracts::side::events::relay_message::parse_log, raw_log)?;
		Ok(Self {
			side_tx_hash: hash,
			message_id: log.message_id,
			sender: log.sender,
			recipient: log.recipient,
		})
	}

	/// serializes message to a byte vector.
	/// mainly used to construct the message byte vector that is then signed
	/// and passed to `SideBridge.submitSignature`
	pub fn to_bytes(&self) -> Vec<u8> {
		let mut result = vec![0u8; MESSAGE_LENGTH];
		result[0..32].copy_from_slice(&self.side_tx_hash.0[..]);
		result[32..64].copy_from_slice(&self.message_id.0[..]);
		result[64..84].copy_from_slice(&self.sender.0[..]);
		result[84..104].copy_from_slice(&self.recipient.0[..]);
		return result;
	}

	/// serializes message to an ethabi payload
	pub fn to_payload(&self) -> Vec<u8> {
		ethabi::encode(&[ethabi::Token::Bytes(self.to_bytes())])
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use quickcheck::TestResult;
	use rustc_hex::FromHex;

	#[test]
	fn test_message_to_main_to_bytes() {
		let side_tx_hash: H256 = "75ebc3036b5a5a758be9a8c0e6f6ed8d46c640dda39845de99d9570ba76798e2"
			.parse()
			.unwrap();
		let message_id: H256 = "75ebc3036b5a5a758be9a8c0e6f6ed8d46c640dda39845de99d9570ba76798ff"
			.parse()
			.unwrap();
		let sender: Address = "eac4a655451e159313c3641e29824e77d6fcb0aa".parse().unwrap();
		let recipient: Address = "eac4a655451e159313c3641e29824e77d6fcb0bb".parse().unwrap();

		let message = MessageToMain {
			side_tx_hash,
			message_id,
			sender,
			recipient,
		};

		assert_eq!(message.to_bytes(), "75ebc3036b5a5a758be9a8c0e6f6ed8d46c640dda39845de99d9570ba76798e275ebc3036b5a5a758be9a8c0e6f6ed8d46c640dda39845de99d9570ba76798ffeac4a655451e159313c3641e29824e77d6fcb0aaeac4a655451e159313c3641e29824e77d6fcb0bb".from_hex::<Vec<u8>>().unwrap())
	}

	quickcheck! {
		fn quickcheck_message_to_main_roundtrips_to_bytes(
			side_tx_hash_raw: Vec<u8>,
			message_id_raw: Vec<u8>,
			sender_raw: Vec<u8>,
			recipient_raw: Vec<u8>
		) -> TestResult {
			if side_tx_hash_raw.len() != 32 ||
				message_id_raw.len() != 32 ||
				sender_raw.len() != 20 ||
				recipient_raw.len() != 20 {
				return TestResult::discard();
			}

			let side_tx_hash = H256::from_slice(side_tx_hash_raw.as_slice());
			let message_id = H256::from_slice(message_id_raw.as_slice());
			let sender = Address::from_slice(sender_raw.as_slice());
			let recipient = Address::from_slice(recipient_raw.as_slice());

			let message = MessageToMain {
				side_tx_hash,
				message_id,
				sender,
				recipient,
			};

			let bytes = message.to_bytes();
			assert_eq!(message, MessageToMain::from_bytes(bytes.as_slice()).unwrap());

			let payload = message.to_payload();
			let mut tokens = ethabi::decode(&[ethabi::ParamType::Bytes], payload.as_slice())
				.unwrap();
			let decoded = tokens.pop().unwrap().to_bytes().unwrap();
			assert_eq!(message, MessageToMain::from_bytes(decoded.as_slice()).unwrap());

			TestResult::passed()
		}
	}
}
