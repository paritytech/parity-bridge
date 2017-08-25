extern crate futures;
extern crate bridge;
#[macro_use]
extern crate tests;

use bridge::bridge::create_deposit_relay;

test_app_stream! {
	name => basic,
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
	init => |app, db| create_deposit_relay(app, db).take(2),
	expected => vec![0x1005, 0x1006],
	mainnet_transport => [
		"eth_blockNumber" =>
			req => r#"[]"#,
			res => r#""0x1011""#;
		"eth_getLogs" =>
			req => r#"[{"address":["0x0000000000000000000000000000000000000000"],"fromBlock":"0x1","limit":null,"toBlock":"0x1005","topics":[["0xe1fffcc4923d04b559f4d29a8bfc6cda04eb5b0d3c460751c2402c5c5cc9109c"],[],[],[]]}]"#,
			res => r#"[]"#;
		"eth_blockNumber" =>
			req => r#"[]"#,
			res => r#""0x1012""#;
		"eth_getLogs" =>
			req => r#"[{"address":["0x0000000000000000000000000000000000000000"],"fromBlock":"0x1006","limit":null,"toBlock":"0x1006","topics":[["0xe1fffcc4923d04b559f4d29a8bfc6cda04eb5b0d3c460751c2402c5c5cc9109c"],[],[],[]]}]"#,
			res => r#"[]"#;
	],
	testnet_transport => []
}
