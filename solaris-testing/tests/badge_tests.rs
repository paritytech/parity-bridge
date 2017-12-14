extern crate ethabi;
#[macro_use]
extern crate ethabi_derive;
#[macro_use]
extern crate ethabi_contract;
extern crate ethereum_types as types;
extern crate rustc_hex;
extern crate solaris;


use_contract!(badgereg, "BadgeReg", "contracts/BadgeReg_sol_BadgeReg.abi");

#[cfg(test)]
fn setup() -> (solaris::evm::Evm, badgereg::BadgeReg) {
    let contract = badgereg::BadgeReg::default();
    let code = include_str!("../contracts/BadgeReg_sol_BadgeReg.bin");
    let mut evm = solaris::evm();

    let owner = 3.into();
    let _address = evm.with_sender(owner).deploy(&code.from_hex().unwrap());

    (evm, contract)
}

#[cfg(test)]
use rustc_hex::FromHex;
#[cfg(test)]
use solaris::unit;
#[cfg(test)]
use solaris::sol;

#[test]
fn badge_reg_test_fee() {
    let (mut evm, contract) = setup();
    let reg = contract.functions();

    // Initial fee is 1 ETH
    assert_eq!(unit::convert(reg.fee().call(&mut evm).unwrap()), unit::ether(1));

    // The owner should be able to set the fee
    reg.set_fee().transact(unit::gwei(10), &mut evm).unwrap();

    // Fee should be updated
    assert_eq!(unit::convert(reg.fee().call(&mut evm).unwrap()), unit::gwei(10));

    // Other address should not be allowed to change the fee
    evm.with_sender(10.into());
    reg.set_fee().transact(unit::gwei(10), &mut evm).unwrap_err();
}

#[test]
fn anyone_should_be_able_to_register_a_badge() {
    let (evm, contract) = setup();
    let reg = contract.functions();

    evm.run(move |mut evm| {
        // Register new entry
        reg.register().transact(sol::address(10), sol::bytes32("test"),
        evm
        .with_value(unit::ether(2))
        .with_sender(5.into())
        .ensure_funds()
        )?;

        // TODO [ToDr] The API here is crap, we need to work on sth better.
        // Check that the event has been fired.
        assert_eq!(
            evm.logs(badgereg::events::Registered::default().create_filter(
                    sol::bytes32("test"),
                    ethabi::Topic::Any,
                    )).len(),
                    1
                  );

        // TODO [ToDr] Perhaps `with_` should not be persistent?
        evm.with_value(0.into());
        // Test that it was registered correctly
        assert_eq!(
            reg.from_name().call(sol::bytes32("test"), &mut evm)?,
            (sol::raw::uint(0), sol::raw::address(10), sol::raw::address(5), )
            );

        Ok(())
    })
}
