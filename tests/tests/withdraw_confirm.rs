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

test_app_stream! {
	name => withdraw_confirm_payload_gas,
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
			"0x00000000000000000000000000000000000000F1",
			"0x00000000000000000000000000000000000000F2",
		],
		signatures => 1;
	txs => Transactions {
		withdraw_confirm: TransactionConfig {
			gas: 0xfe,
			gas_price: 0xa1,
		},
		..Default::default()
	},
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
			res => r#"[{"address":"0x49edf201c1e139282643d5e7c6fb0c7219ad1db8","topics":["0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"],"data":"0x000000000000000000000000aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0","type":"","transactionHash":"0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"}]"#;
		"eth_sign" =>
			req => r#"["0x0000000000000000000000000000000000000001","0xaff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"]"#,
			res => r#""0xa3f20717a250c2b0b729b7e5becbff67fdaef7e0699da4de7ca5895b02a170a12d887fd3b17bfdce3481f10bea41f45ba9f709d39ce8325427b57afcfc994cee1b""#; // TODO currently random
		"eth_sendTransaction" =>
			req => r#"[{"data":"0x630cea8e000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000c00000000000000000000000000000000000000000000000000000000000000041a3f20717a250c2b0b729b7e5becbff67fdaef7e0699da4de7ca5895b02a170a12d887fd3b17bfdce3481f10bea41f45ba9f709d39ce8325427b57afcfc994cee1b000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000054aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364000000000000000000000000","from":"0x0000000000000000000000000000000000000001","gas":"0xfe","gasPrice":"0xa1","to":"0x49edf201c1e139282643d5e7c6fb0c7219ad1db8"}]"#,
			res => r#""0x1db8f385535c0d178b8f40016048f3a3cffee8f94e68978ea4b277f57b638f0b""#;
		"eth_blockNumber" =>
			req => r#"[]"#,
			res => r#""0x1012""#;
		"eth_getLogs" =>
			req => r#"[{"address":["0x49edf201c1e139282643d5e7c6fb0c7219ad1db8"],"fromBlock":"0x1006","limit":null,"toBlock":"0x1006","topics":[["0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"],[],[],[]]}]"#,
			res => r#"[]"#;
	]
}

test_app_stream! {
	name => withdraw_confirm_payload_multiple,
	database => Database {
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
			"0x00000000000000000000000000000000000000F1",
			"0x00000000000000000000000000000000000000F2",
		],
		signatures => 1;
	txs => Transactions {
		withdraw_confirm: TransactionConfig {
			gas: 0xff,
			gas_price: 0xaa,
		},
		..Default::default()
	},
	init => |app, db| create_withdraw_confirm(app, db).take(2),
	expected => vec![0x2, 0x1006],
	mainnet_transport => [],
	testnet_transport => [
		"eth_blockNumber" =>
			req => r#"[]"#,
			res => r#""0xe""#;
		"eth_getLogs" =>
			req => r#"[{"address":["0x49edf201c1e139282643d5e7c6fb0c7219ad1db8"],"fromBlock":"0x1","limit":null,"toBlock":"0x2","topics":[["0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"],[],[],[]]}]"#,
			res => r#"[{"address":"0x49edf201c1e139282643d5e7c6fb0c7219ad1db8","topics":["0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"],"data":"0x000000000000000000000000aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0","type":"","transactionHash":"0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"},{"address":"0x49edf201c1e139282643d5e7c6fb0c7219ad1db8","topics":["0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"],"data":"0x000000000000000000000000aff3454fce5edbc8cca8697c15331677e6ebdddd0000000000000000000000000000000000000000000000000000000000000099","type":"","transactionHash":"0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424399"}]"#;
		"eth_sign" =>
			req => r#"["0x0000000000000000000000000000000000000001","0xaff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"]"#,
			res => r#""0x8697c15331677e6ebccccaff3454fce5edbc8cca8697c15331677aff3454fce5edbc8cca8697c15331677e6ebccccaff3454fce5edbc8cca8697c15331677e6ebc""#;
		"eth_sign" =>
			req => r#"["0x0000000000000000000000000000000000000001","0xaff3454fce5edbc8cca8697c15331677e6ebdddd0000000000000000000000000000000000000000000000000000000000000099884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424399"]"#,
			res => r#""0x8697c15331677e6ebccccaff3454fce5edbc8cca8697c15331677aff3454fce5edbc8cca8697c15331677e6ebccccaff3454fce5edbc8cca8697c15331677e6ebd""#;
		"eth_sendTransaction" =>
			req => r#"[{"data":"0x630cea8e000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000c000000000000000000000000000000000000000000000000000000000000000418697c15331677e6ebccccaff3454fce5edbc8cca8697c15331677aff3454fce5edbc8cca8697c15331677e6ebccccaff3454fce5edbc8cca8697c15331677e6ebc000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000054aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364000000000000000000000000","from":"0x0000000000000000000000000000000000000001","gas":"0xff","gasPrice":"0xaa","to":"0x49edf201c1e139282643d5e7c6fb0c7219ad1db8"}]"#,
			res => r#""0x1db8f385535c0d178b8f40016048f3a3cffee8f94e68978ea4b277f57b638f0b""#;
		"eth_sendTransaction" =>
			req => r#"[{"data":"0x630cea8e000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000c000000000000000000000000000000000000000000000000000000000000000418697c15331677e6ebccccaff3454fce5edbc8cca8697c15331677aff3454fce5edbc8cca8697c15331677e6ebccccaff3454fce5edbc8cca8697c15331677e6ebd000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000054aff3454fce5edbc8cca8697c15331677e6ebdddd0000000000000000000000000000000000000000000000000000000000000099884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424399000000000000000000000000","from":"0x0000000000000000000000000000000000000001","gas":"0xff","gasPrice":"0xaa","to":"0x49edf201c1e139282643d5e7c6fb0c7219ad1db8"}]"#,
			res => r#""0x1db8f385535c0d178b8f40016048f3a3cffee8f94e68978ea4b277f57b638f0c""#;
		"eth_blockNumber" =>
			req => r#"[]"#,
			res => r#""0x1012""#;
		"eth_getLogs" =>
			req => r#"[{"address":["0x49edf201c1e139282643d5e7c6fb0c7219ad1db8"],"fromBlock":"0x3","limit":null,"toBlock":"0x1006","topics":[["0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"],[],[],[]]}]"#,
			res => r#"[]"#;
	]
}
