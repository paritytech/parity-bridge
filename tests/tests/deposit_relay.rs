extern crate futures;
#[macro_use]
extern crate serde_json;
extern crate bridge;
#[macro_use]
extern crate tests;

use bridge::bridge::create_deposit_relay;

const DEPOSIT_TOPIC: &str = "0xe1fffcc4923d04b559f4d29a8bfc6cda04eb5b0d3c460751c2402c5c5cc9109c";

test_app_stream! {
	name => deposit_relay_basic,
	database => Database::default(),
	home =>
		account => "0000000000000000000000000000000000000001",
		confirmations => 12;
	foreign =>
		account => "0000000000000000000000000000000000000001",
		confirmations => 12;
	authorities =>
		accounts => [
			"0000000000000000000000000000000000000001",
			"0000000000000000000000000000000000000002",
		],
		signatures => 1;
	txs => Transactions::default(),
	init => |app, db| create_deposit_relay(app, db).take(2),
	expected => vec![0x1005, 0x1006],
	home_transport => [
		"eth_blockNumber" =>
			req => json!([]),
			res => json!("0x1011");
		"eth_getLogs" =>
			req => json!([{
				"address": ["0x0000000000000000000000000000000000000000"],
				"fromBlock": "0x1",
				"limit": null,
				"toBlock": "0x1005",
				"topics": [[DEPOSIT_TOPIC], null, null, null]
			}]),
			res => json!([]);
		"eth_blockNumber" =>
			req => json!([]),
			res => json!("0x1012");
		"eth_getLogs" =>
			req => json!([{
				"address": ["0x0000000000000000000000000000000000000000"],
				"fromBlock": "0x1006",
				"limit": null,
				"toBlock": "0x1006",
				"topics": [[DEPOSIT_TOPIC], null, null, null]
			}]),
			res => json!([]);
	],
	foreign_transport => []
}

test_app_stream! {
	name => deposit_relay_single_log,
	database => Database {
		checked_deposit_relay: 5,
		..Default::default()
	},
	home =>
		account => "0000000000000000000000000000000000000001",
		confirmations => 12;
	foreign =>
		account => "0000000000000000000000000000000000000001",
		confirmations => 12;
	authorities =>
		accounts => [
			"0000000000000000000000000000000000000001",
			"0000000000000000000000000000000000000002",
		],
		signatures => 1;
	txs => Transactions::default(),
	init => |app, db| create_deposit_relay(app, db).take(2),
	expected => vec![0x1005, 0x1006],
	home_transport => [
		"eth_blockNumber" =>
			req => json!([]),
			res => json!("0x1011");
		"eth_getLogs" =>
			req => json!([{
				"address": ["0x0000000000000000000000000000000000000000"],
				"fromBlock": "0x6",
				"limit": null,
				"toBlock":"0x1005",
				"topics": [[DEPOSIT_TOPIC], null, null, null]
			}]),
			res => json!([{
				"address": "0x0000000000000000000000000000000000000000",
				"topics": [DEPOSIT_TOPIC],
				"data": "0x000000000000000000000000aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0",
				"type": "",
				"transactionHash": "0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"
			}]);
		"eth_blockNumber" =>
			req => json!([]),
			res => json!("0x1012");
		"eth_getLogs" =>
			req => json!([{
				"address": ["0x0000000000000000000000000000000000000000"],
				"fromBlock": "0x1006",
				"limit": null,
				"toBlock": "0x1006",
				"topics":[[DEPOSIT_TOPIC], null, null, null]
			}]),
			res => json!([]);
	],
	foreign_transport => [
		"eth_sendTransaction" =>
			req => json!([{
				"data": "0x26b3293f000000000000000000000000aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364",
				"from": "0x0000000000000000000000000000000000000001",
				"gas": "0x0",
				"gasPrice": "0x0",
				"to": "0x0000000000000000000000000000000000000000"
			}]),
			res => json!("0x1db8f385535c0d178b8f40016048f3a3cffee8f94e68978ea4b277f57b638f0b");
	]
}

