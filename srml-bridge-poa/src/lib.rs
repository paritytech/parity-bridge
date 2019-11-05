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

#![cfg_attr(not(feature = "std"), no_std)]

use rstd::{prelude::*, iter::from_fn};
use codec::{Decode, Encode};
use support::{decl_module, decl_storage};
use sr_primitives::RuntimeDebug;
use primitives::{Address, U256, H256, Header, Receipt};
use validators::{ValidatorsSource, ValidatorsConfiguration};

pub use import::{import_header, header_import_requires_receipts};

mod error;
mod finality;
mod import;
mod validators;
mod verification;

/// Authority round engine configuration parameters.
#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug)]
pub struct AuraConfiguration {
	/// Empty step messages transition block.
	pub empty_steps_transition: u64,
	/// Transition block to strict empty steps validation.
	pub strict_empty_steps_transition: u64,
	/// Monotonic step validation transition block.
	pub validate_step_transition: u64,
	/// Chain score validation transition block.
	pub validate_score_transition: u64,
	/// First block for which a 2/3 quorum (instead of 1/2) is required.
	pub two_thirds_majority_transition: u64,
	/// Minimum gas limit.
	pub min_gas_limit: U256,
	/// Maximum gas limit.
	pub max_gas_limit: U256,
	/// Maximum size of extra data.
	pub maximum_extra_data_size: u64,
}

/// Imported block header.
#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug)]
pub struct ImportedHeader {
	/// The block header itself.
	pub header: Header,
	/// Total difficulty of the chain.
	pub total_difficulty: U256,
	// only TODO store ID here
	/// The set of validators that is expected to produce direct descendants of
	/// this block. If header enacts new set, this would be the new set. Otherwise
	/// this is the set that has produced the block itself.
	/// The hash is the hash of block where validators set has been enacted.
	pub next_validators: (H256, Vec<Address>),
}

/// The storage that is used by the client.
///
/// Storage modification must be discarded if block import has failed.
pub trait Storage {
	/// Get number of initial (genesis or checkpoint) block.
	fn initial_block(&self) -> u64;
	/// Get best known block.
	fn best_block(&self) -> (u64, H256, U256);
	/// Get imported header by its hash.
	fn header(&self, hash: &H256) -> Option<ImportedHeader>;
	/// Get new validators that are scheduled by given header.
	fn scheduled_change(&self, hash: &H256) -> Option<Vec<Address>>;
	/// Insert imported header.
	fn insert_header(
		&mut self,
		is_best: bool,
		hash: H256,
		header: ImportedHeader,
		scheduled_change: Option<Vec<Address>>,
	);
}

/// The module configuration trait
pub trait Trait: system::Trait {
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		/// Import Aura chain header.
		pub fn import_header(_origin, header: Header, receipts: Option<Vec<Receipt>>) {
			import::import_header(
				&mut BridgeStorage,
				&kovan_aura_config(),
				&kovan_validators_config(),
				header,
				receipts,
			).map_err(|e| e.msg())?;
		}
	}
}

decl_storage! {
	trait Store for Module<T: Trait> as Bridge {
		/// Initial block (genesis or block from checkpoint).
		InitialBlock: (u64, H256);
		/// Best known block.
		BestBlock: (u64, H256, U256);
		/// Map of imported headers by hash.
		Headers: map H256 => Option<ImportedHeader>;
		/// Map of validators set changes scheduled by given header.
		ScheduledChanges: map H256 => Option<Vec<Address>>;
	}
	add_extra_genesis {
		config(initial_header): Header;
		config(initial_difficulty): U256;
		config(initial_validators): Vec<Address>;
		build(|config| {
			// the initial blocks should be selected so that:
			// 1) it doesn't signal validators changes;
			// 2) there are no scheduled validators changes from previous blocks;
			// 3) (implied) all direct children of initial block are authred by the same validators set.

			// TODO: ensure that !initial_validators.is_empty()

			let initial_hash = config.initial_header.hash();
			InitialBlock::put((config.initial_header.number, initial_hash));
			BestBlock::put((config.initial_header.number, initial_hash, config.initial_difficulty));
			Headers::insert(initial_hash, ImportedHeader {
				header: config.initial_header.clone(),
				total_difficulty: config.initial_difficulty,
				next_validators: (initial_hash, config.initial_validators.clone()),
			});
		})
	}
}

impl<T: Trait> Module<T> {
	/// Returns true if the import of given block requires transactions receipts.
	pub fn is_import_requires_receipts(header: Header) -> bool {
		import::header_import_requires_receipts(
			&BridgeStorage,
			&kovan_validators_config(),
			&header,
		)
	}
}

/// Runtime bridge storage.
struct BridgeStorage;

impl Storage for BridgeStorage {
	fn initial_block(&self) -> u64 {
		InitialBlock::get().0
	}

