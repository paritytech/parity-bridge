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
use primitives::{H256, Header, Receipt, U256};
use crate::{AuraConfiguration, ImportedHeader, Storage};
use crate::error::Error;
use crate::finality::finalize_blocks;
use crate::validators::{Validators, ValidatorsConfiguration};
use crate::verification::verify_aura_header;

/// Number of headers behind the best block that we store. We never import
/// any headers that have block.parent.number < best_block.number - PRUNE_DEPTH.
///
/// I.e. if the best block is 100_000 and we try to import alternative block with
/// number = 100_000 - 2_048 = 97_952, the import will succeed.
/// But if we try to import alternative block with number = 100_000 - 2_048 + 1 = 97_953,
/// the import will fail.
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
	let (hash, best_total_difficulty) = is_importable_header(storage, &header)?;

	// verify and insert header
	let validators = Validators::new(storage.initial_block(), validators_config);
	let parent_header = verify_aura_header(
		storage,
		aura_config,
		//&validators,
		&header,
	)?;

	// check if block schedules new validators
	let (scheduled_validators, immediately_enacted_validators) = validators.extract_validators_change(&header, receipts)?;

	// check if block finalizes some other blocks and corresponding scheduled validators
	let finalized_blocks = finalize_blocks(
		storage,
		&parent_header.next_validators,
		&hash,
		&header,
		aura_config.two_thirds_majority_transition,
	)?;
	let enacted_validators = immediately_enacted_validators
		.or_else(|| validators.finalize_validators_change(storage, &finalized_blocks));

	// and finally insert the block
	let total_difficulty = parent_header.total_difficulty + header.difficulty;
	let is_best = total_difficulty > best_total_difficulty;
	let next_validators = enacted_validators
		.map(|enacted_validators| (hash, enacted_validators))
		.unwrap_or_else(|| parent_header.next_validators.clone());
	let imported_header = ImportedHeader {
		header,
		total_difficulty,
		next_validators,
	};
	storage.insert_header(is_best, hash, imported_header, scheduled_validators);

	// TODO: prune

	Ok(hash)
}

/// Returns true if transactions receipts are required to import given header.
pub fn header_import_requires_receipts<S: Storage>(
	storage: &S,
	validators_config: &ValidatorsConfiguration,
	header: &Header,
) -> bool {
	is_importable_header(storage, header)
		.map(|_| Validators::new(storage.initial_block(), validators_config))
		.map(|validators| validators.maybe_signals_validators_change(header))
		.unwrap_or(false)
}


/// Checks that we are able to ***try to** import this header.
/// Returns error if we should not try to import this block.
/// Returns the hash of the header and total difficulty of the best known block otherwise.
fn is_importable_header<S: Storage>(storage: &S, header: &Header) -> Result<(H256, U256), Error> {
	// we never import any header that is beyound prune depth
	let (best_block_number, _, best_total_difficulty) = storage.best_block();
	if header.number < best_block_number.saturating_sub(PRUNE_DEPTH) {
		return Err(Error::AncientHeader);
	}
	// we never import any header that competes with initial header
	if header.number <= storage.initial_block() {
		return Err(Error::AncientHeader);
	}
	// we never import any header with known hash
	let hash = header.hash();
	if storage.header(&hash).is_some() {
		return Err(Error::KnownHeader);
	}

	Ok((hash, best_total_difficulty))
}
