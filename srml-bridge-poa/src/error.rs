use sr_primitives::RuntimeDebug;

/// Header import error.
#[derive(RuntimeDebug)]
pub enum Error {
	/// The header is beyound last finalized and can not be imported.
	AncientHeader,
	/// The header is already imported.
	KnownHeader,
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
	/// Missing transaction receipts for the operation.
	MissingTransactionsReceipts,
}

impl Error {
	pub fn msg(&self) -> &'static str {
		match *self {
			Error::AncientHeader => "Header is beyound last finalized and can not be imported",
			Error::KnownHeader => "Header is already imported",
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
			Error::MissingTransactionsReceipts => "The import operation requires transactions receipts",
		}
	}
}
