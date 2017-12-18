extern crate ethabi;
#[macro_use]
extern crate ethabi_derive;
#[macro_use]
extern crate ethabi_contract;
extern crate ethereum_types as types;
extern crate rustc_hex;
extern crate solaris;
extern crate ethcore;

use rustc_hex::FromHex;
use solaris::unit;
use solaris::sol;
use ethabi::Caller;
use types::{U256, H256, Address};

use_contract!(foreign_bridge, "ForeignBridge", "contracts/bridge_sol_ForeignBridge.abi");

fn log_entry_to_raw_log(log_entry: &ethcore::log_entry::LogEntry) -> ethabi::RawLog {
	let topics: Vec<ethabi::Hash> = log_entry.topics.iter().map(|x| x.0).collect();
	ethabi::RawLog::from((topics, log_entry.data.clone()))
}

#[test]
fn should_allow_a_single_authority_to_confirm_a_deposit() {
	let contract = foreign_bridge::ForeignBridge::default();
	let code_hex = include_str!("../contracts/bridge_sol_ForeignBridge.bin");
	let code_bytes = code_hex.from_hex().unwrap();

	let mut evm = solaris::evm();

	let authority_addresses = vec![
		sol::address(10),
		sol::address(11),
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
	// TODO ether to wei
	let value: U256 = 1.into();

	let contract_address = evm
		.with_sender(contract_owner_address)
		.with_gas(4_000_000.into())
		.deploy(&constructor_result)
		.unwrap();

	let fns = contract.functions();

	let result = evm
		.with_sender(authority_addresses[0].clone())
		.with_gas(4_000_000.into())
		.transact(fns.deposit().input(user_address, value, transaction_hash));

	assert_eq!(
		evm.logs().len(),
		1,
		"exactly one event should be created");

	let log_entry = &evm.logs()[0];
	let raw_log = log_entry_to_raw_log(log_entry);
	let deposit_log = contract.events().deposit().parse_log(raw_log)
		.expect("the event should be a deposit event");
	assert_eq!(Address::from(deposit_log.recipient), user_address);
	assert_eq!(U256::from(deposit_log.value), value);
}
