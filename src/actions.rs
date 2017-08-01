struct Address;
struct U256;
struct Bytes32;
struct Bytes;
struct Signature;

/// Kovan Action
pub struct Deposit {
	/// funds owner
	recipient: Address,
	/// funds
	value: U256,
	/// hash of associated ethereum tx
	hash: Bytes32,
}

/// Kovan Action
pub struct CollectSignatures {
	/// keccak(message)
	signature: Signature,
	/// message to be relayed to main network
	message: Bytes,
}

/// Mainnet Action
pub struct Withdraw {
	/// keccak(recipient + value + hash)
	signatures: Vec<Signature>,
	/// withdraw address
	recipient: Address,
	/// withdraw value
	value: U256,
	/// hash of associated kovan tx
	hash: Bytes32,
}

/// All transaction types
pub enum Call {
	Deposit(Deposit),
	CollectSignatures(CollectSignatures),
	Withdraw(Withdraw),
}

pub struct Action {
	/// Call type
	call: Call,
	/// Transaction hash
	hash: Bytes32,
	/// Number of block which includes action, if empty transaction is unconfirmed
	receipt: Option<u64>,
}

pub struct DepositEvent {
	recipient: Address,
	value: U256,
}

fn deposit_to_call(event: DepositEvent, hash: Bytes32) -> Call {
	let deposit = Deposit {
		recipient: event.recipient,
		value: event.value,
		hash: hash,
	};
	Call::Deposit(deposit)
}

pub struct WithdrawEvent {
	recipient: Address,
	value: U256,
}

fn withdraw_to_call(event: WithdrawEvent, hash: Bytes32) -> Call {
	// TODO: event.recipient + event.value + hash
	let message = Bytes;
	let collect = CollectSignatures {
		// TODO: keccak(message),
		signature: Signature,
		message: message,
	};
	Call::CollectSignatures(collect)
}

pub struct CollectSignaturesEvent {
	signatures: Vec<Signature>,
	message: Bytes,
}

fn collect_to_call(event: CollectSignaturesEvent, _hash: Bytes32) -> Call {
	let withdraw = Withdraw {
		signatures: event.signatures,
		// TODO: event.message
		recipient: Address,
		// TODO: event.message
		value: U256,
		// TODO: event.message
		hash: Bytes32,
	};
	Call::Withdraw(withdraw)
}

pub struct Database {}

impl Database {
	fn find_all_unconfirmed_actions(&self) -> Vec<Action> {
		unimplemented!();
	}
}
