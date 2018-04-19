extern crate bridge;
extern crate ethereum_types;
/// spins up two parity nodes with the dev chain.
/// starts one bridge authority that connects the two.
/// does a deposit by sending ether to the HomeBridge.
/// asserts that the deposit got relayed to foreign chain.
/// does a withdraw by executing ForeignBridge.transferToHomeBridge.
/// asserts that the withdraw got relayed to home chain.
extern crate tempdir;
extern crate tokio_core;
extern crate web3;

use std::process::Command;
use std::time::Duration;
use std::thread;
use std::path::Path;

use tokio_core::reactor::Core;

use web3::transports::http::Http;
use web3::api::Namespace;
use ethereum_types::{Address, U256};

const TMP_PATH: &str = "tmp";
const MAX_PARALLEL_REQUESTS: usize = 10;

fn parity_home_command() -> Command {
    let mut command = Command::new("parity");
    command
        .arg("--base-path")
        .arg(format!("{}/home", TMP_PATH))
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

fn parity_foreign_command() -> Command {
    let mut command = Command::new("parity");
    command
        .arg("--base-path")
        .arg(format!("{}/foreign", TMP_PATH))
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

    // start a parity node that represents the home chain
    let mut parity_home = parity_home_command()
        .spawn()
        .expect("failed to spawn parity home node");

    // start a parity node that represents the foreign chain
    let mut parity_foreign = parity_foreign_command()
        .spawn()
        .expect("failed to spawn parity foreign node");

    // give the clients time to start up
    thread::sleep(Duration::from_millis(3000));

    // A address containing a lot of tokens (0x00a329c0648769a73afac7f9381e08fb43dbea72) should be
    // automatically added with a password being an empty string.
    // source: https://paritytech.github.io/wiki/Private-development-chain.html
    let user_address = "0x00a329c0648769a73afac7f9381e08fb43dbea72";

    let receiver_address = "0x05b344a728ebb2219459a008271264aef16adbc1";

    let authority_address = "0x00bd138abd70e2f00903268f3db08f2d25677c9e";

    // create authority account on home
    // this is currently not supported in web3 crate so we have to use curl
    let exit_status = Command::new("curl")
		.arg("--data").arg(r#"{"jsonrpc":"2.0","method":"parity_newAccountFromPhrase","params":["node0", ""],"id":0}"#)
		.arg("-H").arg("Content-Type: application/json")
		.arg("-X").arg("POST")
		.arg("localhost:8550")
		.status()
		.expect("failed to create authority account on home");
    assert!(exit_status.success());
    // TODO [snd] assert that created address matches authority_address

    // create authority account on foreign
    // this is currently not supported in web3 crate so we have to use curl
    let exit_status = Command::new("curl")
		.arg("--data").arg(r#"{"jsonrpc":"2.0","method":"parity_newAccountFromPhrase","params":["node0", ""],"id":0}"#)
		.arg("-H").arg("Content-Type: application/json")
		.arg("-X").arg("POST")
		.arg("localhost:8551")
		.status()
		.expect("failed to create/unlock authority account on foreign");
    assert!(exit_status.success());
    // TODO [snd] assert that created address matches authority_address

    // give the operations time to complete
    thread::sleep(Duration::from_millis(5000));

    // kill the clients so we can restart them with the accounts unlocked
    parity_home.kill().unwrap();
    parity_foreign.kill().unwrap();

    // wait for clients to shut down
    thread::sleep(Duration::from_millis(5000));

    // start a parity node that represents the home chain with accounts unlocked
    let mut parity_home = parity_home_command()
        .arg("--unlock")
        .arg(format!("{},{}", user_address, authority_address))
        .arg("--password")
        .arg("password.txt")
        .spawn()
        .expect("failed to spawn parity home node");

    // start a parity node that represents the foreign chain with accounts unlocked
    let mut parity_foreign = parity_foreign_command()
        .arg("--unlock")
        .arg(format!("{},{}", user_address, authority_address))
        .arg("--password")
        .arg("password.txt")
        .spawn()
        .expect("failed to spawn parity foreign node");

    // give nodes time to start up
    thread::sleep(Duration::from_millis(10000));

    // deploy bridge contracts

    assert!(
        Command::new("env")
            .arg("RUST_BACKTRACE=1")
            .arg("../target/debug/parity-bridge-deploy")
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

    let home_contract_address = "0xebd3944af37ccc6b67ff61239ac4fef229c8f69f";
    let foreign_contract_address = "0xebd3944af37ccc6b67ff61239ac4fef229c8f69f";

    let mut event_loop = Core::new().unwrap();

    // connect to home
    let home = bridge::contracts::home::HomeBridge::default();
    let home_transport = Http::with_event_loop("http://localhost:8550", &event_loop.handle(), MAX_PARALLEL_REQUESTS)
        .expect("failed to connect to home at http://localhost:8550");
    let home_eth = web3::api::Eth::new(home_transport.clone());

    // connect to foreign
    let foreign_transport = Http::with_event_loop("http://localhost:8551", &event_loop.handle(), MAX_PARALLEL_REQUESTS)
        .expect("failed to connect to foreign at http://localhost:8551");
    let foreign = bridge::contracts::foreign::ForeignBridge::default();
    let foreign_eth = web3::api::Eth::new(foreign_transport.clone());

    let response = event_loop
        .run(home_eth.call(
            web3::types::CallRequest {
                from: None,
                to: home_contract_address.into(),
                gas: None,
                gas_price: None,
                value: None,
                data: Some(web3::types::Bytes(
                    home.functions().estimated_gas_cost_of_withdraw().input(),
                )),
            },
            None,
        ))
        .unwrap();
    assert_eq!(
        home.functions()
            .estimated_gas_cost_of_withdraw()
            .output(response.0.as_slice())
            .unwrap(),
        U256::from(200000),
        "estimated gas cost of withdraw must be correct"
    );

    println!("\ngive authority some funds to do relay later\n");

    let balance = event_loop
        .run(home_eth.balance(authority_address.into(), None))
        .unwrap();
    assert_eq!(balance, web3::types::U256::from(0));
    event_loop
        .run(web3::confirm::send_transaction_with_confirmation(
            &home_transport,
            web3::types::TransactionRequest {
                from: user_address.into(),
                to: Some(authority_address.into()),
                gas: None,
                gas_price: Some(10.into()),
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
        .run(home_eth.balance(authority_address.into(), None))
        .unwrap();
    assert_eq!(balance, web3::types::U256::from(1000000000));

    // ensure receiver has 0 balance initially
    let balance = event_loop
        .run(home_eth.balance(receiver_address.into(), None))
        .unwrap();
    assert_eq!(balance, web3::types::U256::from(0));

    // ensure home contract has 0 balance initially
    let balance = event_loop
        .run(home_eth.balance(home_contract_address.into(), None))
        .unwrap();
    assert_eq!(balance, web3::types::U256::from(0));

    println!("\nuser deposits ether into HomeBridge\n");

    event_loop
        .run(web3::confirm::send_transaction_with_confirmation(
            &home_transport,
            web3::types::TransactionRequest {
                from: user_address.into(),
                to: Some(home_contract_address.into()),
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

    // ensure home contract balance has increased
    let balance = event_loop
        .run(home_eth.balance(home_contract_address.into(), None))
        .unwrap();
    assert_eq!(balance, web3::types::U256::from(1000000000));

    println!("\ndeposit into home complete. give it plenty of time to get mined and relayed\n");
    thread::sleep(Duration::from_millis(10000));

    let response = event_loop
        .run(foreign_eth.call(
            web3::types::CallRequest {
                from: None,
                to: foreign_contract_address.into(),
                gas: None,
                gas_price: None,
                value: None,
                data: Some(web3::types::Bytes(
                    foreign.functions().total_supply().input(),
                )),
            },
            None,
        ))
        .unwrap();
    assert_eq!(
        foreign
            .functions()
            .total_supply()
            .output(response.0.as_slice())
            .unwrap(),
        U256::from(1000000000),
        "totalSupply on ForeignBridge should have increased"
    );

    let response = event_loop
        .run(
            foreign_eth.call(
                web3::types::CallRequest {
                    from: None,
                    to: foreign_contract_address.into(),
                    gas: None,
                    gas_price: None,
                    value: None,
                    data: Some(web3::types::Bytes(
                        foreign
                            .functions()
                            .balance_of()
                            .input(Address::from(user_address)),
                    )),
                },
                None,
            ),
        )
        .unwrap();
    assert_eq!(
        foreign
            .functions()
            .balance_of()
            .output(response.0.as_slice())
            .unwrap(),
        U256::from(1000000000),
        "balance on ForeignBridge should have increased"
    );

    println!("\nconfirmed that deposit reached foreign\n");

    println!("\nuser executes ForeignBridge.transferHomeViaRelay\n");
    let transfer_payload = foreign.functions().transfer_home_via_relay().input(
        Address::from(receiver_address),
        U256::from(1000000000),
        U256::from(1000),
    );
    event_loop
        .run(web3::confirm::send_transaction_with_confirmation(
            &foreign_transport,
            web3::types::TransactionRequest {
                from: user_address.into(),
                to: Some(foreign_contract_address.into()),
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

    println!("\nForeignBridge.transferHomeViaRelay transaction sent. give it plenty of time to get mined and relayed\n");
    thread::sleep(Duration::from_millis(10000));

    // test that withdraw completed
    let balance = event_loop
        .run(home_eth.balance(receiver_address.into(), None))
        .unwrap();
    println!("balance = {}", balance);
    assert_eq!(balance, web3::types::U256::from(800000000));

    // ensure home contract balance has decreased
    let balance = event_loop
        .run(home_eth.balance(home_contract_address.into(), None))
        .unwrap();
    assert_eq!(balance, web3::types::U256::from(0));

    println!("\nconfirmed that withdraw reached home\n");

    bridge1.kill().unwrap();

    // wait for bridge to shut down
    thread::sleep(Duration::from_millis(1000));

    parity_home.kill().unwrap();
    parity_foreign.kill().unwrap();
}
