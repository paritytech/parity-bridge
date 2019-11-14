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
use primitives::{H256, Header, Receipt};
use crate::{AuraConfiguration, Storage};
use crate::error::Error;
use crate::finality::finalize_blocks;
use crate::validators::{Validators, ValidatorsConfiguration};
use crate::verification::verify_aura_header;

/// Number of headers behind the best finalized block that we store.
const PRUNE_DEPTH: u64 = 2048;

/// Imports given header and updates blocks finality (if required).
///
/// Transactions receipts must be provided if `header_import_requires_receipts()`
/// has returned true.
pub fn import_header<S: Storage>(
	storage: &mut S,
	aura_config: &AuraConfiguration,
	validators_config: &ValidatorsConfiguration,
	header: Header,
	receipts: Option<Vec<Receipt>>,
) -> Result<H256, Error> {
	// first check that we are able to import this header at all
	let hash = is_importable_header(storage, &header)?;

	// verify header
	let import_context = verify_aura_header(
		storage,
		aura_config,
		&header,
	)?;

	// check if block schedules new validators
	let validators = Validators::new(validators_config);
	let (scheduled_change, enacted_change) =
		validators.extract_validators_change(&header, receipts)?;

	// check if block finalizes some other blocks and corresponding scheduled validators
	let (prev_finalized_number, prev_finalized_hash) = storage.finalized_block();
	let finalized_blocks = finalize_blocks(
		storage,
		&prev_finalized_hash,
		(import_context.validators_start(), import_context.validators()),
		&hash,
		&header,
		aura_config.two_thirds_majority_transition,
	)?;
	let enacted_change = enacted_change
		.or_else(|| validators.finalize_validators_change(storage, &finalized_blocks));

	// and finally insert the block
	let (_, _, best_total_difficulty) = storage.best_block();
	let total_difficulty = import_context.total_difficulty() + header.difficulty;
	let is_best = total_difficulty > best_total_difficulty;
	storage.insert_header(import_context.into_import_header(
		is_best,
		hash,
		header,
		total_difficulty,
		enacted_change,
		scheduled_change,
	));

	// now prune old headers.
	// the pruning strategy is to store all unfinalized blocks and blocks
	// within PRUNE_DEPTH range before finalized blocks
	let last_finalized = finalized_blocks.last().cloned();
	if let Some((last_finalized_number, last_finalized_hash)) = last_finalized {
		let first_block_to_prune = prev_finalized_number.saturating_sub(PRUNE_DEPTH);
		let last_block_to_prune = last_finalized_number.saturating_sub(PRUNE_DEPTH);
		storage.prune_headers(first_block_to_prune, last_block_to_prune + 1);
		storage.set_finalized_block(last_finalized_number, last_finalized_hash);
	}

	Ok(hash)
}

/// Returns true if transactions receipts are required to import given header.
pub fn header_import_requires_receipts<S: Storage>(
	storage: &S,
	validators_config: &ValidatorsConfiguration,
	header: &Header,
) -> bool {
	is_importable_header(storage, header)
		.map(|_| Validators::new(validators_config))
		.map(|validators| validators.maybe_signals_validators_change(header))
		.unwrap_or(false)
}


/// Checks that we are able to ***try to** import this header.
/// Returns error if we should not try to import this block.
/// Returns the hash of the header and total difficulty of the best known block otherwise.
fn is_importable_header<S: Storage>(storage: &S, header: &Header) -> Result<H256, Error> {
	// we never import any header that competes with finalized header
	let (finalized_block_number, _) = storage.finalized_block();
	if header.number <= finalized_block_number {
		return Err(Error::AncientHeader);
	}
	// we never import any header with known hash
	let hash = header.hash();
	if storage.header(&hash).is_some() {
		return Err(Error::KnownHeader);
	}

	Ok(hash)
}

#[cfg(test)]
mod tests {
	use crate::{kovan_aura_config, kovan_validators_config};
	use crate::tests::{InMemoryStorage, block1, genesis, validator, validators_addresses};
	use crate::validators::ValidatorsSource;
	use super::*;

	#[test]
	fn rejects_finalized_block_competitors() {
		let mut storage = InMemoryStorage::new(genesis(), validators_addresses(3));
		storage.set_finalized_block(100, Default::default());
		assert_eq!(
			import_header(&mut storage, &kovan_aura_config(), &kovan_validators_config(), Default::default(), None),
			Err(Error::AncientHeader),
		);
	}

	#[test]
	fn rejects_known_header() {
		let validators = (0..3).map(|i| validator(i as u8)).collect::<Vec<_>>();
		let mut storage = InMemoryStorage::new(genesis(), validators_addresses(3));
		assert_eq!(
			import_header(&mut storage, &kovan_aura_config(), &kovan_validators_config(), block1(&validators), None)
				.map(|_| ()),
			Ok(()),
		);
		assert_eq!(
			import_header(&mut storage, &kovan_aura_config(), &kovan_validators_config(), block1(&validators), None),
			Err(Error::KnownHeader),
		);
	}

	#[test]
	fn import_header_works() {
		let validators_config = ValidatorsConfiguration::Multi(vec![
			(0, ValidatorsSource::List(validators_addresses(3))),
			(1, ValidatorsSource::List(validators_addresses(2))),
		]);
		let validators = (0..3).map(|i| validator(i as u8)).collect::<Vec<_>>();
		let mut storage = InMemoryStorage::new(genesis(), validators_addresses(3));
		let header = block1(&validators);
		let hash = header.hash();
		assert_eq!(
			import_header(&mut storage, &kovan_aura_config(), &validators_config, header, None)
				.map(|_| ()),
			Ok(()),
		);

		// check that new validators will be used for next header
		let imported_header = storage.stored_header(&hash).unwrap();
		assert_eq!(
			imported_header.next_validators_set_id,
			1, // new set is enacted from config
		);
	}
}
