use std::process::Command;

fn main() {
    // rerun build script if bridge contract has changed.
    // without this cargo doesn't since the bridge contract
    // is outside the crate directories
    println!("cargo:rerun-if-changed=../contracts/bridge.sol");
    let exit_status = Command::new("solc")
        .arg("--abi")
        .arg("--bin")
        .arg("--output-dir").arg("../compiled_contracts")
        .arg("--overwrite")
        .arg("../contracts/bridge.sol")
        .status()
		.unwrap_or_else(|e| panic!("Error compiling solidity contracts: {}", e));
	assert!(exit_status.success(), "There was an error while compiling contracts code.");
}
