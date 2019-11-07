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

pub use primitive_types::{H160, H256, H512, U128, U256};
pub use tiny_keccak::keccak256;

#[cfg(feature = "test-helpers")]
pub use rlp::encode as rlp_encode;

use rstd::prelude::*;
use codec::{Decode, Encode};
use ethbloom::{Bloom as EthBloom, Input as BloomInput};
use parity_bytes::Bytes;
use rlp::{Decodable, DecoderError, Rlp, RlpStream};
use sr_primitives::RuntimeDebug;
use fixed_hash::construct_fixed_hash;

#[cfg(feature = "std")]
use serde::{Serialize, Deserialize};
#[cfg(feature = "std")]
use serde_big_array::big_array;
use impl_rlp::impl_fixed_hash_rlp;
use impl_serde::impl_fixed_hash_serde;

construct_fixed_hash! { pub struct H520(65); }
impl_fixed_hash_rlp!(H520, 65);
#[cfg(feature = "std")]
impl_fixed_hash_serde!(H520, 65);

/// An ethereum address.
pub type Address = H160;

/// An Aura header.
#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Default, Serialize, Deserialize))]
pub struct Header {
	/// Parent block hash.
	pub parent_hash: H256,
	/// Block timestamp.
	pub timestamp: u64,
	/// Block number.
	pub number: u64,
	/// Block author.
	pub author: Address,

	/// Transactions root.
	pub transactions_root: H256,
	/// Block uncles hash.
	pub uncles_hash: H256,
	/// Block extra data.
	pub extra_data: Bytes,

	/// State root.
	pub state_root: H256,
	/// Block receipts root.
	pub receipts_root: H256,
	/// Block bloom.
	pub log_bloom: Bloom,
	/// Gas used for contracts execution.
	pub gas_used: U256,
	/// Block gas limit.
	pub gas_limit: U256,

	/// Block difficulty.
	pub difficulty: U256,
	/// Vector of post-RLP-encoded fields.
	pub seal: Vec<Bytes>,
}

/// Information describing execution of a transaction.
#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug)]
pub struct Receipt {
	/// The total gas used in the block following execution of the transaction.
	pub gas_used: U256,
	/// The OR-wide combination of all logs' blooms for this transaction.
	pub log_bloom: Bloom,
	/// The logs stemming from this transaction.
	pub logs: Vec<LogEntry>,
	/// Transaction outcome.
	pub outcome: TransactionOutcome,
}

/// Transaction outcome store in the receipt.
#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug)]
pub enum TransactionOutcome {
	/// Status and state root are unknown under EIP-98 rules.
	Unknown,
	/// State root is known. Pre EIP-98 and EIP-658 rules.
	StateRoot(H256),
	/// Status code is known. EIP-658 rules.
	StatusCode(u8),
}

/// A record of execution for a `LOG` operation.
#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug)]
pub struct LogEntry {
	/// The address of the contract executing at the point of the `LOG` operation.
	pub address: Address,
	/// The topics associated with the `LOG` operation.
	pub topics: Vec<H256>,
	/// The data associated with the `LOG` operation.
	pub data: Bytes,
}

/// Logs bloom.
#[derive(Clone, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Bloom(
	#[cfg_attr(feature = "std", serde(with = "BigArray"))]
	[u8; 256]
);

#[cfg(feature = "std")]
big_array! { BigArray; }

/// An empty step message that is included in a seal, the only difference is that it doesn't include
/// the `parent_hash` in order to save space. The included signature is of the original empty step
/// message, which can be reconstructed by using the parent hash of the block in which this sealed
/// empty message is included.
pub struct SealedEmptyStep {
	/// Signature of the original message author.
	pub signature: H520,
	/// The step this message is generated for.
	pub step: u64,
}

impl Header {
	/// Get the hash of this header (keccak of the RLP with seal).
	pub fn hash(&self) -> H256 {
		keccak256(&self.rlp(true)).into()
	}

	/// Gets the seal hash of this header.
	pub fn seal_hash(&self, include_empty_steps: bool) -> Option<H256> {
		Some(match include_empty_steps {
			true => {
				let mut message = self.hash().as_bytes().to_vec();
				message.extend_from_slice(self.seal.get(2)?);
				keccak256(&message).into()
			},
			false => keccak256(&self.rlp(false)).into(),
		})
	}

