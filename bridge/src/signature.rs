/// ECDSA signatures:
/// conversion from/to byte vectors.
/// from/to v, r, s components.

use ethereum_types::H256;
use ethabi;

pub const SIGNATURE_LENGTH: usize = 65;

/// an ECDSA signature consisting of `v`, `r` and `s`
#[derive(PartialEq, Debug)]
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

#[cfg(test)]
mod test {
	use quickcheck::TestResult;
	use super::*;

	quickcheck! {
		fn quickcheck_signature_roundtrips(v: u8, r_raw: Vec<u8>, s_raw: Vec<u8>) -> TestResult {
			if r_raw.len() != 32 || s_raw.len() != 32 {
				return TestResult::discard();
			}

			let r: H256 = r_raw.as_slice().into();
			let s: H256 = s_raw.as_slice().into();
			let signature = Signature { v, r, s };
			assert_eq!(v, signature.v);
			assert_eq!(r, signature.r);
			assert_eq!(s, signature.s);

			let bytes = signature.to_bytes();
			assert_eq!(v, Signature::v_from_bytes(bytes.as_slice()));
			assert_eq!(r, Signature::r_from_bytes(bytes.as_slice()));
			assert_eq!(s, Signature::s_from_bytes(bytes.as_slice()));

			assert_eq!(signature, Signature::from_bytes(bytes.as_slice()));

			let payload = signature.to_payload();
			let mut tokens = ethabi::decode(&[ethabi::ParamType::Bytes], payload.as_slice())
				.unwrap();
			let decoded = tokens.pop().unwrap().to_bytes().unwrap();
			assert_eq!(signature, Signature::from_bytes(decoded.as_slice()));

			TestResult::passed()
		}
	}
}
