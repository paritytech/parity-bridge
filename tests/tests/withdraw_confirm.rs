extern crate futures;
extern crate bridge;
#[macro_use]
extern crate tests;

use bridge::bridge::create_withdraw_confirm;

test_app_stream! {
	name => withdraw_confirm_basic,
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
	txs => Transactions::default(),
	init => |app, db| create_withdraw_confirm(app, db).take(2),
	expected => vec![0x1005, 0x1006],
	mainnet_transport => [],
	testnet_transport => [
		"eth_blockNumber" =>
			req => r#"[]"#,
			res => r#""0x1011""#;
		"eth_getLogs" =>
			req => r#"[{"address":["0x0000000000000000000000000000000000000000"],"fromBlock":"0x1","limit":null,"toBlock":"0x1005","topics":[["0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"],[],[],[]]}]"#,
			res => r#"[]"#;
		"eth_blockNumber" =>
			req => r#"[]"#,
			res => r#""0x1012""#;
		"eth_getLogs" =>
			req => r#"[{"address":["0x0000000000000000000000000000000000000000"],"fromBlock":"0x1006","limit":null,"toBlock":"0x1006","topics":[["0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"],[],[],[]]}]"#,
			res => r#"[]"#;
	]
}

test_app_stream! {
	name => withdraw_confirm_confirmations,
	database => Database {
		checked_withdraw_confirm: 0xf5,
		..Database::default()
	},
	mainnet =>
		account => "0x0000000000000000000000000000000000000001",
		confirmations => 1;
	testnet =>
		account => "0x0000000000000000000000000000000000000001",
		confirmations => 12;
	authorities =>
		accounts => [
			"0x0000000000000000000000000000000000000001",
			"0x0000000000000000000000000000000000000002",
		],
		signatures => 1;
	txs => Transactions::default(),
	init => |app, db| create_withdraw_confirm(app, db).take(2),
	expected => vec![0x1005, 0x1006],
	mainnet_transport => [],
	testnet_transport => [
		"eth_blockNumber" =>
			req => r#"[]"#,
			res => r#""0x0100""#;
		"eth_blockNumber" =>
			req => r#"[]"#,
			res => r#""0x1011""#;
		"eth_getLogs" =>
			req => r#"[{"address":["0x0000000000000000000000000000000000000000"],"fromBlock":"0xf6","limit":null,"toBlock":"0x1005","topics":[["0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"],[],[],[]]}]"#,
			res => r#"[]"#;
		"eth_blockNumber" =>
			req => r#"[]"#,
			res => r#""0x1012""#;
		"eth_getLogs" =>
			req => r#"[{"address":["0x0000000000000000000000000000000000000000"],"fromBlock":"0x1006","limit":null,"toBlock":"0x1006","topics":[["0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"],[],[],[]]}]"#,
			res => r#"[]"#;
	]
}

test_app_stream! {
	name => withdraw_confirm_contract_address,
	database => Database {
		checked_withdraw_confirm: 0x00F5,
		mainnet_contract_address: "0x49edf201c1e139282643d5e7c6fb0c7219ad1db7".parse().unwrap(),
		testnet_contract_address: "0x49edf201c1e139282643d5e7c6fb0c7219ad1db8".parse().unwrap(),
		..Database::default()
	},
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
	txs => Transactions::default(),
	init => |app, db| create_withdraw_confirm(app, db).take(2),
	expected => vec![0x1005, 0x1006],
	mainnet_transport => [],
	testnet_transport => [
		"eth_blockNumber" =>
			req => r#"[]"#,
			res => r#""0x0100""#;
		"eth_blockNumber" =>
			req => r#"[]"#,
			res => r#""0x1011""#;
		"eth_getLogs" =>
			req => r#"[{"address":["0x49edf201c1e139282643d5e7c6fb0c7219ad1db8"],"fromBlock":"0xf6","limit":null,"toBlock":"0x1005","topics":[["0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"],[],[],[]]}]"#,
			res => r#"[]"#;
		"eth_blockNumber" =>
			req => r#"[]"#,
			res => r#""0x1012""#;
		"eth_getLogs" =>
			req => r#"[{"address":["0x49edf201c1e139282643d5e7c6fb0c7219ad1db8"],"fromBlock":"0x1006","limit":null,"toBlock":"0x1006","topics":[["0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"],[],[],[]]}]"#,
			res => r#"[]"#;
	]
}
