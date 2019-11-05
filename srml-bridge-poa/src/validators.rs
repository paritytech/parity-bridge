// Copyright 2019 Parity Technologies (UK) Ltd.
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

use rstd::prelude::*;
use primitives::{Address, H256, Header, LogEntry, Receipt, U256};
use crate::Storage;
use crate::error::Error;

/// The hash of InitiateChange event of the validators set contract.
const CHANGE_EVENT_HASH: &'static [u8; 32] = &[0x55, 0x25, 0x2f, 0xa6, 0xee, 0xe4, 0x74, 0x1b,
	0x4e, 0x24, 0xa7, 0x4a, 0x70, 0xe9, 0xc1, 0x1f, 0xd2, 0xc2, 0x28, 0x1d, 0xf8, 0xd6, 0xea,
	0x13, 0x12, 0x6f, 0xf8, 0x45, 0xf7, 0x82, 0x5c, 0x89];

/// Where source of validators addresses come from. This covers the chain lifetime.
pub enum ValidatorsConfiguration {
	/// There's a single source for the whole chain lifetime.
	Single(ValidatorsSource),
	/// Validators source changes at given blocks. The blocks are ordered
	/// by the block number.
	Multi(Vec<(u64, ValidatorsSource)>),
}

/// Where validators addresses come from.
///
/// This source is valid within some blocks range. The blocks range could
/// cover multiple epochs - i.e. the validators that are authoring blocks
/// within this range could change, but the source itself can not.
pub enum ValidatorsSource {
	/// The validators addresses are hardcoded and never change.
	List(Vec<Address>),
	/// The validators addresses are determined by the validators set contract
	/// deployed at given address. The contract must implement the `ValidatorSet`
	/// interface. Additionally, the initial validators set must be provided.
	Contract(Address, Vec<Address>),
}

/// Validators manager.
pub struct Validators<'a> {
	config: &'a ValidatorsConfiguration,
}

impl<'a> Validators<'a> {
	/// Creates new validators manager using given configuration.
	pub fn new(initial_block_number: u64, config: &'a ValidatorsConfiguration) -> Self {
		if let ValidatorsConfiguration::Multi(ref sources) = *config {
			if sources.is_empty() || sources[0].0 > initial_block_number {
				panic!("Validators source for initial block is not provided");
			}
		}

		Self { config }
	}

	/// Returns true if header (probabilistically) signals validators change and
	/// the caller needs to provide transactions receipts to import the header.
	pub fn maybe_signals_validators_change(&self, header: &Header) -> bool {
		let (_, source) = self.source_at(header.number);

		// if we are taking validators set from the fixed list, there's always
		// single epoch
		// => we never require transactions receipts
		let contract_address = match source {
			ValidatorsSource::List(_) => return false,
			ValidatorsSource::Contract(contract_address, _) => contract_address,
		};

		// else we need to check logs bloom and if it has required bits set, it means
		// that the contract has (probably) emitted epoch change event
		let expected_bloom = LogEntry {
			address: *contract_address,
			topics: vec![
				CHANGE_EVENT_HASH.into(),
				header.parent_hash,
			],
			data: Vec::new(), // irrelevant for bloom.
		}.bloom();

		header.log_bloom.contains(&expected_bloom)
	}

	/// Extracts validators change signal from the header.
	///
	/// Returns tuple where first element is the change scheduled by this header 
	/// (i.e. this change is only applied starting from the block that has finalized
	/// current block). The second element is the immediately applied change.
	pub fn extract_validators_change(
		&self,
		header: &Header,
		receipts: Option<Vec<Receipt>>,
	) -> Result<(Option<Vec<Address>>, Option<Vec<Address>>), Error> {
		// TODO: verify receipts root!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!

		// let's first check if new source is starting from this header
		let (starts_at, source) = self.source_at(header.number + 1);
		if starts_at == header.number {
			match *source {
				ValidatorsSource::List(ref new_list) => return Ok((None, Some(new_list.clone()))),
				ValidatorsSource::Contract(_, ref new_list) => return Ok((Some(new_list.clone()), None)),
			}
		}

		// else deal with previous source
		let (_, source) = self.source_at(header.number);

		// if we are taking validators set from the fixed list, there's always
		// single epoch
		// => we never require transactions receipts
		let contract_address = match source {
			ValidatorsSource::List(_) => return Ok((None, None)),
			ValidatorsSource::Contract(contract_address, _) => contract_address,
		};

		// else we need to check logs bloom and if it has required bits set, it means
		// that the contract has (probably) emitted epoch change event
		let expected_bloom = LogEntry {
			address: *contract_address,
			topics: vec![
				CHANGE_EVENT_HASH.into(),
				header.parent_hash,
			],
			data: Vec::new(), // irrelevant for bloom.
		}.bloom();

		if !header.log_bloom.contains(&expected_bloom) {
			return Ok((None, None));
		}

		let receipts = receipts.ok_or(Error::MissingTransactionsReceipts)?;

		// iterate in reverse because only the _last_ change in a given
		// block actually has any effect
		Ok((receipts.iter()
			.rev()
			.filter(|r| r.log_bloom.contains(&expected_bloom))
			.flat_map(|r| r.logs.iter())
			.filter(|l| l.address == *contract_address &&
				l.topics.len() == 2 &&
				l.topics[0].as_fixed_bytes() == CHANGE_EVENT_HASH &&
				l.topics[1] == header.parent_hash
			)
			.filter_map(|l| {
				let data_len = l.data.len();
				if data_len < 64 {
					return None;
				}

				let new_validators_len_u256 = U256::from_big_endian(&l.data[32..64]);
				let new_validators_len = new_validators_len_u256.low_u64();
				if new_validators_len_u256 != new_validators_len.into() {
					return None;
				}

				if (data_len - 64) as u64 != new_validators_len.saturating_mul(32) {
					return None;
				}

				Some(l.data[64..]
					.chunks(32)
					.map(|chunk| {
						let mut new_validator = Address::default();
						new_validator.as_mut().copy_from_slice(&chunk[12..32]);
						new_validator
					})
					.collect())
			})
			.next(), None))
	}

	/// Finalize changes when blocks are finalized.
	pub fn finalize_validators_change<S: Storage>(&self, storage: &mut S, finalized_blocks: &[(u64, H256)]) -> Option<Vec<Address>> {
		for (_, finalized_hash) in finalized_blocks.iter().rev() {
			if let Some(changes) = storage.scheduled_change(finalized_hash) {
				return Some(changes);
			}
		}
		None
	}

	/// Returns source of validators that should author the header.
	fn source_at<'b>(&'b self, header_number: u64) -> (u64, &'b ValidatorsSource) {
		match self.config {
			ValidatorsConfiguration::Single(ref source) => (0, source),
			ValidatorsConfiguration::Multi(ref sources) =>
				sources.iter().rev()
					.find(|&(begin, _)| *begin < header_number)
					.map(|(begin, source)| (*begin, source))
					.expect("there's always entry for the initial block;\
						we do not touch any headers with number < initial block number; qed"),
		}
	}
}

impl ValidatorsSource {
	/// Returns initial validators set.
	pub fn initial_epoch_validators(&self) -> Vec<Address> {
		match self {
			ValidatorsSource::List(ref list) => list.clone(),
			ValidatorsSource::Contract(_, ref list) => list.clone(),
		}
	}
}

/// Get validator that should author the block at given step.
pub fn step_validator(header_validators: &[Address], header_step: u64) -> Address {
	header_validators[(header_step % header_validators.len() as u64) as usize]
}
