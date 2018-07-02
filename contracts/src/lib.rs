extern crate ethabi;
#[macro_use]
extern crate ethabi_derive;
#[macro_use]
extern crate ethabi_contract;

use_contract!(home, "../compiled_contracts/HomeBridge.abi");
use_contract!(foreign, "../compiled_contracts/ForeignBridge.abi");
