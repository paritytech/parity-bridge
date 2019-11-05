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

use runtime_io::secp256k1_ecdsa_recover;
use primitives::{Address, Header, H256, H520, SealedEmptyStep, U128, U256, public_to_address};
use crate::{AuraConfiguration, ImportedHeader, Storage};
use crate::error::Error;
use crate::validators::{/*Validators, */step_validator};

/// Verify header by Aura rules.
pub fn verify_aura_header<S: Storage>(
	storage: &S,
	params: &AuraConfiguration,
//	validators: &Validators,
	header: &Header,
) -> Result<ImportedHeader, Error> {
	// let's do the lightest check first
	contextless_checks(params, header)?;

	// the rest of heck requires parent
	let parent = storage.header(&header.parent_hash).ok_or(Error::MissingParentBlock)?;
	let epoch_validators = &parent.next_validators.1;
	let header_step = header.step().ok_or(Error::MissingStep)?;
	let parent_step = parent.header.step().ok_or(Error::MissingStep)?;

	// Ensure header is from the step after parent.
	if header_step == parent_step
		|| (header.number >= params.validate_step_transition && header_step <= parent_step) {
		return Err(Error::DoubleVote);
	}

	// If empty step messages are enabled we will validate the messages in the seal, missing messages are not
	// reported as there's no way to tell whether the empty step message was never sent or simply not included.
	let empty_steps_len = match header.number >= params.empty_steps_transition {
		true => {
			let strict_empty_steps = header.number >= params.strict_empty_steps_transition;
			let empty_steps = header.empty_steps().ok_or(Error::MissingEmptySteps)?;
			let empty_steps_len = empty_steps.len();
			let mut prev_empty_step = 0;

			for empty_step in empty_steps {
				if empty_step.step <= parent_step || empty_step.step >= header_step {
					return Err(Error::InsufficientProof);
				}

				if !verify_empty_step(&header.parent_hash, &empty_step, &epoch_validators) {
					return Err(Error::InsufficientProof);
				}

				if strict_empty_steps {
					if empty_step.step <= prev_empty_step {
						return Err(Error::InsufficientProof);
					}

					prev_empty_step = empty_step.step;
				}
			}

			empty_steps_len
		},
		false => 0,
	};

	// Validate chain score.
	if header.number >= params.validate_score_transition {
		let expected_difficulty = calculate_score(parent_step.into(), header_step.into(), empty_steps_len.into());
		if header.difficulty != expected_difficulty {
			return Err(Error::InvalidDifficulty);
		}
	}

	let expected_validator = step_validator(&epoch_validators, header_step);
	if header.author != expected_validator {
		return Err(Error::NotValidator);
	}

	let validator_signature = header.signature().ok_or(Error::MissingSignature)?;
	let header_seal_hash = header.seal_hash(header.number >= params.empty_steps_transition)
		.ok_or(Error::MissingEmptySteps)?;
	let is_invalid_proposer = !verify_signature(&expected_validator, &validator_signature, &header_seal_hash);
	if is_invalid_proposer {
		return Err(Error::NotValidator);
	}

	Ok(parent)
}

/// Perform basic checks that only require header iteself.
fn contextless_checks(config: &AuraConfiguration, header: &Header) -> Result<(), Error> {
	let expected_seal_fields = expected_header_seal_fields(config, header);
	if header.seal.len() != expected_seal_fields {
		return Err(Error::InvalidSealArity);
	}
	if header.number >= u64::max_value() {
		return Err(Error::RidiculousNumber);
	}
	if header.gas_used > header.gas_limit {
		return Err(Error::TooMuchGasUsed);
	}
	if header.gas_limit < config.min_gas_limit {
		return Err(Error::InvalidGasLimit);
	}
	if header.gas_limit > config.max_gas_limit {
		return Err(Error::InvalidGasLimit);
	}
	if header.number != 0 && header.extra_data.len() as u64 > config.maximum_extra_data_size {
		return Err(Error::ExtraDataOutOfBounds);
	}

	// we can't detect if block is from future in runtime
	// => let's only do an overflow check
	if header.timestamp > i32::max_value() as u64 {
		return Err(Error::TimestampOverflow);
	}

	Ok(())
}

/// Returns expected number of seal fields in the header.
fn expected_header_seal_fields(config: &AuraConfiguration, header: &Header) -> usize {
	if header.number >= config.empty_steps_transition {
		3
	} else {
		2
	}
}

/// Verify single sealed empty step.
fn verify_empty_step(parent_hash: &H256, step: &SealedEmptyStep, validators: &[Address]) -> bool {
	let expected_validator = step_validator(validators, step.step);
	let message = step.message(parent_hash);
	verify_signature(&expected_validator, &step.signature, &message)
}

/// Chain scoring: total weight is sqrt(U256::max_value())*height - step
fn calculate_score(parent_step: u64, current_step: u64, current_empty_steps: usize) -> U256 {
	U256::from(U128::max_value()) + U256::from(parent_step) - U256::from(current_step) + U256::from(current_empty_steps)
}

/// Verify that the signature over message has been produced by given validator.
fn verify_signature(expected_validator: &Address, signature: &H520, message: &H256) -> bool {
	secp256k1_ecdsa_recover(signature.as_fixed_bytes(), message.as_fixed_bytes())
		.map(|public| public_to_address(&public))
		.map(|address| *expected_validator == address)
		.unwrap_or(false)
}
