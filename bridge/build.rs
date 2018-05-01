use std::process::Command;

fn main() {
    // rerun build script if bridge contract has changed.
    // without this cargo doesn't since the bridge contract
    // is outside the crate directories
    println!("cargo:rerun-if-changed=../contracts/bridge.sol");

    // make last git commit hash (`git rev-parse HEAD`)
    // available via `env!("GIT_HASH")` in sources
    let output = Command::new("git")
        .args(&["rev-parse", "HEAD"])
        .output()
        .unwrap();
    let git_hash = String::from_utf8(output.stdout).unwrap();
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);

    match Command::new("solc").arg("--version").output() {
            Ok(exit_status) => {
                    let output_string = String::from_utf8(exit_status.stdout).unwrap();
                    let solc_version = output_string.lines().last().unwrap();
                    println!("cargo:rustc-env=SOLC_VERSION={}", solc_version);
            }
            Err(err) => {
                    if let std::io::ErrorKind::NotFound = err.kind() {
                            panic!("`solc` executable not found in `$PATH`. `solc` is required to compile the bridge contracts. please install it: https://solidity.readthedocs.io/en/develop/installing-solidity.html");
                    } else {
                            panic!("Unable to run solc: {}", err);
                    }
            }
    }

    // compile contracts for inclusion with ethabis `use_contract!`
    match Command::new("solc")
        .arg("--abi")
        .arg("--bin")
        .arg("--optimize")
        .arg("--output-dir")
        .arg("../compiled_contracts")
        .arg("--overwrite")
        .arg("../contracts/bridge.sol")
        .status()
    {
        Ok(exit_status) => {
            if !exit_status.success() {
                if let Some(code) = exit_status.code() {
                    panic!("`solc` exited with error exit status code `{}`", code);
                } else {
                    panic!("`solc` exited because it was terminated by a signal");
                }
            }
        }
        Err(err) => {
            if let std::io::ErrorKind::NotFound = err.kind() {
                panic!("`solc` executable not found in `$PATH`. `solc` is required to compile the bridge contracts. please install it: https://solidity.readthedocs.io/en/develop/installing-solidity.html");
            } else {
                panic!("an error occurred when trying to spawn `solc`: {}", err);
            }
        }
    }
}
