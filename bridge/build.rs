use std::fs;
use std::process::Command;
use std::fs::DirEntry;

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
                } else{
                    let output = Command::new("solc").args(&["--version"]).output().unwrap();
                    let output_string = String::from_utf8(output.stdout).unwrap();
                    let solc_version = output_string.lines().last().unwrap();
                    println!("cargo:rustc-env=SOLC_VERSION={}", solc_version);
                }
            }
            Err(err) => {
                if let std::io::ErrorKind::NotFound = err.kind() {
                    match Command::new("solcjs").arg("--version").output() {
        Ok(exit_status) => {
            let output_string = String::from_utf8(exit_status.stdout).unwrap();
            let solc_version = output_string.lines().last().unwrap();
            println!("cargo:rustc-env=SOLC_VERSION={}", solc_version);

            match fs::remove_dir_all("../compiled_contracts"){
                Ok(()) => {
                    println!("Removed old files");
                }
                Err(err) => {
                    println!("Files not removed: {}", err);
                }
            };
            // compile contracts for inclusion with ethabis `use_contract!`
            match Command::new("solcjs")
                .arg("--abi")
                .arg("--bin")
                .arg("--optimize")
                .arg("--output-dir")
                .arg("../compiled_contracts")
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
                let prepend = "../compiled_contracts/";
                let paths = fs::read_dir("../compiled_contracts").unwrap();
                for path in paths {
                let _file_name: String = format!("{}{}",prepend, get_file_name(path.unwrap()));
                let _replaced_name : String = str::replace(&_file_name, "___contracts_bridge_sol_", "");
                match fs::rename(&_file_name, &_replaced_name){
                    Ok(_status) => {
                        println!("Renamed successfully");

                    }
                    Err(err) => {
                        println!("Tried looking for {}", &_file_name);
                        panic!("Error renaming: {}", err);
                    }
                }
                println!("Old name: {}, New name: {}", _file_name, _replaced_name);
            }
        }
        Err(err) => {
            if let std::io::ErrorKind::NotFound = err.kind() {
                panic!("`solcjs` executable not found in `$PATH`. Try running `npm install -g solc`");
            } else {
                panic!("Unable to run solcjs: {}", err);
            }
        }
    }
                    // make solc version used to compile contracts (`solc --version`)
                    // available via `env!("SOLC_VERSION")` in sources
                    let output = Command::new("solcjs").args(&["--version"]).output().unwrap();
                    let output_string = String::from_utf8(output.stdout).unwrap();
                    let solc_version = output_string.lines().last().unwrap();
                    println!("cargo:rustc-env=SOLC_VERSION={}", solc_version);

                } else {
                    panic!("an error occurred when trying to spawn `solc`: {}", err);
                }
            }
        }
}
fn get_file_name(path: DirEntry) -> String {
    let file_name : String = path
        .file_name()
        .into_string()
        .expect("error: the first argument is not a file \
                    system path representable in UTF-8.");
    file_name
}
