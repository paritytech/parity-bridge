/// spins up two parity nodes with the dev chain.
/// starts bridge that connects the two.

extern crate tempdir;
extern crate ethereum_types;
extern crate rustc_hex;
extern crate web3;
extern crate tokio_core;
extern crate bridge;
extern crate futures;

use std::process::{Command, Child};
use std::time::Duration;
use std::thread;
use std::path::Path;
use futures::Future;

use tokio_core::reactor::Core;
use rustc_hex::ToHex;

use web3::transports::ipc::Ipc;
use web3::api::Namespace;
use ethereum_types::{Address, U256};

const TMP_PATH: &str = "tmp";

fn parity_home_command() -> Command {
	let mut command = Command::new("parity");
	command
		.arg("--base-path").arg(format!("{}/home", TMP_PATH))
		// .arg("--chain").arg("home_chain.json")
		.arg("--chain").arg("dev")
		// .arg("--config").arg("parity_home_config.toml")
		.arg("--ipc-path").arg("home.ipc")
		// .arg("--db-path").arg("tmp/home_chain")
		.arg("--logging").arg("rpc=trace")
		// .arg("--no-jsonrpc")
		.arg("--jsonrpc-port").arg("8550")
		.arg("--jsonrpc-apis").arg("all")
		.arg("--port").arg("30310")
		.arg("--gasprice").arg("0")
		.arg("--reseal-min-period").arg("0")
		.arg("--no-ws")
		.arg("--no-dapps")
		.arg("--no-ui");
	command
}

fn parity_foreign_command() -> Command {
	let mut command = Command::new("parity");
	command
		.arg("--base-path").arg(format!("{}/foreign", TMP_PATH))
		// .arg("--chain").arg("home_chain.json")
		.arg("--chain").arg("dev")
		// .arg("--config").arg("parity_home_config.toml")
		.arg("--ipc-path").arg("foreign.ipc")
		// .arg("--db-path").arg("tmp/home_chain")
		.arg("--logging").arg("rpc=trace")
		// .arg("--no-jsonrpc")
		.arg("--jsonrpc-port").arg("8551")
		.arg("--jsonrpc-apis").arg("all")
		.arg("--port").arg("30311")
		.arg("--gasprice").arg("0")
		.arg("--reseal-min-period").arg("0")
		.arg("--no-ws")
		.arg("--no-dapps")
		.arg("--no-ui");
	command
}

