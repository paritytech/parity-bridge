use_contract!(home, "HomeBridge", "../compiled_contracts/HomeBridge.abi");
use_contract!(
    foreign,
    "ForeignBridge",
    "../compiled_contracts/ForeignBridge.abi"
);

pub use self::home::HomeBridge;
pub use self::foreign::ForeignBridge;
