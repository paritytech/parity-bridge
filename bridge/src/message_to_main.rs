use ethereum_types::{Address, H256, U256};
use contracts::foreign::events::Withdraw;
use web3::types::Log;
use ethabi;
use error::Error;
use tiny_keccak;
use helpers;

/// the message that is relayed from side to main.
/// contains all the information required for the relay.
/// validators sign off on this message.
#[derive(PartialEq, Debug)]
pub struct MessageToMain {
    pub recipient: Address,
    pub value: U256,
    pub side_tx_hash: H256,
    pub main_gas_price: U256,
}

/// length of a `MessageToMain.to_bytes()` in bytes
pub const MESSAGE_LENGTH: usize = 116;

impl MessageToMain {
    /// parses message from a byte slice
    pub fn from_bytes(bytes: &[u8]) -> Self {
        assert_eq!(bytes.len(), MESSAGE_LENGTH);

        Self {
            recipient: bytes[0..20].into(),
            value: U256::from_big_endian(&bytes[20..52]),
            side_tx_hash: bytes[52..84].into(),
            main_gas_price: U256::from_big_endian(&bytes[84..MESSAGE_LENGTH]),
        }
    }

    pub fn keccak256(&self) -> H256 {
        tiny_keccak::keccak256(&self.to_bytes()).into()
    }

    /// construct a message from a `Withdraw` event that was logged on `foreign`
    pub fn from_log(raw_log: &Log) -> Result<Self, Error> {
        let hash = raw_log
            .transaction_hash
            .ok_or_else(|| "`log` must be mined and contain `transaction_hash`")?;
        let log = helpers::parse_log(&Withdraw::default(), raw_log)?;
        Ok(Self {
            recipient: log.recipient,
            value: log.value,
            side_tx_hash: hash,
            main_gas_price: log.home_gas_price,
        })
    }

    /// serializes message to a byte vector.
    /// mainly used to construct the message byte vector that is then signed
    /// and passed to `ForeignBridge.submitSignature`
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result = vec![0u8; MESSAGE_LENGTH];
        result[0..20].copy_from_slice(&self.recipient.0[..]);
        self.value.to_big_endian(&mut result[20..52]);
        result[52..84].copy_from_slice(&self.side_tx_hash.0[..]);
        self.main_gas_price
            .to_big_endian(&mut result[84..MESSAGE_LENGTH]);
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
    use rustc_hex::FromHex;

    #[test]
    fn test_message_to_main_to_bytes() {
        let recipient: Address = "0xeac4a655451e159313c3641e29824e77d6fcb0ce".into();
        let value = U256::from_dec_str("3800000000000000").unwrap();
        let side_tx_hash: H256 =
            "0x75ebc3036b5a5a758be9a8c0e6f6ed8d46c640dda39845de99d9570ba76798e2".into();
        let main_gas_price = U256::from_dec_str("8000000000").unwrap();

        let message = MessageToMain {
            recipient,
            value,
            side_tx_hash,
            main_gas_price,
        };

        assert_eq!(message.to_bytes(), "eac4a655451e159313c3641e29824e77d6fcb0ce000000000000000000000000000000000000000000000000000d80147225800075ebc3036b5a5a758be9a8c0e6f6ed8d46c640dda39845de99d9570ba76798e200000000000000000000000000000000000000000000000000000001dcd65000".from_hex().unwrap())
    }

    quickcheck! {
        fn quickcheck_message_to_main_roundtrips_to_bytes(
            recipient_raw: Vec<u8>,
            value_raw: u64,
            side_tx_hash_raw: Vec<u8>,
            main_gas_price_raw: u64
        ) -> TestResult {
            if recipient_raw.len() != 20 || side_tx_hash_raw.len() != 32 {
                return TestResult::discard();
            }

            let recipient: Address = recipient_raw.as_slice().into();
            let value: U256 = value_raw.into();
            let side_tx_hash: H256 = side_tx_hash_raw.as_slice().into();
            let main_gas_price: U256 = main_gas_price_raw.into();

            let message = MessageToMain {
                recipient,
                value,
                side_tx_hash,
                main_gas_price
            };

            let bytes = message.to_bytes();
            assert_eq!(message, MessageToMain::from_bytes(bytes.as_slice()));

            let payload = message.to_payload();
            let mut tokens = ethabi::decode(&[ethabi::ParamType::Bytes], payload.as_slice())
                .unwrap();
            let decoded = tokens.pop().unwrap().to_bytes().unwrap();
            assert_eq!(message, MessageToMain::from_bytes(decoded.as_slice()));

            TestResult::passed()
        }
    }
}
