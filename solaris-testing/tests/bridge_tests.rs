extern crate ethabi;
#[macro_use]
extern crate ethabi_derive;
#[macro_use]
extern crate ethabi_contract;
extern crate ethereum_types as types;
extern crate rustc_hex;
extern crate solaris;
extern crate ethcore;
extern crate ethkey;
extern crate sha3;

use rustc_hex::FromHex;
use ethabi::Caller;
use types::{U256, H256, Address};
use ethkey::Generator;
use sha3::{Digest, Sha3_256};

use_contract!(foreign_bridge, "ForeignBridge", "contracts/bridge_sol_ForeignBridge.abi");

#[test]
fn should_allow_a_single_authority_to_confirm_a_deposit() {
	let contract = foreign_bridge::ForeignBridge::default();
	let code_hex = include_str!("../contracts/bridge_sol_ForeignBridge.bin");
	let code_bytes = code_hex.from_hex().unwrap();

	let mut evm = solaris::evm();

	let authority_addresses = vec![
		Address::from(10),
		Address::from(11),
	];

	let required_signatures: U256 = 1.into();

	let contract_owner_address: Address = 3.into();
	let user_address: Address = 1.into();

	let constructor_result = contract.constructor(
		code_bytes,
		required_signatures,
		authority_addresses.iter().cloned()
	);

	let transaction_hash: H256 = "0xe55bb43c36cdf79e23b4adc149cdded921f0d482e613c50c6540977c213bc408".into();
	let value: U256 = solaris::wei::from_ether(1);

	let _contract_address = evm
		.with_sender(contract_owner_address)
		.deploy(&constructor_result)
		.expect("contract deployment should succeed");

	let fns = contract.functions();

	assert_eq!(
		U256::from(0),
		U256::from(&*evm.call(fns.balances().input(user_address)).unwrap()),
		"initial balance should be 0"
	);

	evm
		.with_sender(authority_addresses[0].clone())
		.transact(fns.deposit().input(user_address, value, transaction_hash))
		.expect("the call to deposit should succeed");

	assert_eq!(
		evm.logs(None).len(),
		1,
		"exactly one event should be created");

	assert_eq!(
		evm.logs(contract.events().deposit().create_filter()).len(),
		1,
		"exactly one deposit event should be created");

	assert_eq!(
		evm.logs(contract.events().withdraw().create_filter()).len(),
		0,
		"no withdraw event should be created");

	let log = evm.logs(None).pop().expect("there must be at least 1 event");
	let deposit_log = contract.events().deposit().parse_log(log)
		.expect("the event should be a deposit event");
	assert_eq!(Address::from(deposit_log.recipient), user_address);
	assert_eq!(U256::from(deposit_log.value), value);

	assert_eq!(
		value,
		U256::from(&*evm.call(fns.balances().input(user_address)).unwrap()),
		"balance should have changed to `value`"
	);
}

// TODO [snd] better name
fn message_bytes_to_message(message_bytes: &[u8]) -> ethkey::Message {
	let mut hasher = Sha3_256::default();
	let prefix = "\u{0019}Ethereum Signed Message:\n";
	hasher.input(prefix.as_bytes());
	hasher.input(message_bytes.len().to_string().as_bytes());
	hasher.input(message_bytes);
	let result = hasher.result();
	(&*result).into()
}

fn sign(
	secret: &ethkey::Secret,
	message_bytes: &[u8]
) -> Result<ethkey::Signature, ethkey::Error> {
	ethkey::sign(secret, &message_bytes_to_message(message_bytes))
}

fn signature_to_bytes(signature: &ethkey::Signature) -> Vec<u8> {
	let signature: &[u8; 65] = &*signature;
	let mut result = Vec::new();
	result.extend_from_slice(signature);
	result
}

#[test]
fn should_successfully_submit_signature_and_trigger_collected_signatures_event() {
	let contract = foreign_bridge::ForeignBridge::default();
	let code_hex = include_str!("../contracts/bridge_sol_ForeignBridge.bin");
	let code_bytes = code_hex.from_hex().unwrap();

	let mut evm = solaris::evm();

	let authority_keypairs = vec![
		ethkey::Random.generate().unwrap(),
		ethkey::Random.generate().unwrap(),
	];

	let authority_addresses: Vec<Address> = authority_keypairs
		.iter()
		.map(|keypair| keypair.address().0.into())
		.collect();

	let required_signatures: U256 = 1.into();

	let message = "111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111".from_hex().unwrap();
	assert_eq!(message.len(), 84);

	let signature = sign(&authority_keypairs[0].secret(), &message).unwrap();
	println!("signature = {}", signature);

	let signature_bytes = signature_to_bytes(&signature);
	assert_eq!(signature_bytes.len(), 65);

	assert_eq!(
		ethkey::recover(&signature, &message_bytes_to_message(&message)).unwrap(),
		*authority_keypairs[0].public()
	);

	let constructor_result = contract.constructor(
		code_bytes,
		required_signatures,
		authority_addresses.iter().cloned()
	);

	let contract_owner_address: Address = 3.into();

	let _contract_address = evm
		.with_sender(contract_owner_address)
		.deploy(&constructor_result)
		.expect("contract deployment should succeed");

	let fns = contract.functions();

	evm
		.with_sender(authority_addresses[0].clone())
		.transact(fns.submit_signature().input(signature_bytes, message))
		.expect("the call to submit_signature should succeed");
}
