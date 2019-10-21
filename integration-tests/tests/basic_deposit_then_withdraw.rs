// Copyright 2017 Parity Technologies (UK) Ltd.
// This file is part of Parity-Bridge.

// Parity-Bridge is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity-Bridge is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity-Bridge.  If not, see <http://www.gnu.org/licenses/>.

//! spins up two parity nodes with the dev chain.
//! starts one bridge authority that connects the two.
//! does a deposit by sending ether to the MainBridge.
//! asserts that the deposit got relayed to side chain.
//! does a withdraw by executing SideBridge.transferToMainViaRelay.
//! asserts that the withdraw got relayed to main chain.
extern crate bridge;
extern crate bridge_contracts;
extern crate ethabi;
extern crate ethereum_types;
extern crate rustc_hex;
extern crate tempfile;
extern crate tokio_core;
extern crate web3;

use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;

use tokio_core::reactor::Core;

use bridge::helpers::AsyncCall;
use rustc_hex::FromHex;
use web3::transports::http::Http;
use web3::types::Address;

const TMP_PATH: &str = "tmp";
const MAX_PARALLEL_REQUESTS: usize = 10;
const TIMEOUT: Duration = Duration::from_secs(1);

fn parity_main_command() -> Command {
    let mut command = Command::new("parity");
    command
        .arg("--base-path")
        .arg(format!("{}/main", TMP_PATH))
        .arg("--chain")
        .arg("./spec.json")
        .arg("--no-ipc")
        .arg("--logging")
        .arg("rpc=trace,miner=trace,executive=trace")
        .arg("--jsonrpc-port")
        .arg("8550")
        .arg("--jsonrpc-apis")
        .arg("all")
        .arg("--port")
        .arg("30310")
        .arg("--gasprice")
        .arg("0")
        .arg("--reseal-min-period")
        .arg("0")
        .arg("--no-ws")
        .arg("--no-dapps")
        .arg("--no-warp")
        .arg("--no-ui");
    command
}

fn parity_side_command() -> Command {
    let mut command = Command::new("parity");
    command
        .arg("--base-path")
        .arg(format!("{}/side", TMP_PATH))
        .arg("--chain")
        .arg("./spec.json")
        .arg("--no-ipc")
        .arg("--logging")
        .arg("rpc=trace,miner=trace,executive=trace")
        .arg("--jsonrpc-port")
        .arg("8551")
        .arg("--jsonrpc-apis")
        .arg("all")
        .arg("--port")
        .arg("30311")
        .arg("--gasprice")
        .arg("0")
        .arg("--reseal-min-period")
        .arg("0")
        .arg("--no-ws")
        .arg("--no-dapps")
        .arg("--no-warp")
        .arg("--no-ui");
    command
}

