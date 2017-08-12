use web3::types::{Filter, FilterBuilder, Address, TransactionRequest, U256, H256, H160, Bytes, BlockNumber, Log};
use ethabi::{Contract, Token};
use error::{Error, ResultExt};
use contracts::{EthereumDeposit};

pub struct EthereumBridge<'a>(pub &'a Contract);

impl<'a> EthereumBridge<'a> {
	pub fn deposits_filter(&self, address: Address) -> FilterBuilder {
		let event = self.0.event("Deposit").expect("to find event `Deposit`");
		FilterBuilder::default()
			.address(vec![address])
			.topics(Some(vec![H256(event.signature())]), None, None, None)
	}

	pub fn deposit_from_log(&self, log: Log) -> Result<EthereumDeposit, Error> {
		let event = self.0.event("Deposit").expect("to find event `Deposit`");
		let mut decoded = event.decode_log(
			log.topics.into_iter().map(|t| t.0).collect(),
			log.data.0
		)?;

		if decoded.len() != 2 {
			return Err("Invalid len of decoded deposit event".into())
		}

		let value = decoded.pop().and_then(|v| v.value.to_uint()).map(U256).chain_err(|| "expected uint")?;
		let recipient = decoded.pop().and_then(|v| v.value.to_address()).map(H160).chain_err(|| "expected address")?;

		let result = EthereumDeposit {
			recipient,
			value,
			hash: log.transaction_hash.expect("hash to exist"),
		};

		Ok(result)
	}
}