test_app_stream! {
	name => deposit_relay_check_gas,
	database => Database {
		checked_deposit_relay: 5,
		..Default::default()
	},
	home =>
		account => "0000000000000000000000000000000000000001",
		confirmations => 12;
	foreign =>
		account => "0000000000000000000000000000000000000001",
		confirmations => 12;
	authorities =>
		accounts => [
			"0000000000000000000000000000000000000001",
			"0000000000000000000000000000000000000002",
		],
		signatures => 1;
	txs => Transactions {
		deposit_relay: TransactionConfig {
			gas: 0xfd,
			gas_price: 0xa0,
		},
		..Default::default()
	},
	init => |app, db| create_deposit_relay(app, db).take(1),
	expected => vec![0x1005],
	home_transport => [
		"eth_blockNumber" =>
			req => json!([]),
			res => json!("0x1011");
		"eth_getLogs" =>
			req => json!([{
				"address": ["0x0000000000000000000000000000000000000000"],
				"fromBlock": "0x6",
				"limit": null,
				"toBlock": "0x1005",
				"topics": [[DEPOSIT_TOPIC], null, null, null]
			}]),
			res => json!([{
				"address": "0x0000000000000000000000000000000000000000",
				"topics": [DEPOSIT_TOPIC],
				"data": "0x000000000000000000000000aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0","type":"","transactionHash":"0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"
			}]);
	],
	foreign_transport => [
		"eth_sendTransaction" =>
			req => json!([{
				"data": "0x26b3293f000000000000000000000000aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364",
				"from": "0x0000000000000000000000000000000000000001",
				"gas": "0xfd",
				"gasPrice": "0xa0",
				"to": "0x0000000000000000000000000000000000000000"
			}]),
			res => json!("0x1db8f385535c0d178b8f40016048f3a3cffee8f94e68978ea4b277f57b638f0b");
	]
}

test_app_stream! {
	name => deposit_relay_contract_address,
	database => Database {
		home_contract_address: "0000000000000000000000000000000000000cc1".into(),
		foreign_contract_address: "0000000000000000000000000000000000000dd1".into(),
		..Default::default()
	},
	home =>
		account => "0000000000000000000000000000000000000001",
		confirmations => 12;
	foreign =>
		account => "0000000000000000000000000000000000000001",
		confirmations => 12;
	authorities =>
		accounts => [
			"0000000000000000000000000000000000000001",
			"0000000000000000000000000000000000000002",
		],
		signatures => 1;
	txs => Transactions::default(),
	init => |app, db| create_deposit_relay(app, db).take(1),
	expected => vec![0x1005],
	home_transport => [
		"eth_blockNumber" =>
			req => json!([]),
			res => json!("0x1011");
		"eth_getLogs" =>
			req => json!([{
				"address": ["0x0000000000000000000000000000000000000cc1"],
				"fromBlock": "0x1",
				"limit": null,
				"toBlock": "0x1005",
				"topics": [[DEPOSIT_TOPIC], null, null, null]
			}]),
			res => json!([{
				"address": "0x0000000000000000000000000000000000000cc1",
				"topics": ["0xe1fffcc4923d04b559f4d29a8bfc6cda04eb5b0d3c460751c2402c5c5cc9109c"],
				"data": "0x000000000000000000000000aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0",
				"type": "",
				"transactionHash": "0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"
			}]);
	],
	foreign_transport => [
		"eth_sendTransaction" =>
			req => json!([{
				"data": "0x26b3293f000000000000000000000000aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364",
				"from": "0x0000000000000000000000000000000000000001",
				"gas": "0x0",
				"gasPrice": "0x0",
				"to": "0x0000000000000000000000000000000000000dd1"
			}]),
			res => json!("0x1db8f385535c0d178b8f40016048f3a3cffee8f94e68978ea4b277f57b638f0b");
	]
}

