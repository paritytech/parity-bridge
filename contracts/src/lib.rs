extern crate ethabi;
#[macro_use]
extern crate ethabi_derive;
#[macro_use]
extern crate ethabi_contract;

use_contract!(home, "HomeBridge", "../compiled_contracts/HomeBridge.abi");
use_contract!(
    foreign,
    "ForeignBridge",
    "../compiled_contracts/ForeignBridge.abi"
);
