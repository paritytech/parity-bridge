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
use ethabi;
/// ECDSA signatures:
/// conversion from/to byte vectors.
/// from/to v, r, s components.
use ethereum_types::H256;

use error::Error;

pub const SIGNATURE_LENGTH: usize = 65;

/// an ECDSA signature consisting of `v`, `r` and `s`
#[derive(PartialEq, Debug)]
pub struct Signature {
    pub v: u8,
    pub r: H256,
    pub s: H256,
}

impl Signature {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() != SIGNATURE_LENGTH {
            bail!("`bytes`.len() must be {}", SIGNATURE_LENGTH);
        }

        Ok(Self {
            v: bytes[64],
            r: H256::from_slice(&bytes[0..32]),
            s: H256::from_slice(&bytes[32..64]),
        })
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
    use super::*;
    use quickcheck::TestResult;

    quickcheck! {
        fn quickcheck_signature_roundtrips(v: u8, r_raw: Vec<u8>, s_raw: Vec<u8>) -> TestResult {
            if r_raw.len() != 32 || s_raw.len() != 32 {
                return TestResult::discard();
            }

            let r = H256::from_slice(r_raw.as_slice());
            let s = H256::from_slice(s_raw.as_slice());
            let signature = Signature { v, r, s };
            assert_eq!(v, signature.v);
            assert_eq!(r, signature.r);
            assert_eq!(s, signature.s);

            let bytes = signature.to_bytes();

            assert_eq!(signature, Signature::from_bytes(bytes.as_slice()).unwrap());

            let payload = signature.to_payload();
            let mut tokens = ethabi::decode(&[ethabi::ParamType::Bytes], payload.as_slice())
                .unwrap();
            let decoded = tokens.pop().unwrap().to_bytes().unwrap();
            assert_eq!(signature, Signature::from_bytes(decoded.as_slice()).unwrap());

            TestResult::passed()
        }
    }
}
