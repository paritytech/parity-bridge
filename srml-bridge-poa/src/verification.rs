use runtime_io::secp256k1_ecdsa_recover;
use support::StorageMap;
use primitives::{Address, Header, H256, H520, SealedEmptyStep, U128, U256, keccak256};
use crate::{AuraParams, Headers, Trait};
use crate::validators::Validators;

/// Verification error.
pub enum Error {
	/// Seal has an incorrect format.
	InvalidSealArity,
	/// Block number isn't sensible.
	RidiculousNumber,
	/// Block has too much gas used.
	TooMuchGasUsed,
	/// Gas limit header field is invalid.
	InvalidGasLimit,
	/// Extra data is of an invalid length.
	ExtraDataOutOfBounds,
	/// Timestamp header overflowed.
	TimestampOverflow,
	/// The parent header is missing from the blockchain.
	MissingParentBlock,
	/// The header step is missing from the header.
	MissingStep,
	/// The header signature is missing from the header.
	MissingSignature,
	/// Empty steps are missing from the header.
	MissingEmptySteps,
	/// The same author issued different votes at the same step.
	DoubleVote,
	/// Validation proof insufficient.
	InsufficientProof,
	/// Difficulty header field is invalid.
	InvalidDifficulty,
	/// The received block is from an incorrect proposer.
	NotValidator,
}

impl Error {
	pub fn msg(&self) -> &'static str {
		match *self {
			Error::InvalidSealArity => "Header has an incorrect seal",
			Error::RidiculousNumber => "Header has too large number",
			Error::TooMuchGasUsed => "Header has too much gas used",
			Error::InvalidGasLimit => "Header has invalid gas limit",
			Error::ExtraDataOutOfBounds => "Header has too large extra data",
			Error::TimestampOverflow => "Header has too large timestamp",
			Error::MissingParentBlock => "Header has unknown parent hash",
			Error::MissingStep => "Header is missing step seal",
			Error::MissingSignature => "Header is missing signature seal",
			Error::MissingEmptySteps => "Header is missing empty steps seal",
			Error::DoubleVote => "Header has invalid step in seal",
			Error::InsufficientProof => "Header has insufficient proof",
			Error::InvalidDifficulty => "Header has invalid difficulty",
			Error::NotValidator => "Header is sealed by unexpected validator",
		}
	}
}

/// Verify header by Aura rules.
pub fn verify_aura_header<T: Trait>(
	params: &AuraParams,
	validators: &Validators,
	header: &Header,
) -> Result<(), Error> {
	// let's do the lightest check first
	contextless_checks(params, header)?;

	// the rest of heck requires parent
	let validators_set = validators.at(&header.parent_hash);
	let parent = Headers::get(&header.parent_hash).ok_or(Error::MissingParentBlock)?;
	let header_step = header.step().ok_or(Error::MissingStep)?;
	let parent_step = parent.step().ok_or(Error::MissingStep)?;

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

				if !verify_empty_step(&header.parent_hash, &empty_step, &validators_set) {
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

	let expected_validator = step_validator(&validators_set, header_step);
	if header.author != expected_validator {
		return Err(Error::NotValidator);
	}

	let validator_signature = header.signature().ok_or(Error::MissingSignature)?;
	let header_seal_hash = header.seal_hash(header.number >= params.empty_steps_transition);
	let is_invalid_proposer = !verify_signature(&expected_validator, &validator_signature, &header_seal_hash);
	if is_invalid_proposer {
		return Err(Error::NotValidator);
	}

	Ok(())
}

/// Perform basic checks that only require header iteself.
fn contextless_checks(params: &AuraParams, header: &Header) -> Result<(), Error> {
	let expected_seal_fields = expected_header_seal_fields(params, &header);
	if header.seal.len() != expected_seal_fields {
		return Err(Error::InvalidSealArity);
	}
	if header.number >= u64::max_value() {
		return Err(Error::RidiculousNumber);
	}
	if header.gas_used > header.gas_limit {
		return Err(Error::TooMuchGasUsed);
	}
	if header.gas_limit < params.min_gas_limit {
		return Err(Error::InvalidGasLimit);
	}
	if header.gas_limit > params.max_gas_limit {
		return Err(Error::InvalidGasLimit);
	}
	if header.number != 0 && header.extra_data.len() as u64 > params.maximum_extra_data_size {
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
fn expected_header_seal_fields(params: &AuraParams, header: &Header) -> usize {
	if header.number >= params.empty_steps_transition {
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

/// Returns expected validator at given step.
fn step_validator(validators: &[Address], step: u64) -> Address {
	validators[(step % validators.len() as u64) as usize]
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

/// Convert public key into correcponding ethereum address.
fn public_to_address(public: &[u8; 64]) -> Address {
	let hash = keccak256(public);
	let mut result = Address::zero();
	result.as_bytes_mut().copy_from_slice(&hash[12..]);
	result
}
