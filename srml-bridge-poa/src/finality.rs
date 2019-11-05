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
use rstd::collections::{
	btree_map::{BTreeMap, Entry},
	btree_set::BTreeSet,
	vec_deque::VecDeque,
};
use runtime_io::secp256k1_ecdsa_recover;
use primitives::{Address, H256, Header, SealedEmptyStep, public_to_address};
use crate::{Storage, ancestry};
use crate::error::Error;

/// Tries to finalize blocks when given block is imported.
pub fn finalize_blocks<S: Storage>(
	storage: &S,
	header_validators: &(H256, Vec<Address>),
	hash: &H256,
	header: &Header,
	two_thirds_majority_transition: u64,
) -> Result<Vec<(u64, H256)>, Error> {
	fn add_signers(
		all_signers: &[Address],
		add_signers: &[Address],
		counts: &mut BTreeMap<Address, u64>,
	) -> Result<(), Error> {
		for signer in add_signers {
			if !all_signers.contains(signer) {
				return Err(Error::NotValidator);
			}

			*counts.entry(*signer).or_insert(0) += 1;
		}

		Ok(())
	}

	fn remove_signers(remove_signers: &[Address], counts: &mut BTreeMap<Address, u64>) {
		for signer in remove_signers {
			match counts.entry(*signer) {
				Entry::Occupied(mut entry) => {
					if *entry.get() <= 1 {
						entry.remove();
					} else {
						*entry.get_mut() -= 1;
					}
				},
				Entry::Vacant(_) => unreachable!("we only remove signers that have been added; qed"),
			}
		}
	}

	let is_finalized = |number: u64, all_signers_len: u64, signed_len: u64|
		(number < two_thirds_majority_transition && signed_len * 2 > all_signers_len) ||
		(number >= two_thirds_majority_transition && signed_len * 3 > all_signers_len * 2);

	let mut parent_empty_step_signers = empty_steps_signers(header);
	let ancestry = ancestry(storage, header)
		.map(|(hash, header)| {
			let header = header.header;
			let mut signers = vec![header.author];
			signers.extend(parent_empty_step_signers.drain(..));

			let empty_step_signers = empty_steps_signers(&header);
			let res = (hash, header.number, signers);
			parent_empty_step_signers = empty_step_signers;
			res
		})
		.take_while(|&(hash, _, _)| hash != header_validators.0); // TODO: should be updated on pruning???

	let mut sign_count = BTreeMap::new();
	let mut headers = VecDeque::new();
	for (hash, number, signers) in ancestry {
		add_signers(&header_validators.1, &signers, &mut sign_count)?;
		if is_finalized(number, header_validators.1.len() as u64, sign_count.len() as u64) {
			remove_signers(&signers, &mut sign_count);
			break;
		}

		headers.push_front((hash, number, signers));
	}

	if !header_validators.1.contains(&header.author) {
		return Err(Error::NotValidator);
	}

	*sign_count.entry(header.author).or_insert(0) += 1;
	headers.push_back((*hash, header.number, vec![header.author]));

	let mut newly_finalized = Vec::new();
	while let Some((oldest_hash, oldest_number, signers)) = headers.pop_front() {
		if !is_finalized(oldest_number, header_validators.1.len() as u64, sign_count.len() as u64) {
			break;
		}

		remove_signers(&signers, &mut sign_count);
		newly_finalized.push((oldest_number, oldest_hash));
	}

	Ok(newly_finalized)
}

/// Returns unique set of empty steps signers.
fn empty_steps_signers(header: &Header) -> Vec<Address> {
	header.empty_steps()
		.into_iter()
		.flat_map(|steps| steps)
		.filter_map(|step| empty_step_signer(&step, &header.parent_hash))
		.collect::<BTreeSet<_>>()
		.into_iter()
		.collect()
}

/// Returns author of empty step signature.
fn empty_step_signer(empty_step: &SealedEmptyStep, parent_hash: &H256) -> Option<Address> {
	let message = empty_step.message(parent_hash);
	secp256k1_ecdsa_recover(empty_step.signature.as_fixed_bytes(), message.as_fixed_bytes())
		.ok()
		.map(|public| public_to_address(&public))
}