	/// Get step this header is generated for.
	pub fn step(&self) -> Option<u64> {
		self.seal.get(0).map(|x| Rlp::new(&x)).and_then(|x| x.as_val().ok())
	}

	/// Get header author' signature.
	pub fn signature(&self) -> Option<H520> {
		self.seal.get(1).and_then(|x| Rlp::new(x).as_val().ok())
	}

	/// Extracts the empty steps from the header seal.
	pub fn empty_steps(&self) -> Option<Vec<SealedEmptyStep>> {
		self.seal.get(2).and_then(|x| Rlp::new(x).as_list::<SealedEmptyStep>().ok())
	}

	/// Returns header RLP with or without seals.
	fn rlp(&self, with_seal: bool) -> Bytes {
		let mut s = RlpStream::new();
		if with_seal {
			s.begin_list(13 + self.seal.len());
		} else {
			s.begin_list(13);
		}

		s.append(&self.parent_hash);
		s.append(&self.uncles_hash);
		s.append(&self.author);
		s.append(&self.state_root);
		s.append(&self.transactions_root);
		s.append(&self.receipts_root);
		s.append(&EthBloom::from(self.log_bloom.0));
		s.append(&self.difficulty);
		s.append(&self.number);
		s.append(&self.gas_limit);
		s.append(&self.gas_used);
		s.append(&self.timestamp);
		s.append(&self.extra_data);

		if with_seal {
			for b in &self.seal {
				s.append_raw(b, 1);
			}
		}

		s.out()
	}
}

impl SealedEmptyStep {
	/// Returns message that has to be signed by the validator.
	pub fn message(&self, parent_hash: &H256) -> H256 {
		let mut message = RlpStream::new_list(2);
		message.append(&self.step);
		message.append(parent_hash);
		keccak256(&message.out()).into()
	}

	/// Returns rlp for the vector of empty steps (we only do encoding in tests).
	#[cfg(feature = "test-helpers")]
	pub fn rlp_of(empty_steps: &[SealedEmptyStep]) -> Bytes {
		let mut s = RlpStream::new();
		s.begin_list(empty_steps.len());
		for empty_step in empty_steps {
			s.begin_list(2)
				.append(&empty_step.signature)
				.append(&empty_step.step);
		}
		s.out()
	}
}

impl Decodable for SealedEmptyStep {
	fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
		let signature: H520 = rlp.val_at(0)?;
		let step = rlp.val_at(1)?;

		Ok(SealedEmptyStep { signature, step })
	}
}

impl LogEntry {
	/// Calculates the bloom of this log entry.
	pub fn bloom(&self) -> Bloom {
		let eth_bloom = self.topics.iter().fold(EthBloom::from(BloomInput::Raw(self.address.as_bytes())), |mut b, t| {
			b.accrue(BloomInput::Raw(t.as_bytes()));
			b
		});
		Bloom(*eth_bloom.data())
	}
}

impl Bloom {
	/// Returns true if this bloom has all bits from the other set.
	pub fn contains(&self, other: &Bloom) -> bool {
		self.0.iter().zip(other.0.iter()).all(|(l, r)| (l & r) == *r)
	}
}

impl<'a> From<&'a [u8; 256]> for Bloom {
	fn from(buffer: &'a [u8; 256]) -> Bloom {
		Bloom(*buffer)
	}
}

impl PartialEq<Bloom> for Bloom {
	fn eq(&self, other: &Bloom) -> bool {
		self.0.iter().zip(other.0.iter()).all(|(l, r)| l == r)
	}
}

#[cfg(feature = "std")]
impl Default for Bloom {
	fn default() -> Self {
		Bloom([0; 256])
	}
}

#[cfg(feature = "std")]
impl std::fmt::Debug for Bloom {
	fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
		fmt.debug_struct("Bloom").finish()
	}
}

/// Convert public key into corresponding ethereum address.
pub fn public_to_address(public: &[u8; 64]) -> Address {
	let hash = keccak256(public);
	let mut result = Address::zero();
	result.as_bytes_mut().copy_from_slice(&hash[12..]);
	result
}