test_app_stream! {
	name => deposit_relay_accounts,
	database => Database {
		home_contract_address: "0000000000000000000000000000000000000cc1".into(),
		foreign_contract_address: "0000000000000000000000000000000000000dd1".into(),
		..Default::default()
	},
	home =>
		account => "00000000000000000000000000000000000000ff",
		confirmations => 12;
	foreign =>
		account => "00000000000000000000000000000000000000ee",
		confirmations => 12;
	authorities =>
		accounts => [
			"0000000000000000000000000000000000000001",
			"0000000000000000000000000000000000000002",
		],
		signatures => 1;
	txs => Transactions::default(),
	init => |app, db| create_deposit_relay(app, db).take(1),
	expected => vec![0x1005],
	home_transport => [
		"eth_blockNumber" =>
			req => json!([]),
			res => json!("0x1011");
		"eth_getLogs" =>
			req => json!([{
				"address": ["0x0000000000000000000000000000000000000cc1"],
				"fromBlock": "0x1",
				"limit": null,
				"toBlock": "0x1005",
				"topics": [[DEPOSIT_TOPIC], null, null, null]
			}]),
			res => json!([{
				"address": "0x0000000000000000000000000000000000000cc1",
				"topics": [DEPOSIT_TOPIC],
				"data": "0x000000000000000000000000aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0",
				"type": "",
				"transactionHash": "0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"
			}]);
	],
	foreign_transport => [
		"eth_sendTransaction" =>
			req => json!([{
				"data": "0x26b3293f000000000000000000000000aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364",
				"from": "0x00000000000000000000000000000000000000ee",
				"gas": "0x0",
				"gasPrice": "0x0",
				"to":"0x0000000000000000000000000000000000000dd1"
			}]),
			res => json!("0x1db8f385535c0d178b8f40016048f3a3cffee8f94e68978ea4b277f57b638f0b");
	]
}

test_app_stream! {
	name => deposit_relay_multiple_logs,
	database => Database::default(),
	home =>
		account => "0000000000000000000000000000000000000001",
		confirmations => 12;
	foreign =>
		account => "0000000000000000000000000000000000000001",
		confirmations => 12;
	authorities =>
		accounts => [
			"0000000000000000000000000000000000000001",
			"0000000000000000000000000000000000000002",
		],
		signatures => 1;
	txs => Transactions::default(),
	init => |app, db| create_deposit_relay(app, db).take(1),
	expected => vec![0x1005],
	home_transport => [
		"eth_blockNumber" =>
			req => json!([]),
			res => json!("0x1011");
		"eth_getLogs" =>
			req => json!([{
				"address": ["0x0000000000000000000000000000000000000000"],
				"fromBlock": "0x1",
				"limit": null,
				"toBlock": "0x1005",
				"topics": [[DEPOSIT_TOPIC], null, null, null]
			}]),
			res => json!([
				{
					"address": "0x0000000000000000000000000000000000000000",
					"topics": ["0xe1fffcc4923d04b559f4d29a8bfc6cda04eb5b0d3c460751c2402c5c5cc9109c"],
					"data": "0x000000000000000000000000aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0",
					"type": "",
					"transactionHash": "0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"
				},
				{
					"address":"0x0000000000000000000000000000000000000000",
					"topics": ["0xe1fffcc4923d04b559f4d29a8bfc6cda04eb5b0d3c460751c2402c5c5cc9109c"],
					"data": "0x000000000000000000000000aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0",
					"type": "",
					"transactionHash": "0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a942436f"
				}
			]);
	],
	foreign_transport => [
		"eth_sendTransaction" =>
			req => json!([{
				"data": "0x26b3293f000000000000000000000000aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364",
				"from": "0x0000000000000000000000000000000000000001",
				"gas": "0x0",
				"gasPrice": "0x0",
				"to": "0x0000000000000000000000000000000000000000"
			}]),
			res => json!("0x1db8f385535c0d178b8f40016048f3a3cffee8f94e68978ea4b277f57b638f0b");
		"eth_sendTransaction" =>
			req => json!([{
				"data": "0x26b3293f000000000000000000000000aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a942436f",
				"from": "0x0000000000000000000000000000000000000001",
				"gas": "0x0",
				"gasPrice": "0x0",
				"to": "0x0000000000000000000000000000000000000000"
			}]),
			res => json!("0x1db8f385535c0d178b8f40016048f3a3cffee8f94e68978ea4b277f57b638f0b");
	]
}
