extern crate futures;
extern crate bridge;
#[macro_use]
extern crate tests;

use bridge::bridge::create_withdraw_relay;

test_app_stream! {
	name => withdraw_relay_basic,
	database => Database::default(),
	mainnet =>
		account => "0x0000000000000000000000000000000000000001",
		confirmations => 12;
	testnet =>
		account => "0x0000000000000000000000000000000000000001",
		confirmations => 12;
	authorities =>
		accounts => [
			"0x0000000000000000000000000000000000000001",
			"0x0000000000000000000000000000000000000002",
		],
		signatures => 1;
	init => |app, db| create_withdraw_relay(app, db).take(2),
	expected => vec![0x1005, 0x1006],
	mainnet_transport => [],
	testnet_transport => [
		"eth_blockNumber" =>
			req => r#"[]"#,
			res => r#""0x1011""#;
		"eth_getLogs" =>
			req => r#"[{"address":["0x0000000000000000000000000000000000000000"],"fromBlock":"0x1","limit":null,"toBlock":"0x1005","topics":[["0xeb043d149eedb81369bec43d4c3a3a53087debc88d2525f13bfaa3eecda28b5c"],[],[],[]]}]"#,
			res => r#"[]"#;
		"eth_blockNumber" =>
			req => r#"[]"#,
			res => r#""0x1012""#;
		"eth_getLogs" =>
			req => r#"[{"address":["0x0000000000000000000000000000000000000000"],"fromBlock":"0x1006","limit":null,"toBlock":"0x1006","topics":[["0xeb043d149eedb81369bec43d4c3a3a53087debc88d2525f13bfaa3eecda28b5c"],[],[],[]]}]"#,
			res => r#"[]"#;
	]
}
