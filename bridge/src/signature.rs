/// ECDSA signatures:
/// conversion from/to byte vectors.
/// from/to v, r, s components.

use ethereum_types::{Address, U256, H256};
use ethabi;

pub const SIGNATURE_LENGTH: usize = 65;

pub struct Signature {
	pub v: u8,
	pub r: H256,
	pub s: H256,
}

impl Signature {
	pub fn from_bytes(bytes: &[u8]) -> Self {
		assert_eq!(bytes.len(), SIGNATURE_LENGTH);

		Self {
			v: Self::v_from_bytes(bytes),
			r: Self::r_from_bytes(bytes),
			s: Self::s_from_bytes(bytes),
		}
	}

	pub fn v_from_bytes(bytes: &[u8]) -> u8 {
		bytes[64]
	}

	pub fn r_from_bytes(bytes: &[u8]) -> H256 {
		bytes[0..32].into()
	}

	pub fn s_from_bytes(bytes: &[u8]) -> H256 {
		bytes[32..64].into()
	}

	pub fn to_bytes(&self) -> Vec<u8> {
		let mut result = vec![0u8; SIGNATURE_LENGTH];
		result[0..32].copy_from_slice(&self.r.0[..]);
		result[32..64].copy_from_slice(&self.s.0[..]);
		result[64] = self.v;
		return result;
	}

	pub fn to_payload(&self) -> Vec<u8> {
		ethabi::encode(&[ethabi::Token::Bytes(self.to_bytes())])
	}
}
