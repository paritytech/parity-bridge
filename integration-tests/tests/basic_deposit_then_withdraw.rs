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
extern crate bridge;
extern crate bridge_contracts;
extern crate ethabi;
extern crate ethereum_types;
/// spins up two parity nodes with the dev chain.
/// starts one bridge authority that connects the two.
/// does a deposit by sending ether to the MainBridge.
/// asserts that the deposit got relayed to side chain.
/// does a withdraw by executing SideBridge.transferToMainViaRelay.
/// asserts that the withdraw got relayed to main chain.
extern crate tempdir;
extern crate tokio_core;
extern crate web3;

use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;

use tokio_core::reactor::Core;

use bridge::helpers::AsyncCall;
use ethereum_types::{Address, U256};
use web3::api::Namespace;
use web3::transports::http::Http;

const TMP_PATH: &str = "tmp";
const MAX_PARALLEL_REQUESTS: usize = 10;
const TIMEOUT: Duration = Duration::from_secs(1);

fn parity_main_command() -> Command {
    let mut command = Command::new("parity");
    command
        .arg("--base-path")
        .arg(format!("{}/main", TMP_PATH))
        .arg("--chain")
        .arg("dev")
        .arg("--no-ipc")
        .arg("--logging")
        .arg("rpc=trace")
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
        .arg("dev")
        .arg("--no-ipc")
        .arg("--logging")
        .arg("rpc=trace")
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
    let _tmp_dir = tempdir::TempDir::new(TMP_PATH).expect("failed to create tmp dir");

    println!("\nbuild the deploy executable so we can run it later\n");
    assert!(
        Command::new("cargo")
            .env("RUST_BACKTRACE", "1")
            .current_dir("../deploy")
            .arg("build")
            .status()
            .expect("failed to build parity-bridge-deploy executable")
            .success()
    );

    println!("\nbuild the parity-bridge executable so we can run it later\n");
    assert!(
        Command::new("cargo")
            .env("RUST_BACKTRACE", "1")
            .current_dir("../cli")
            .arg("build")
            .status()
            .expect("failed to build parity-bridge executable")
            .success()
    );

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

    // A address containing a lot of tokens (0x00a329c0648769a73afac7f9381e08fb43dbea72) should be
    // automatically added with a password being an empty string.
    // source: https://paritytech.github.io/wiki/Private-development-chain.html
    let user_address = "0x00a329c0648769a73afac7f9381e08fb43dbea72";

    let receiver_address = "0x05b344a728ebb2219459a008271264aef16adbc1";

    let authority_address = "0x00bd138abd70e2f00903268f3db08f2d25677c9e";

    // create authority account on main
    // this is currently not supported in web3 crate so we have to use curl
    let exit_status = Command::new("curl")
		.arg("--data").arg(r#"{"jsonrpc":"2.0","method":"parity_newAccountFromPhrase","params":["node0", ""],"id":0}"#)
		.arg("-H").arg("Content-Type: application/json")
		.arg("-X").arg("POST")
		.arg("localhost:8550")
		.status()
		.expect("failed to create authority account on main");
    assert!(exit_status.success());
    // TODO [snd] assert that created address matches authority_address

    // TODO don't shell out to curl
    // create authority account on side
    // this is currently not supported in web3 crate so we have to use curl
    let exit_status = Command::new("curl")
		.arg("--data").arg(r#"{"jsonrpc":"2.0","method":"parity_newAccountFromPhrase","params":["node0", ""],"id":0}"#)
		.arg("-H").arg("Content-Type: application/json")
		.arg("-X").arg("POST")
		.arg("localhost:8551")
		.status()
		.expect("failed to create/unlock authority account on side");
    assert!(exit_status.success());
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
        .arg(format!("{},{}", user_address, authority_address))
        .arg("--password")
        .arg("password.txt")
        .spawn()
        .expect("failed to spawn parity main node");

    // start a parity node that represents the side chain with accounts unlocked
    let mut parity_side = parity_side_command()
        .arg("--unlock")
        .arg(format!("{},{}", user_address, authority_address))
        .arg("--password")
        .arg("password.txt")
        .spawn()
        .expect("failed to spawn parity side node");

    // give nodes time to start up
    thread::sleep(Duration::from_millis(10000));

    // deploy bridge contracts

    println!("\ndeploying contracts\n");
    assert!(
        Command::new("env")
            .arg("RUST_BACKTRACE=1")
            .arg("../target/debug/parity-bridge-deploy")
            .env("RUST_LOG", "info")
            .arg("--config")
            .arg("bridge_config.toml")
            .arg("--database")
            .arg("tmp/bridge1_db.txt")
            .status()
            .expect("failed spawn parity-bridge-deploy")
            .success()
    );

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

    let main_contract_address = "0xebd3944af37ccc6b67ff61239ac4fef229c8f69f";
    let side_contract_address = "0xebd3944af37ccc6b67ff61239ac4fef229c8f69f";

    let mut event_loop = Core::new().unwrap();

    // connect to main
    let main_transport = Http::with_event_loop(
        "http://localhost:8550",
        &event_loop.handle(),
        MAX_PARALLEL_REQUESTS,
    ).expect("failed to connect to main at http://localhost:8550");
    let main_eth = web3::api::Eth::new(main_transport.clone());

    // connect to side
    let side_transport = Http::with_event_loop(
        "http://localhost:8551",
        &event_loop.handle(),
        MAX_PARALLEL_REQUESTS,
    ).expect("failed to connect to side at http://localhost:8551");

    let (payload, decoder) = bridge_contracts::main::functions::estimated_gas_cost_of_withdraw::call();
    let response = event_loop
        .run(AsyncCall::new(
            &main_transport,
            main_contract_address.into(),
            TIMEOUT,
            payload,
            decoder,
        ))
        .unwrap();

    assert_eq!(
        response,
        U256::from(200000),
        "estimated gas cost of withdraw must be correct"
    );

    println!("\ngive authority some funds to do relay later\n");

    let balance = event_loop
        .run(main_eth.balance(authority_address.into(), None))
        .unwrap();
    assert_eq!(balance, web3::types::U256::from(0));
    event_loop
        .run(web3::confirm::send_transaction_with_confirmation(
            &main_transport,
            web3::types::TransactionRequest {
                from: user_address.into(),
                to: Some(authority_address.into()),
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
    let balance = event_loop
        .run(main_eth.balance(authority_address.into(), None))
        .unwrap();
    assert_eq!(balance, web3::types::U256::from(1000000000));

    // ensure receiver has 0 balance initially
    let balance = event_loop
        .run(main_eth.balance(receiver_address.into(), None))
        .unwrap();
    assert_eq!(balance, web3::types::U256::from(0));

    // ensure main contract has 0 balance initially
    let balance = event_loop
        .run(main_eth.balance(main_contract_address.into(), None))
        .unwrap();
    assert_eq!(balance, web3::types::U256::from(0));

    println!("\nuser deposits ether into MainBridge\n");

    event_loop
        .run(web3::confirm::send_transaction_with_confirmation(
            &main_transport,
            web3::types::TransactionRequest {
                from: user_address.into(),
                to: Some(main_contract_address.into()),
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

    // ensure main contract balance has increased
    let balance = event_loop
        .run(main_eth.balance(main_contract_address.into(), None))
        .unwrap();
    assert_eq!(balance, web3::types::U256::from(1000000000));

    println!("\ndeposit into main complete. give it plenty of time to get mined and relayed\n");
    thread::sleep(Duration::from_millis(10000));

    let (payload, decoder) = bridge_contracts::side::functions::total_supply::call();
    let response = event_loop
        .run(AsyncCall::new(
            &side_transport,
            side_contract_address.into(),
            TIMEOUT,
            payload,
            decoder,
        ))
        .unwrap();

    assert_eq!(
        response,
        U256::from(1000000000),
        "totalSupply on SideBridge should have increased"
    );

    let (payload, decoder) = bridge_contracts::side::functions::balance_of::call(Address::from(user_address));
    let response = event_loop
        .run(AsyncCall::new(
            &side_transport,
            side_contract_address.into(),
            TIMEOUT,
            payload,
            decoder,
        ))
        .unwrap();

    assert_eq!(
        response,
        U256::from(1000000000),
        "balance on SideBridge should have increased"
    );

    println!("\nconfirmed that deposit reached side\n");

    println!("\nuser executes SideBridge.transferToMainViaRelay\n");
    let transfer_payload = bridge_contracts::side::functions::transfer_to_main_via_relay::encode_input(
        Address::from(receiver_address),
        U256::from(1000000000),
        U256::from(1000),
    );
    event_loop
        .run(web3::confirm::send_transaction_with_confirmation(
            &side_transport,
            web3::types::TransactionRequest {
                from: user_address.into(),
                to: Some(side_contract_address.into()),
                gas: None,
                gas_price: None,
                value: None,
                data: Some(web3::types::Bytes(transfer_payload)),
                condition: None,
                nonce: None,
            },
            Duration::from_secs(1),
            0,
        ))
        .unwrap();

    println!("\nSideBridge.transferToMainViaRelay transaction sent. give it plenty of time to get mined and relayed\n");
    thread::sleep(Duration::from_millis(10000));

    // test that withdraw completed
    let balance = event_loop
        .run(main_eth.balance(receiver_address.into(), None))
        .unwrap();
    println!("balance = {}", balance);
    assert_eq!(balance, web3::types::U256::from(800000000));

    // ensure main contract balance has decreased
    let balance = event_loop
        .run(main_eth.balance(main_contract_address.into(), None))
        .unwrap();
    assert_eq!(balance, web3::types::U256::from(0));

    println!("\nconfirmed that withdraw reached main\n");

    bridge1.kill().unwrap();

    // wait for bridge to shut down
    thread::sleep(Duration::from_millis(1000));

    parity_main.kill().unwrap();
    parity_side.kill().unwrap();
}
