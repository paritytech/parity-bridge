extern crate futures;
extern crate bridge;
#[macro_use]
extern crate tests;

use bridge::bridge::create_withdraw_confirm;

test_app_stream! {
	name => withdraw_confirm_basic,
	database => Database::default(),
	home =>
		account => "0x0000000000000000000000000000000000000001",
		confirmations => 12;
	foreign =>
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
	home_transport => [],
	foreign_transport => [
		"eth_blockNumber" =>
			req => r#"[]"#,
			res => r#""0x1011""#;
		"eth_getLogs" =>
			req => r#"[{"address":["0x0000000000000000000000000000000000000000"],"fromBlock":"0x1","limit":null,"toBlock":"0x1005","topics":[["0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"],null,null,null]}]"#,
			res => r#"[]"#;
		"eth_blockNumber" =>
			req => r#"[]"#,
			res => r#""0x1012""#;
		"eth_getLogs" =>
			req => r#"[{"address":["0x0000000000000000000000000000000000000000"],"fromBlock":"0x1006","limit":null,"toBlock":"0x1006","topics":[["0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"],null,null,null]}]"#,
			res => r#"[]"#;
	]
}

test_app_stream! {
	name => withdraw_confirm_confirmations,
	database => Database {
		checked_withdraw_confirm: 0xf5,
		..Database::default()
	},
	home =>
		account => "0x0000000000000000000000000000000000000001",
		confirmations => 1;
	foreign =>
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
	home_transport => [],
	foreign_transport => [
		"eth_blockNumber" =>
			req => r#"[]"#,
			res => r#""0x0100""#;
		"eth_blockNumber" =>
			req => r#"[]"#,
			res => r#""0x1011""#;
		"eth_getLogs" =>
			req => r#"[{"address":["0x0000000000000000000000000000000000000000"],"fromBlock":"0xf6","limit":null,"toBlock":"0x1005","topics":[["0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"],null,null,null]}]"#,
			res => r#"[]"#;
		"eth_blockNumber" =>
			req => r#"[]"#,
			res => r#""0x1012""#;
		"eth_getLogs" =>
			req => r#"[{"address":["0x0000000000000000000000000000000000000000"],"fromBlock":"0x1006","limit":null,"toBlock":"0x1006","topics":[["0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"],null,null,null]}]"#,
			res => r#"[]"#;
	]
}

test_app_stream! {
	name => withdraw_confirm_contract_address,
	database => Database {
		checked_withdraw_confirm: 0x00F5,
		home_contract_address: "0x49edf201c1e139282643d5e7c6fb0c7219ad1db7".parse().unwrap(),
		foreign_contract_address: "0x49edf201c1e139282643d5e7c6fb0c7219ad1db8".parse().unwrap(),
		..Database::default()
	},
	home =>
		account => "0x0000000000000000000000000000000000000001",
		confirmations => 12;
	foreign =>
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
	home_transport => [],
	foreign_transport => [
		"eth_blockNumber" =>
			req => r#"[]"#,
			res => r#""0x0100""#;
		"eth_blockNumber" =>
			req => r#"[]"#,
			res => r#""0x1011""#;
		"eth_getLogs" =>
			req => r#"[{"address":["0x49edf201c1e139282643d5e7c6fb0c7219ad1db8"],"fromBlock":"0xf6","limit":null,"toBlock":"0x1005","topics":[["0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"],null,null,null]}]"#,
			res => r#"[]"#;
		"eth_blockNumber" =>
			req => r#"[]"#,
			res => r#""0x1012""#;
		"eth_getLogs" =>
			req => r#"[{"address":["0x49edf201c1e139282643d5e7c6fb0c7219ad1db8"],"fromBlock":"0x1006","limit":null,"toBlock":"0x1006","topics":[["0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"],null,null,null]}]"#,
			res => r#"[]"#;
	]
}

test_app_stream! {
	name => withdraw_confirm_payload_gas,
	database => Database {
		checked_withdraw_confirm: 0x00F5,
		home_contract_address: "0x49edf201c1e139282643d5e7c6fb0c7219ad1db7".parse().unwrap(),
		foreign_contract_address: "0x49edf201c1e139282643d5e7c6fb0c7219ad1db8".parse().unwrap(),
		..Database::default()
	},
	home =>
		account => "0x0000000000000000000000000000000000000001",
		confirmations => 12;
	foreign =>
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
	home_transport => [],
	foreign_transport => [
		"eth_blockNumber" =>
			req => r#"[]"#,
			res => r#""0x0100""#;
		"eth_blockNumber" =>
			req => r#"[]"#,
			res => r#""0x1011""#;
		"eth_getLogs" =>
			req => r#"[{"address":["0x49edf201c1e139282643d5e7c6fb0c7219ad1db8"],"fromBlock":"0xf6","limit":null,"toBlock":"0x1005","topics":[["0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"],null,null,null]}]"#,
			res => r#"[{"address":"0x49edf201c1e139282643d5e7c6fb0c7219ad1db8","topics":["0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"],"data":"0x000000000000000000000000aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0","type":"","transactionHash":"0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"}]"#;
		"eth_sign" =>
			req => r#"["0x0000000000000000000000000000000000000001","0xaff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"]"#,
			res => r#""0x8697c15331677e6ebccccaff3454fce5edbc8cca8697c15331677aff3454fce5edbc8cca8697c15331677e6ebccccaff3454fce5edbc8cca8697c15331677e6ebc""#;
		"eth_sendTransaction" =>
			req => r#"[{"data":"0x630cea8e000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000c000000000000000000000000000000000000000000000000000000000000000418697c15331677e6ebccccaff3454fce5edbc8cca8697c15331677aff3454fce5edbc8cca8697c15331677e6ebccccaff3454fce5edbc8cca8697c15331677e6ebc000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000054aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364000000000000000000000000","from":"0x0000000000000000000000000000000000000001","gas":"0xfe","gasPrice":"0xa1","to":"0x49edf201c1e139282643d5e7c6fb0c7219ad1db8"}]"#,
			res => r#""0x1db8f385535c0d178b8f40016048f3a3cffee8f94e68978ea4b277f57b638f0b""#;
		"eth_blockNumber" =>
			req => r#"[]"#,
			res => r#""0x1012""#;
		"eth_getLogs" =>
			req => r#"[{"address":["0x49edf201c1e139282643d5e7c6fb0c7219ad1db8"],"fromBlock":"0x1006","limit":null,"toBlock":"0x1006","topics":[["0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"],null,null,null]}]"#,
			res => r#"[]"#;
	]
}

