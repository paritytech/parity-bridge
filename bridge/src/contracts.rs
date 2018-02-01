use_contract!(home, "HomeBridge", "../compiled_contracts/HomeBridge.abi");
use_contract!(foreign, "ForeignBridge", "../compiled_contracts/ForeignBridge.abi");

pub const MESSAGE_LENGTH: usize = 116;
pub const SIGNATURE_LENGTH: usize = 65;
