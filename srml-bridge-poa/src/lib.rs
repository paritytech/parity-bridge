#![cfg_attr(not(feature = "std"), no_std)]

use rstd::prelude::*;
use codec::{Decode, Encode};
use support::{decl_module, decl_storage};
use sr_primitives::RuntimeDebug;
use primitives::{U256, H256, Header, Receipt};

mod validators;
mod verification;

/// Authority round engine parameters.
#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug)]
pub struct AuraParams {
	/// Empty step messages transition block.
	pub empty_steps_transition: u64,
	/// Transition block to strict empty steps validation.
	pub strict_empty_steps_transition: u64,
	/// Monotonic step validation transition block.
	pub validate_step_transition: u64,
	/// Chain score validation transition block.
	pub validate_score_transition: u64,
	/// Minimum gas limit.
	pub min_gas_limit: U256,
	/// Maximum gas limit.
	pub max_gas_limit: U256,
	/// Maximum size of extra data.
	pub maximum_extra_data_size: u64,
}

/// The module configuration trait
pub trait Trait: system::Trait {
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		/// Import Aura chain header.
		pub fn import_header(_origin, header: Header, _receipts: Option<Vec<Receipt>>) {
			let params = kovan_aura_params();
			let validators = validators::Validators;
			let hash = header.hash();

			verification::verify_aura_header::<T>(&params, &validators, &header)
				.map_err(|err| err.msg())?;

			Headers::insert(hash, header);
		}
	}
}

decl_storage! {
	trait Store for Module<T: Trait> as Bridge {
		/// Map of imported headers by hash.
		pub Headers get(headers): map H256 => Option<Header>;
	}
}

/// Aura engine parameters for Kovan chain.
fn kovan_aura_params() -> AuraParams {
	AuraParams {
		empty_steps_transition: u64::max_value(),
		strict_empty_steps_transition: 0,
		validate_step_transition: 0x16e360,
		validate_score_transition: 0x41a3c4,
		min_gas_limit: 0x1388.into(),
		max_gas_limit: U256::max_value(),
		maximum_extra_data_size: 0x20,
	}
}