#[test]
fn test_basic_deposit_then_withdraw() {
    if Path::new(TMP_PATH).exists() {
        std::fs::remove_dir_all(TMP_PATH).expect("failed to remove tmp dir");
    }
    let _ = std::fs::create_dir_all(TMP_PATH).expect("failed to create tmp dir");
    let _tmp_dir = tempfile::TempDir::new_in(TMP_PATH).expect("failed to create tmp dir");

    println!("\nbuild the deploy executable so we can run it later\n");
    assert!(Command::new("cargo")
        .env("RUST_BACKTRACE", "1")
        .current_dir("../deploy")
        .arg("build")
        .status()
        .expect("failed to build parity-bridge-deploy executable")
        .success());

    println!("\nbuild the parity-bridge executable so we can run it later\n");
    assert!(Command::new("cargo")
        .env("RUST_BACKTRACE", "1")
        .current_dir("../cli")
        .arg("build")
        .status()
        .expect("failed to build parity-bridge executable")
        .success());

    // start a parity node that represents the main chain
    let mut parity_main = parity_main_command()
        .spawn()
        .expect("failed to spawn parity main node");

    // start a parity node that represents the side chain
    let mut parity_side = parity_side_command()
        .spawn()
        .expect("failed to spawn parity side node");

    // give the clients time to start up
    thread::sleep(Duration::from_millis(3000));

    let user_address = "004ec07d2329997267ec62b4166639513386f32e";
    let authority_address = "00bd138abd70e2f00903268f3db08f2d25677c9e";

    let main_contract_address = "ebd3944af37ccc6b67ff61239ac4fef229c8f69f";
    let side_contract_address = "ebd3944af37ccc6b67ff61239ac4fef229c8f69f";
    // Note this has to be the first contract created by `user_address`
    let main_recipient_address = "5f3dba5e45909d1bf126aa0af0601b1a369dbfd7";
    let side_recipient_address = "5f3dba5e45909d1bf126aa0af0601b1a369dbfd7";

    let data_to_relay_to_side = vec![0u8, 1, 5];
    let data_to_relay_to_main = vec![0u8, 1, 5, 7];

    fn new_account(phrase: &str, port: u16) {
        // this is currently not supported in web3 crate so we have to use curl
        let exit_status = Command::new("curl")
            .arg("--data")
            .arg(format!(
                "{}{}{}",
                r#"{"jsonrpc":"2.0","method":"parity_newAccountFromPhrase","params":[""#,
                phrase,
                r#"", ""],"id":0}"#,
            ))
            .arg("-H")
            .arg("Content-Type: application/json")
            .arg("-X")
            .arg("POST")
            .arg(format!("localhost:{}", port))
            .status()
            .expect("failed to create authority account on main");
        assert!(exit_status.success());
    }

    // create authority account on main
    new_account("node0", 8550);
    new_account("user", 8550);
    // TODO [snd] assert that created address matches authority_address

    // TODO don't shell out to curl
    // create authority account on side
    new_account("node0", 8551);
    new_account("user", 8551);
    // TODO [snd] assert that created address matches authority_address

    // give the operations time to complete
    thread::sleep(Duration::from_millis(5000));

    // kill the clients so we can restart them with the accounts unlocked
    parity_main.kill().unwrap();
    parity_side.kill().unwrap();

    // wait for clients to shut down
    thread::sleep(Duration::from_millis(5000));

    // start a parity node that represents the main chain with accounts unlocked
    let mut parity_main = parity_main_command()
        .arg("--unlock")
        .arg(format!("0x{},0x{}", user_address, authority_address))
        .arg("--password")
        .arg("password.txt")
        .spawn()
        .expect("failed to spawn parity main node");

    // start a parity node that represents the side chain with accounts unlocked
    let mut parity_side = parity_side_command()
        .arg("--unlock")
        .arg(format!("0x{},0x{}", user_address, authority_address))
        .arg("--password")
        .arg("password.txt")
        .spawn()
        .expect("failed to spawn parity side node");

    // give nodes time to start up
    thread::sleep(Duration::from_millis(10000));

    // deploy bridge contracts

    println!("\ndeploying contracts\n");
    assert!(Command::new("env")
        .arg("RUST_BACKTRACE=1")
        .arg("../target/debug/parity-bridge-deploy")
        .env("RUST_LOG", "info")
        .arg("--config")
        .arg("bridge_config.toml")
        .arg("--database")
        .arg("tmp/bridge1_db.txt")
        .status()
        .expect("failed spawn parity-bridge-deploy")
        .success());

    // start bridge authority 1
    let mut bridge1 = Command::new("env")
        .arg("RUST_BACKTRACE=1")
        .arg("../target/debug/parity-bridge")
        .env("RUST_LOG", "info")
        .arg("--config")
        .arg("bridge_config.toml")
        .arg("--database")
        .arg("tmp/bridge1_db.txt")
        .spawn()
        .expect("failed to spawn bridge process");

    let mut event_loop = Core::new().unwrap();

    // connect to main
    let main_transport = Http::with_event_loop(
        "http://localhost:8550",
        &event_loop.handle(),
        MAX_PARALLEL_REQUESTS,
    )
    .expect("failed to connect to main at http://localhost:8550");

    // connect to side
    let side_transport = Http::with_event_loop(
        "http://localhost:8551",
        &event_loop.handle(),
        MAX_PARALLEL_REQUESTS,
    )
    .expect("failed to connect to side at http://localhost:8551");

    println!("\ngive authority some funds to do relay later\n");

    event_loop
        .run(web3::confirm::send_transaction_with_confirmation(
            &main_transport,
            web3::types::TransactionRequest {
                from: user_address.parse().unwrap(),
                to: Some(authority_address.parse().unwrap()),
                gas: None,
                gas_price: None,
                value: Some(1000000000.into()),
                data: None,
                condition: None,
                nonce: None,
            },
            Duration::from_secs(1),
            0,
        ))
        .unwrap();

    event_loop
        .run(web3::confirm::send_transaction_with_confirmation(
            &side_transport,
            web3::types::TransactionRequest {
                from: user_address.parse().unwrap(),
                to: Some(authority_address.parse().unwrap()),
                gas: None,
                gas_price: None,
                value: Some(1000000000.into()),
                data: None,
                condition: None,
                nonce: None,
            },
            Duration::from_secs(1),
            0,
        ))
        .unwrap();

    println!("\ndeploy BridgeRecipient contracts\n");

    event_loop
        .run(web3::confirm::send_transaction_with_confirmation(
            &main_transport,
            web3::types::TransactionRequest {
                from: user_address.parse().unwrap(),
                to: None,
                gas: None,
                gas_price: None,
                value: None,
                data: Some(
                    include_str!("../../compiled_contracts/RecipientTest.bin")
                        .from_hex()
                        .unwrap()
                        .into(),
                ),
                condition: None,
                nonce: None,
            },
            Duration::from_secs(1),
            0,
        ))
        .unwrap();

    event_loop
        .run(web3::confirm::send_transaction_with_confirmation(
            &side_transport,
            web3::types::TransactionRequest {
                from: user_address.parse().unwrap(),
                to: None,
                gas: None,
                gas_price: None,
                value: None,
                data: Some(
                    include_str!("../../compiled_contracts/RecipientTest.bin")
                        .from_hex()
                        .unwrap()
                        .into(),
                ),
                condition: None,
                nonce: None,
            },
            Duration::from_secs(1),
            0,
        ))
        .unwrap();

    println!("\nSend the message to main chain and wait for the relay to side\n");

    let (payload, _) = bridge_contracts::main::functions::relay_message::call(
        data_to_relay_to_side.clone(),
        main_recipient_address.parse::<Address>().unwrap(),
    );

    event_loop
        .run(web3::confirm::send_transaction_with_confirmation(
            &main_transport,
            web3::types::TransactionRequest {
                from: user_address.parse().unwrap(),
                to: Some(main_contract_address.parse().unwrap()),
                gas: None,
                gas_price: None,
                value: None,
                data: Some(payload.into()),
                condition: None,
                nonce: None,
            },
            Duration::from_secs(1),
            0,
        ))
        .unwrap();

    println!(
        "\nSending message to main complete. Give it plenty of time to get mined and relayed\n"
    );
    thread::sleep(Duration::from_millis(10000));

    let (payload, decoder) = bridge_contracts::test::functions::last_data::call();

    let response = event_loop
        .run(AsyncCall::new(
            &side_transport,
            side_recipient_address.parse().unwrap(),
            TIMEOUT,
            payload,
            decoder,
        ))
        .unwrap();

    assert_eq!(
        response, data_to_relay_to_side,
        "data was not relayed properly to the side chain"
    );

    println!("\nSend the message to side chain and wait for the relay to main\n");

    let (payload, _) = bridge_contracts::side::functions::relay_message::call(
        data_to_relay_to_main.clone(),
        main_recipient_address.parse::<Address>().unwrap(),
    );

    event_loop
        .run(web3::confirm::send_transaction_with_confirmation(
            &side_transport,
            web3::types::TransactionRequest {
                from: user_address.parse().unwrap(),
                to: Some(side_contract_address.parse().unwrap()),
                gas: None,
                gas_price: None,
                value: None,
                data: Some(payload.into()),
                condition: None,
                nonce: None,
            },
            Duration::from_secs(1),
            0,
        ))
        .unwrap();

    println!(
        "\nSending message to side complete. Give it plenty of time to get mined and relayed\n"
    );
    thread::sleep(Duration::from_millis(15000));

    //dwd

    //let main_web3 = web3::Web3::new(&main_transport);
    //let code_future = main_web3.eth().code(main_recipient_address.into(), None);
    //let code = event_loop.run(code_future).unwrap();
    //println!("code: {:?}", code);

    //// TODO: remove
    //bridge1.kill().unwrap();

    //// wait for bridge to shut down
    //thread::sleep(Duration::from_millis(1000));
    //parity_main.kill().unwrap();
    //parity_side.kill().unwrap();

    //assert!(false);

    let (payload, decoder) = bridge_contracts::test::functions::last_data::call();

    let response = event_loop
        .run(AsyncCall::new(
            &main_transport,
            main_recipient_address.parse().unwrap(),
            TIMEOUT,
            payload,
            decoder,
        ))
        .unwrap();

    assert_eq!(
        response, data_to_relay_to_main,
        "data was not relayed properly to the main chain"
    );

    bridge1.kill().unwrap();

    // wait for bridge to shut down
    thread::sleep(Duration::from_millis(1000));

    parity_main.kill().unwrap();
    parity_side.kill().unwrap();
}
