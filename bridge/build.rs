extern crate solc;

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
        .expect("`git rev-parse HEAD` failed to run. run it yourself to verify. file an issue if this persists");
    let git_hash = String::from_utf8(output.stdout).unwrap();
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);

    // make solc version used to compile contracts (`solc --version`)
    // available via `env!("SOLC_VERSION")` in sources
    let output = Command::new("solc").args(&["--version"]).output().expect(
        "`solc --version` failed to run. run it yourself to verify. file an issue if this persists",
    );
    let output_string = String::from_utf8(output.stdout).unwrap();
    let solc_version = output_string.lines().last().unwrap();
    println!("cargo:rustc-env=SOLC_VERSION={}", solc_version);

    // compile contracts for inclusion with ethabis `use_contract!`
    solc::compile_dir("../contracts", "../compiled_contracts").unwrap();
}