fn main() {
	if Path::new(TMP_PATH).exists() {
		std::fs::remove_dir_all(TMP_PATH).expect("failed to remove tmp dir");
	}
	let tmp_dir = tempdir::TempDir::new(TMP_PATH).expect("failed to create tmp dir");

	// TODO home_base
	// TODO foreign_base

	// build the bridge cli process
	let exit_status = Command::new("cargo")
		.env("RUST_BACKTRACE", "1")
		.current_dir("../cli")
		.arg("build")
		.status()
		.expect("failed to compile bridge cli");
	assert!(exit_status.success());

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

	// create authority account on home
	let exit_status = Command::new("curl")
		.arg("--data").arg(r#"{"jsonrpc":"2.0","method":"parity_newAccountFromPhrase","params":["node0", ""],"id":0}"#)
		.arg("-H").arg("Content-Type: application/json")
		.arg("-X").arg("POST")
		.arg("localhost:8550")
		.status()
		.expect("failed to create authority account on home");
	assert!(exit_status.success());

	// create authority account on foreign
	let exit_status = Command::new("curl")
		.arg("--data").arg(r#"{"jsonrpc":"2.0","method":"parity_newAccountFromPhrase","params":["node0", ""],"id":0}"#)
		.arg("-H").arg("Content-Type: application/json")
		.arg("-X").arg("POST")
		.arg("localhost:8551")
		.status()
		.expect("failed to create/unlock authority account on foreign");
	assert!(exit_status.success());

	// give the operations time to complete
	thread::sleep(Duration::from_millis(5000));

	// kill the clients so we can restart them with the accounts unlocked
	parity_home.kill().unwrap();
	parity_foreign.kill().unwrap();

	// wait for clients to shut down
	thread::sleep(Duration::from_millis(5000));

	// start a parity node that represents the home chain with accounts unlocked
	let mut parity_home = parity_home_command()
		.arg("--unlock").arg("0x00a329c0648769a73afac7f9381e08fb43dbea72,0x00bd138abd70e2f00903268f3db08f2d25677c9e")
		.arg("--password").arg("password.txt")
		.spawn()
		.expect("failed to spawn parity home node");

	// start a parity node that represents the foreign chain with accounts unlocked
	let mut parity_foreign = parity_foreign_command()
		.arg("--unlock").arg("0x00a329c0648769a73afac7f9381e08fb43dbea72,0x00bd138abd70e2f00903268f3db08f2d25677c9e")
		.arg("--password").arg("password.txt")
		.spawn()
		.expect("failed to spawn parity foreign node");

	// give nodes time to start up
	thread::sleep(Duration::from_millis(10000));

	// unlock authority account on home so bridge can connect later
	// let exit_status = Command::new("curl")
	//	   .arg("--data").arg(r#"{"jsonrpc":"2.0","method":"personal_unlockAccount","params":["0x00bd138abd70e2f00903268f3db08f2d25677c9e", "node0", "0x0"],"id":0}"#)
	//	   .arg("-H").arg("Content-Type: application/json")
	//	   .arg("-X").arg("POST")
	//	   .arg("localhost:8550")
	//	   .status()
	//	.expect("failed to unlock authority account on home");
	// assert!(exit_status.success());

	// unlock authority account on home so bridge can connect later
	// let exit_status = Command::new("curl")
	//	   .arg("--data").arg(r#"{"jsonrpc":"2.0","method":"personal_unlockAccount","params":["0x00bd138abd70e2f00903268f3db08f2d25677c9e", "node0", "0x0"],"id":0}"#)
	//	   .arg("-H").arg("Content-Type: application/json")
	//	   .arg("-X").arg("POST")
	//	   .arg("localhost:8551")
	//	   .status()
	//	.expect("failed to unlock authority account on foreign");
	// assert!(exit_status.success());

	// unlock user account which has a lot of ether on dev chain
	// on home and foreign
	// so we can make transactions
	// let exit_status = Command::new("curl")
	//	   .arg("--data").arg(r#"{"jsonrpc":"2.0","method":"personal_unlockAccount","params":["0x00a329c0648769a73afac7f9381e08fb43dbea72", "", "0x0"],"id":0}"#)
	//	   .arg("-H").arg("Content-Type: application/json")
	//	   .arg("-X").arg("POST")
	//	   .arg("localhost:8550")
	//	   .status()
	//	.expect("failed to unlock user account on home");
	// let exit_status = Command::new("curl")
	//	   .arg("--data").arg(r#"{"jsonrpc":"2.0","method":"personal_unlockAccount","params":["0x00a329c0648769a73afac7f9381e08fb43dbea72", "", "0x0"],"id":0}"#)
	//	   .arg("-H").arg("Content-Type: application/json")
	//	   .arg("-X").arg("POST")
	//	   .arg("localhost:8551")
	//	   .status()
	//	.expect("failed to unlock user account on foreign");

	// start bridge authority 1
	let mut bridge1 = Command::new("env")
		.arg("RUST_BACKTRACE=1")
		.arg("../target/debug/bridge")
		// .env("RUST_BACKTRACE", "1")
		.env("RUST_LOG", "info")
		.arg("--config").arg("bridge_config.toml")
		.arg("--database").arg("tmp/bridge1_db.txt")
		.spawn()
		.expect("failed to spawn bridge process");

	// give the bridge time to start up and deploy the contracts
	thread::sleep(Duration::from_millis(10000));

	// user deposits into HomeBridge
	let exit_status = Command::new("curl")
		.arg("--data").arg(r#"{"jsonrpc":"2.0","method":"eth_sendTransaction","params":[{
			"from": "0x00a329c0648769a73afac7f9381e08fb43dbea72",
			"to": "0xebd3944af37ccc6b67ff61239ac4fef229c8f69f",
			"value": "0x186a0"
		}],"id":0}"#)
		.arg("-H").arg("Content-Type: application/json")
		.arg("-X").arg("POST")
		.arg("localhost:8550")
		.status()
		.expect("failed to deposit into HomeBridge");
	assert!(exit_status.success());

	println!("\ndeposit into home sent\n");

	// wait for it to be mined
	thread::sleep(Duration::from_millis(10000));

	let mut event_loop = Core::new().unwrap();
	let foreign_transport = Ipc::with_event_loop("foreign.ipc", &event_loop.handle())
		.expect("failed to connect to foreign.ipc");
	let foreign = bridge::contracts::foreign::ForeignBridge::default();
	let foreign_eth = web3::api::Eth::new(foreign_transport);
	let home_transport = Ipc::with_event_loop("home.ipc", &event_loop.handle())
		.expect("failed to connect to home.ipc");
	let home = bridge::contracts::home::HomeBridge::default();
	let home_eth = web3::api::Eth::new(home_transport);

	// balance on ForeignBridge should have increased
	let balance_payload = foreign.functions().balances().input(Address::from("0x00a329c0648769a73afac7f9381e08fb43dbea72"));
	println!("balance_payload: {}", balance_payload.to_hex());

	let future = foreign_eth.call(web3::types::CallRequest{
		from: None,
		to: web3::types::Address::from(&Address::from("0xebd3944af37ccc6b67ff61239ac4fef229c8f69f").0[..]),
		gas: None,
		gas_price: None,
		value: None,
		data: Some(web3::types::Bytes(balance_payload)),
	}, None);

	println!("waiting for future");
	let response = event_loop.run(future).unwrap();
	let balance = U256::from(response.0.as_slice());
	assert_eq!(balance, U256::from(100000));

	println!("\nconfirmed that deposit reached foreign\n");

	thread::sleep(Duration::from_millis(10000));

	// withdraw
	let transfer_payload = foreign.functions()
		.transfer_home_via_relay()
		.input(
			Address::from("0x00aa39d30f0d20ff03a22ccfc30b7efbfca597c2"),
			U256::from(100000));
	let future = foreign_eth.send_transaction(web3::types::TransactionRequest{
		from: web3::types::Address::from(&Address::from("0x00a329c0648769a73afac7f9381e08fb43dbea72").0[..]),
		to: Some(web3::types::Address::from(&Address::from("0xebd3944af37ccc6b67ff61239ac4fef229c8f69f").0[..])),
		gas: None,
		gas_price: None,
		value: None,
		data: Some(web3::types::Bytes(transfer_payload)),
		condition: None,
		nonce: None,
	});
	let response = event_loop.run(future).unwrap();

	println!("\ntransfer to home transaction sent\n");

	thread::sleep(Duration::from_millis(10000));

	// test that withdraw completed
	let future = home_eth.balance(web3::types::Address::from(&Address::from("0x00aa39d30f0d20ff03a22ccfc30b7efbfca597c2").0[..]), None);
	println!("waiting for future");
	let balance = event_loop.run(future).unwrap();
	assert!(balance > web3::types::U256::from(0));

	println!("\nconfirmed that withdraw reached home\n");

	bridge1.kill().unwrap();

	// wait for bridge to shut down
	thread::sleep(Duration::from_millis(1000));

	parity_home.kill().unwrap();
	parity_foreign.kill().unwrap();
}