	fn best_block(&self) -> (u64, H256, U256) {
		BestBlock::get()
	}

	fn header(&self, hash: &H256) -> Option<ImportedHeader> {
		Headers::get(hash)
	}

	fn scheduled_change(&self, hash: &H256) -> Option<Vec<Address>> {
		ScheduledChanges::get(hash)
	}

	fn insert_header(
		&mut self,
		is_best: bool,
		hash: H256,
		header: ImportedHeader,
		scheduled_change: Option<Vec<Address>>,
	) {
		if is_best {
			BestBlock::put((header.header.number, hash, header.total_difficulty));
		}
		Headers::insert(&hash, header);
		if let Some(scheduled_change) = scheduled_change {
			ScheduledChanges::insert(&hash, scheduled_change);
		}
	}
}

/// Aura engine configuration for Kovan chain.
pub fn kovan_aura_config() -> AuraConfiguration {
	AuraConfiguration {
		empty_steps_transition: u64::max_value(),
		strict_empty_steps_transition: 0,
		validate_step_transition: 0x16e360,
		validate_score_transition: 0x41a3c4,
		two_thirds_majority_transition: u64::max_value(),
		min_gas_limit: 0x1388.into(),
		max_gas_limit: U256::max_value(),
		maximum_extra_data_size: 0x20,
	}
}

/// Validators configuration for Kovan chain.
pub fn kovan_validators_config() -> ValidatorsConfiguration {
	ValidatorsConfiguration::Multi(vec![
		(0, ValidatorsSource::List(vec![
			[0x00, 0xD6, 0xCc, 0x1B, 0xA9, 0xcf, 0x89, 0xBD, 0x2e, 0x58,
				0x00, 0x97, 0x41, 0xf4, 0xF7, 0x32, 0x5B, 0xAd, 0xc0, 0xED].into(),
			[0x00, 0x42, 0x7f, 0xea, 0xe2, 0x41, 0x9c, 0x15, 0xb8, 0x9d,
				0x1c, 0x21, 0xaf, 0x10, 0xd1, 0xb6, 0x65, 0x0a, 0x4d, 0x3d].into(),
			[0x4E, 0xd9, 0xB0, 0x8e, 0x63, 0x54, 0xC7, 0x0f, 0xE6, 0xF8,
				0xCB, 0x04, 0x11, 0xb0, 0xd3, 0x24, 0x6b, 0x42, 0x4d, 0x6c].into(),
			[0x00, 0x20, 0xee, 0x4B, 0xe0, 0xe2, 0x02, 0x7d, 0x76, 0x60,
				0x3c, 0xB7, 0x51, 0xeE, 0x06, 0x95, 0x19, 0xbA, 0x81, 0xA1].into(),
			[0x00, 0x10, 0xf9, 0x4b, 0x29, 0x6a, 0x85, 0x2a, 0xaa, 0xc5,
				0x2e, 0xa6, 0xc5, 0xac, 0x72, 0xe0, 0x3a, 0xfd, 0x03, 0x2d].into(),
			[0x00, 0x77, 0x33, 0xa1, 0xFE, 0x69, 0xCF, 0x3f, 0x2C, 0xF9,
				0x89, 0xF8, 0x1C, 0x7b, 0x4c, 0xAc, 0x16, 0x93, 0x38, 0x7A].into(),
			[0x00, 0xE6, 0xd2, 0xb9, 0x31, 0xF5, 0x5a, 0x3f, 0x17, 0x01,
				0xc7, 0x38, 0x9d, 0x59, 0x2a, 0x77, 0x78, 0x89, 0x78, 0x79].into(),
			[0x00, 0xe4, 0xa1, 0x06, 0x50, 0xe5, 0xa6, 0xD6, 0x00, 0x1C,
				0x38, 0xff, 0x8E, 0x64, 0xF9, 0x70, 0x16, 0xa1, 0x64, 0x5c].into(),
			[0x00, 0xa0, 0xa2, 0x4b, 0x9f, 0x0e, 0x5e, 0xc7, 0xaa, 0x4c,
				0x73, 0x89, 0xb8, 0x30, 0x2f, 0xd0, 0x12, 0x31, 0x94, 0xde].into(),
		])),
		(10960440, ValidatorsSource::List(vec![
			[0x00, 0xD6, 0xCc, 0x1B, 0xA9, 0xcf, 0x89, 0xBD, 0x2e, 0x58,
				0x00, 0x97, 0x41, 0xf4, 0xF7, 0x32, 0x5B, 0xAd, 0xc0, 0xED].into(),
			[0x00, 0x10, 0xf9, 0x4b, 0x29, 0x6a, 0x85, 0x2a, 0xaa, 0xc5,
				0x2e, 0xa6, 0xc5, 0xac, 0x72, 0xe0, 0x3a, 0xfd, 0x03, 0x2d].into(),
			[0x00, 0xa0, 0xa2, 0x4b, 0x9f, 0x0e, 0x5e, 0xc7, 0xaa, 0x4c,
				0x73, 0x89, 0xb8, 0x30, 0x2f, 0xd0, 0x12, 0x31, 0x94, 0xde].into(),
		])),
		(10960500, ValidatorsSource::Contract(
			[0xaE, 0x71, 0x80, 0x7C, 0x1B, 0x0a, 0x09, 0x3c, 0xB1, 0x54,
				0x7b, 0x68, 0x2D, 0xC7, 0x83, 0x16, 0xD9, 0x45, 0xc9, 0xB8].into(),
			vec![
				[0xd0, 0x5f, 0x74, 0x78, 0xc6, 0xaa, 0x10, 0x78, 0x12, 0x58,
					0xc5, 0xcc, 0x8b, 0x4f, 0x38, 0x5f, 0xc8, 0xfa, 0x98, 0x9c].into(),
				[0x03, 0x80, 0x1e, 0xfb, 0x0e, 0xfe, 0x2a, 0x25, 0xed, 0xe5,
					0xdd, 0x3a, 0x00, 0x3a, 0xe8, 0x80, 0xc0, 0x29, 0x2e, 0x4d].into(),
				[0xa4, 0xdf, 0x25, 0x5e, 0xcf, 0x08, 0xbb, 0xf2, 0xc2, 0x80,
					0x55, 0xc6, 0x52, 0x25, 0xc9, 0xa9, 0x84, 0x7a, 0xbd, 0x94].into(),
				[0x59, 0x6e, 0x82, 0x21, 0xa3, 0x0b, 0xfe, 0x6e, 0x7e, 0xff,
					0x67, 0xfe, 0xe6, 0x64, 0xa0, 0x1c, 0x73, 0xba, 0x3c, 0x56].into(),
				[0xfa, 0xad, 0xfa, 0xce, 0x3f, 0xbd, 0x81, 0xce, 0x37, 0xb0,
					0xe1, 0x9c, 0x0b, 0x65, 0xff, 0x42, 0x34, 0x14, 0x81, 0x32].into(),
			],
		)),
	])
}