test_app_stream! {
	name => withdraw_confirm_payload_multiple,
	database => Database {
		home_contract_address: "0x49edf201c1e139282643d5e7c6fb0c7219ad1db7".parse().unwrap(),
		foreign_contract_address: "0x49edf201c1e139282643d5e7c6fb0c7219ad1db8".parse().unwrap(),
		..Database::default()
	},
	home =>
		account => "0x0000000000000000000000000000000000000001",
		confirmations => 12;
	foreign =>
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
	home_transport => [],
	foreign_transport => [
		"eth_blockNumber" =>
			req => r#"[]"#,
			res => r#""0xe""#;
		"eth_getLogs" =>
			req => r#"[{"address":["0x49edf201c1e139282643d5e7c6fb0c7219ad1db8"],"fromBlock":"0x1","limit":null,"toBlock":"0x2","topics":[["0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"],null,null,null]}]"#,
			res => r#"[{"address":"0x49edf201c1e139282643d5e7c6fb0c7219ad1db8","topics":["0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"],"data":"0x000000000000000000000000aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0","type":"","transactionHash":"0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"},{"address":"0x49edf201c1e139282643d5e7c6fb0c7219ad1db8","topics":["0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"],"data":"0x000000000000000000000000001da5bcab735024168f00b43abcc9ef522392e90000000000000000000000000000000000000000000000000000000000000099","type":"","transactionHash":"0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424399"}]"#;
		"eth_sign" =>
			req => r#"["0x0000000000000000000000000000000000000001","0xaff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"]"#,
			res => r#""0x8697c15331677e6ebccccaff3454fce5edbc8cca8697c15331677aff3454fce5edbc8cca8697c15331677e6ebccccaff3454fce5edbc8cca8697c15331677e6ebc""#;
		"eth_sign" =>
			req => r#"["0x0000000000000000000000000000000000000001","0x001da5bcab735024168f00b43abcc9ef522392e90000000000000000000000000000000000000000000000000000000000000099884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424399"]"#,
			res => r#""0x8a3b24c56e46f6fc9fa7ed14795745348059b8ac84d6ee93323e83a429e760ae6e89510834ee4d65eefacd74cddca53df61b5eba1c3007ed88d2eebff2e0e2151b""#;
		"eth_sendTransaction" =>
			req => r#"[{"data":"0x630cea8e000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000c000000000000000000000000000000000000000000000000000000000000000418697c15331677e6ebccccaff3454fce5edbc8cca8697c15331677aff3454fce5edbc8cca8697c15331677e6ebccccaff3454fce5edbc8cca8697c15331677e6ebc000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000054aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364000000000000000000000000","from":"0x0000000000000000000000000000000000000001","gas":"0xff","gasPrice":"0xaa","to":"0x49edf201c1e139282643d5e7c6fb0c7219ad1db8"}]"#,
			res => r#""0x1db8f385535c0d178b8f40016048f3a3cffee8f94e68978ea4b277f57b638f0b""#;
		"eth_sendTransaction" =>
			req => r#"[{"data":"0x630cea8e000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000c000000000000000000000000000000000000000000000000000000000000000418a3b24c56e46f6fc9fa7ed14795745348059b8ac84d6ee93323e83a429e760ae6e89510834ee4d65eefacd74cddca53df61b5eba1c3007ed88d2eebff2e0e2151b000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000054001da5bcab735024168f00b43abcc9ef522392e90000000000000000000000000000000000000000000000000000000000000099884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424399000000000000000000000000","from":"0x0000000000000000000000000000000000000001","gas":"0xff","gasPrice":"0xaa","to":"0x49edf201c1e139282643d5e7c6fb0c7219ad1db8"}]"#,
			res => r#""0x1db8f385535c0d178b8f40016048f3a3cffee8f94e68978ea4b277f57b638f0c""#;
		"eth_blockNumber" =>
			req => r#"[]"#,
			res => r#""0x1012""#;
		"eth_getLogs" =>
			req => r#"[{"address":["0x49edf201c1e139282643d5e7c6fb0c7219ad1db8"],"fromBlock":"0x3","limit":null,"toBlock":"0x1006","topics":[["0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"],null,null,null]}]"#,
			res => r#"[]"#;
	]
}
