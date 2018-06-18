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
            r: bytes[0..32].into(),
            s: bytes[32..64].into(),
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

            let r: H256 = r_raw.as_slice().into();
            let s: H256 = s_raw.as_slice().into();
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