/// Return iterator of given header ancestors.
pub(crate) fn ancestry<'a, S: Storage>(storage: &'a S, header: &Header) -> impl Iterator<Item = (H256, ImportedHeader)> + 'a {
	let mut parent_hash = header.parent_hash.clone();
	from_fn(move || {
		let header = storage.header(&parent_hash);
		match header {
			Some(header) => {
				if header.header.number == 0 {
					return None;
				}

				let hash = parent_hash.clone();
				parent_hash = header.header.parent_hash.clone();
				Some((hash, header))
			},
			None => None
		}
	})
}

#[cfg(test)]
pub(crate) mod tests {
	use std::collections::HashMap;
	use super::*;

	pub struct InMemoryStorage {
		initial_block: u64,
		best_block: (u64, H256, U256),
		headers: HashMap<H256, ImportedHeader>,
		scheduled_changes: HashMap<H256, Vec<Address>>,
	}

	impl InMemoryStorage {
		pub fn new(initial_header: Header, initial_validators: Vec<Address>) -> Self {
			let hash = initial_header.hash();
			InMemoryStorage {
				initial_block: initial_header.number,
				best_block: (initial_header.number, hash, 0.into()),
				headers: vec![(
					hash,
					ImportedHeader {
						header: initial_header,
						total_difficulty: 0.into(),
						next_validators: (hash, initial_validators),
					},
				)].into_iter().collect(),
				scheduled_changes: HashMap::new(),
			}
		}
	}

	impl Storage for InMemoryStorage {
		fn initial_block(&self) -> u64 {
			self.initial_block
		}

		fn best_block(&self) -> (u64, H256, U256) {
			self.best_block.clone()
		}

		fn header(&self, hash: &H256) -> Option<ImportedHeader> {
			self.headers.get(hash).cloned()
		}

		fn scheduled_change(&self, hash: &H256) -> Option<Vec<Address>> {
			self.scheduled_changes.get(hash).cloned()
		}

		fn insert_header(
			&mut self,
			is_best: bool,
			hash: H256,
			header: ImportedHeader,
			scheduled_change: Option<Vec<Address>>,
		) {
			if is_best {
				self.best_block = (header.header.number, hash, header.total_difficulty);
			}
			self.headers.insert(hash, header);
			if let Some(scheduled_change) = scheduled_change {
				self.scheduled_changes.insert(hash, scheduled_change);
			}

			/*if self.headers.len() > 2048 {
				self.headers.pop_front();
			}*/
		}
	}
}
