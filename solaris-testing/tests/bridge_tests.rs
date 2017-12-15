extern crate ethabi;
#[macro_use]
extern crate ethabi_derive;
#[macro_use]
extern crate ethabi_contract;
extern crate ethereum_types as types;
extern crate rustc_hex;
extern crate solaris;

use rustc_hex::FromHex;
use solaris::unit;
use solaris::sol;
use ethabi::Caller;
use types::{U256, H256, Address};

use_contract!(foreign_bridge, "ForeignBridge", "contracts/bridge_sol_ForeignBridge.abi");

#[test]
fn should_allow_a_single_authority_to_confirm_a_deposit() {
    let contract = foreign_bridge::ForeignBridge::default();
    let code_hex = include_str!("../contracts/bridge_sol_ForeignBridge.bin");
    let code_bytes = code_hex.from_hex().unwrap();

    let mut evm = solaris::evm();

    let authority_addresses = vec![
        sol::address(10),
        sol::address(11),
        // sol::address(12),
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
        // .with_gas_price(0.into())
        // .with_gas(500000.into())
        .with_gas(4_000_000.into())
        // .ensure_funds()
        .deploy(&constructor_result)
        .unwrap();
    println!("deploy complete. contract_address = {:?}", contract_address);

    let fns = contract.functions();

    let result = fns
        .deposit()
        .transact(
            user_address,
            value,
            transaction_hash,
            evm
                .with_sender(authority_addresses[0].clone())
                .with_gas(4_000_000.into())
        )
        .unwrap();

    // let result = evm
    //     .with_sender(authority_addresses[0].clone())
    //     .with_gas(4_000_000.into())
    //     .call(fns.deposit().input(user_address, value, transaction_hash));

    println!("result = {:?}", result);

	let filter = foreign_bridge::events::Deposit::default().create_filter();
    assert_eq!(
        evm.logs(filter).len(),
        1,
        "exactly one deposit event should be created");

	let filter = foreign_bridge::events::Deposit::default().create_filter();
    for log in evm.logs(filter) {
        println!("log entry = {:?}", log);
    }

    // assert_eq!(
    //     U256::from_big_endian(&*evm.call(fns.is_authority().input(user_address)).unwrap()),
    //     sol::uint(0)
    // );
    //
    // for authority_address in authority_addresses.iter() {
    //     assert_eq!(
    //         U256::from_big_endian(&*evm.call(fns.is_authority().input(*authority_address)).unwrap()),
    //         sol::uint(1)
    //     );
    // }
}
