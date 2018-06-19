extern crate bridge;
extern crate ethabi;
extern crate ethereum_types;
/// test interactions of withdraw_confirm state machine with RPC
extern crate futures;
extern crate rustc_hex;
#[macro_use]
extern crate serde_json;

// use bridge::bridge::create_withdraw_confirm;
// use bridge::contracts;
// use bridge::message_to_mainnet::MessageToMainnet;
// use ethabi::{encode, Token};
// use rustc_hex::{FromHex, ToHex};
//
// const WITHDRAW_TOPIC: &str = "0xf279e6a1f5e320cca91135676d9cb6e44ca8a08c0b88342bcdb1144f6511b568";
//
// test_app_stream! {
//     name => withdraw_confirm_basic,
//     database => Database::default(),
//     home =>
//         account => "0000000000000000000000000000000000000001",
//         confirmations => 12;
//     foreign =>
//         account => "0000000000000000000000000000000000000001",
//         confirmations => 12;
//     authorities =>
//         accounts => [
//             "0000000000000000000000000000000000000001",
//             "0000000000000000000000000000000000000002",
//         ],
//         signatures => 1;
//     txs => Transactions::default(),
//     init => |app, db| create_withdraw_confirm(app, db).take(2),
//     expected => vec![0x1005, 0x1006],
//     home_transport => [],
//     foreign_transport => [
//         "eth_blockNumber" =>
//             req => json!([]),
//             res => json!("0x1011");
//         "eth_getLogs" =>
//             req => json!([{
//                 "address": ["0x0000000000000000000000000000000000000000"],
//                 "fromBlock": "0x1",
//                 "limit": null,
//                 "toBlock": "0x1005",
//                 "topics": [[WITHDRAW_TOPIC], null, null, null]
//             }]),
//             res => json!([]);
//         "eth_blockNumber" =>
//             req => json!([]),
//             res => json!("0x1012");
//         "eth_getLogs" =>
//             req => json!([{
//                 "address": ["0x0000000000000000000000000000000000000000"],
//                 "fromBlock": "0x1006",
//                 "limit": null,
//                 "toBlock": "0x1006",
//                 "topics":[[WITHDRAW_TOPIC], null, null, null]
//             }]),
//             res => json!([]);
//     ]
// }
//
// test_app_stream! {
//     name => withdraw_confirm_confirmations,
//     database => Database {
//         checked_withdraw_confirm: 0xf5,
//         ..Database::default()
//     },
//     home =>
//         account => "0000000000000000000000000000000000000001",
//         confirmations => 1;
//     foreign =>
//         account => "0000000000000000000000000000000000000001",
//         confirmations => 12;
//     authorities =>
//         accounts => [
//             "0000000000000000000000000000000000000001",
//             "0000000000000000000000000000000000000002",
//         ],
//         signatures => 1;
//     txs => Transactions::default(),
//     init => |app, db| create_withdraw_confirm(app, db).take(2),
//     expected => vec![0x1005, 0x1006],
//     home_transport => [],
//     foreign_transport => [
//         "eth_blockNumber" =>
//             req => json!([]),
//             res => json!("0x0100");
//         "eth_blockNumber" =>
//             req => json!([]),
//             res => json!("0x1011");
//         "eth_getLogs" =>
//             req => json!([{
//                 "address":["0x0000000000000000000000000000000000000000"],
//                 "fromBlock": "0xf6",
//                 "limit": null, "toBlock": "0x1005",
//                 "topics": [[WITHDRAW_TOPIC], null, null, null]
//             }]),
//             res => json!([]);
//         "eth_blockNumber" =>
//             req => json!([]),
//             res => json!("0x1012");
//         "eth_getLogs" =>
//             req => json!([{
//                 "address":["0x0000000000000000000000000000000000000000"],
//                 "fromBlock": "0x1006",
//                 "limit": null,
//                 "toBlock": "0x1006",
//                 "topics":[[WITHDRAW_TOPIC], null, null, null]
//             }]),
//             res => json!([]);
//     ]
// }
//
// test_app_stream! {
//     name => withdraw_confirm_contract_address,
//     database => Database {
//         checked_withdraw_confirm: 0x00F5,
//         home_contract_address: "49edf201c1e139282643d5e7c6fb0c7219ad1db7".into(),
//         foreign_contract_address: "49edf201c1e139282643d5e7c6fb0c7219ad1db8".into(),
//         ..Database::default()
//     },
//     home =>
//         account => "0000000000000000000000000000000000000001",
//         confirmations => 12;
//     foreign =>
//         account => "0000000000000000000000000000000000000001",
//         confirmations => 12;
//     authorities =>
//         accounts => [
//             "0000000000000000000000000000000000000001",
//             "0000000000000000000000000000000000000002",
//         ],
//         signatures => 1;
//     txs => Transactions::default(),
//     init => |app, db| create_withdraw_confirm(app, db).take(2),
//     expected => vec![0x1005, 0x1006],
//     home_transport => [],
//     foreign_transport => [
//         "eth_blockNumber" =>
//             req => json!([]),
//             res => json!("0x0100");
//         "eth_blockNumber" =>
//             req => json!([]),
//             res => json!("0x1011");
//         "eth_getLogs" =>
//             req => json!([{
//                 "address":["0x49edf201c1e139282643d5e7c6fb0c7219ad1db8"],
//                 "fromBlock": "0xf6",
//                 "limit": null,
//                 "toBlock": "0x1005",
//                 "topics":[[WITHDRAW_TOPIC],null,null,null]
//             }]),
//             res => json!([]);
//         "eth_blockNumber" =>
//             req => json!([]),
//             res => json!("0x1012");
//         "eth_getLogs" =>
//             req => json!([{
//                 "address": ["0x49edf201c1e139282643d5e7c6fb0c7219ad1db8"],
//                 "fromBlock": "0x1006",
//                 "limit": null,
//                 "toBlock": "0x1006",
//                 "topics":[[WITHDRAW_TOPIC],null,null,null]
//             }]),
//             res => json!([]);
//     ]
// }
//
// test_app_stream! {
//     name => withdraw_confirm_payload_gas,
//     database => Database {
//         checked_withdraw_confirm: 0x00F5,
//         home_contract_address: "49edf201c1e139282643d5e7c6fb0c7219ad1db7".into(),
//         foreign_contract_address: "49edf201c1e139282643d5e7c6fb0c7219ad1db8".into(),
//         ..Database::default()
//     },
//     home =>
//         account => "0000000000000000000000000000000000000001",
//         confirmations => 12;
//     foreign =>
//         account => "0000000000000000000000000000000000000001",
//         confirmations => 12;
//     authorities =>
//         accounts => [
//             "00000000000000000000000000000000000000F1",
//             "00000000000000000000000000000000000000F2",
//         ],
//         signatures => 1;
//     txs => Transactions {
//         withdraw_confirm: TransactionConfig {
//             gas: 0xfe,
//             gas_price: 0xa1,
//         },
//         ..Default::default()
//     },
//     init => |app, db| create_withdraw_confirm(app, db).take(2),
//     expected => vec![0x1005, 0x1006],
//     home_transport => [],
//     foreign_transport => [
//         "eth_blockNumber" =>
//             req => json!([]),
//             res => json!("0x0100");
//         "eth_blockNumber" =>
//             req => json!([]),
//             res => json!("0x1011");
//         "eth_getLogs" =>
//             req => json!([{
//                 "address": ["0x49edf201c1e139282643d5e7c6fb0c7219ad1db8"],
//                 "fromBlock": "0xf6",
//                 "limit": null,
//                 "toBlock": "0x1005",
//                 "topics": [[WITHDRAW_TOPIC], null, null, null]
//             }]),
//             res => json!([{
//                 "address": "0x49edf201c1e139282643d5e7c6fb0c7219ad1db8",
//                 "topics": [WITHDRAW_TOPIC],
//                 "data": format!("0x{}", encode(&[
//                     Token::Address([1u8; 20].into()),
//                     Token::Uint(10000.into()),
//                     Token::Uint(1000.into()),
//                 ]).to_hex()),
//                 "type": "",
//                 "transactionHash": "0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"
//             }]);
//         "eth_sign" =>
//             req => json!([
//                 "0x0000000000000000000000000000000000000001",
//                 format!("0x{}", MessageToMainnet {
//                     recipient: [1u8; 20].into(),
//                     value: 10000.into(),
//                     sidenet_transaction_hash: "0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364".into(),
//                     mainnet_gas_price: 1000.into(),
//                 }
//                 .to_bytes()
//                 .to_hex())
//             ]),
//             res => json!("0x8697c15331677e6ebccccaff3454fce5edbc8cca8697c15331677aff3454fce5edbc8cca8697c15331677e6ebccccaff3454fce5edbc8cca8697c15331677e6ebc");
//         // `submitSignature`
//         "eth_sendTransaction" =>
//             req => json!([{
//                 "data": format!("0x{}", contracts::foreign::ForeignBridge::default()
//                     .functions()
//                     .submit_signature()
//                     .input(
//                         "8697c15331677e6ebccccaff3454fce5edbc8cca8697c15331677aff3454fce5edbc8cca8697c15331677e6ebccccaff3454fce5edbc8cca8697c15331677e6ebc".from_hex().unwrap(),
//                         MessageToMainnet {
//                             recipient: [1u8; 20].into(),
//                             value: 10000.into(),
//                             sidenet_transaction_hash: "0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364".into(),
//                             mainnet_gas_price: 1000.into(),
//                         }.to_bytes()
//                     )
//                     .to_hex()),
//                 "from": "0x0000000000000000000000000000000000000001",
//                 "gas": "0xfe",
//                 "gasPrice": "0xa1",
//                 "to":"0x49edf201c1e139282643d5e7c6fb0c7219ad1db8"
//             }]),
//             res => json!("0x1db8f385535c0d178b8f40016048f3a3cffee8f94e68978ea4b277f57b638f0b");
//         "eth_blockNumber" =>
//             req => json!([]),
//             res => json!("0x1012");
//         "eth_getLogs" =>
//             req => json!([{
//                 "address": ["0x49edf201c1e139282643d5e7c6fb0c7219ad1db8"],
//                 "fromBlock": "0x1006",
//                 "limit": null,
//                 "toBlock": "0x1006",
//                 "topics":[[WITHDRAW_TOPIC], null, null, null]
//             }]),
//             res => json!([]);
//     ]
// }
//
// test_app_stream! {
//     name => withdraw_confirm_payload_multiple,
//     database => Database {
//         home_contract_address: "49edf201c1e139282643d5e7c6fb0c7219ad1db7".into(),
//         foreign_contract_address: "49edf201c1e139282643d5e7c6fb0c7219ad1db8".into(),
//         ..Database::default()
//     },
//     home =>
//         account => "0000000000000000000000000000000000000001",
//         confirmations => 12;
//     foreign =>
//         account => "0000000000000000000000000000000000000001",
//         confirmations => 12;
//     authorities =>
//         accounts => [
//             "00000000000000000000000000000000000000F1",
//             "00000000000000000000000000000000000000F2",
//         ],
//         signatures => 1;
//     txs => Transactions {
//         withdraw_confirm: TransactionConfig {
//             gas: 0xff,
//             gas_price: 0xaa,
//         },
//         ..Default::default()
//     },
//     init => |app, db| create_withdraw_confirm(app, db).take(2),
//     expected => vec![0x2, 0x1006],
//     home_transport => [],
//     foreign_transport => [
//         "eth_blockNumber" =>
//             req => json!([]),
//             res => json!("0xe");
//         "eth_getLogs" =>
//             req => json!([{
//                 "address": ["0x49edf201c1e139282643d5e7c6fb0c7219ad1db8"],
//                 "fromBlock": "0x1",
//                 "limit": null,
//                 "toBlock": "0x2",
//                 "topics": [[WITHDRAW_TOPIC], null, null, null]
//             }]),
//             res => json!([{
//                 "address": "0x49edf201c1e139282643d5e7c6fb0c7219ad1db8",
//                 "topics": [WITHDRAW_TOPIC],
//                 "data": format!("0x{}", encode(&[
//                     Token::Address([1u8; 20].into()),
//                     Token::Uint(10000.into()),
//                     Token::Uint(1000.into()),
//                 ]).to_hex()),
//                 "type": "",
//                 "transactionHash": "0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"},
//             {
//                 "address":"0x49edf201c1e139282643d5e7c6fb0c7219ad1db8",
//                 "topics": [WITHDRAW_TOPIC],
//                 "data": format!("0x{}", encode(&[
//                     Token::Address([2u8; 20].into()),
//                     Token::Uint(42.into()),
//                     Token::Uint(100.into()),
//                 ]).to_hex()),
//                 "type":"",
//                 "transactionHash":"0xfffedad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"
//             }]);
//         "eth_sign" =>
//             req => json!([
//                 "0x0000000000000000000000000000000000000001",
//                 format!("0x{}", MessageToMainnet {
//                     recipient: [1u8; 20].into(),
//                     value: 10000.into(),
//                     sidenet_transaction_hash: "0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364".into(),
//                     mainnet_gas_price: 1000.into(),
//                 }
//                 .to_bytes()
//                 .to_hex())
//             ]),
//             res => json!("0x8697c15331677e6ebccccaff3454fce5edbc8cca8697c15331677aff3454fce5edbc8cca8697c15331677e6ebccccaff3454fce5edbc8cca8697c15331677e6ebc");
//         "eth_sign" =>
//             req => json!([
//                 "0x0000000000000000000000000000000000000001",
//                 format!("0x{}", MessageToMainnet {
//                     recipient: [2u8; 20].into(),
//                     value: 42.into(),
//                     sidenet_transaction_hash: "0xfffedad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364".into(),
//                     mainnet_gas_price: 100.into(),
//                 }
//                 .to_bytes()
//                 .to_hex())
//             ]),
//             res => json!("0x8a3b24c56e46f6fc9fa7ed14795745348059b8ac84d6ee93323e83a429e760ae6e89510834ee4d65eefacd74cddca53df61b5eba1c3007ed88d2eebff2e0e2151b");
//         // `submitSignature`
//         "eth_sendTransaction" =>
//             req => json!([{
//                 "data": format!("0x{}", contracts::foreign::ForeignBridge::default()
//                     .functions()
//                     .submit_signature()
//                     .input(
//                         "8697c15331677e6ebccccaff3454fce5edbc8cca8697c15331677aff3454fce5edbc8cca8697c15331677e6ebccccaff3454fce5edbc8cca8697c15331677e6ebc".from_hex().unwrap(),
//                         MessageToMainnet {
//                             recipient: [1u8; 20].into(),
//                             value: 10000.into(),
//                             sidenet_transaction_hash: "0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364".into(),
//                             mainnet_gas_price: 1000.into(),
//                         }.to_bytes()
//                     )
//                     .to_hex()),
//                 "from": "0x0000000000000000000000000000000000000001",
//                 "gas": "0xff",
//                 "gasPrice": "0xaa",
//                 "to":"0x49edf201c1e139282643d5e7c6fb0c7219ad1db8"
//             }]),
//             res => json!("0x1db8f385535c0d178b8f40016048f3a3cffee8f94e68978ea4b277f57b638f0b");
//         // `submitSignature`
//         "eth_sendTransaction" =>
//             req => json!([{
//                 "data": format!("0x{}", contracts::foreign::ForeignBridge::default()
//                     .functions()
//                     .submit_signature()
//                     .input(
//                         "8a3b24c56e46f6fc9fa7ed14795745348059b8ac84d6ee93323e83a429e760ae6e89510834ee4d65eefacd74cddca53df61b5eba1c3007ed88d2eebff2e0e2151b".from_hex().unwrap(),
//                         MessageToMainnet {
//                             recipient: [2u8; 20].into(),
//                             value: 42.into(),
//                             sidenet_transaction_hash: "0xfffedad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364".into(),
//                             mainnet_gas_price: 100.into(),
//                         }.to_bytes()
//                     )
//                     .to_hex()),
//                 "from": "0x0000000000000000000000000000000000000001",
//                 "gas": "0xff",
//                 "gasPrice": "0xaa",
//                 "to":"0x49edf201c1e139282643d5e7c6fb0c7219ad1db8"
//             }]),
//             res => json!("0x1db8f385535c0d178b8f40016048f3a3cffee8f94e68978ea4b277f57b638f0c");
//         "eth_blockNumber" =>
//             req => json!([]),
//             res => json!("0x1012");
//         "eth_getLogs" =>
//             req => json!([{
//                 "address": ["0x49edf201c1e139282643d5e7c6fb0c7219ad1db8"],
//                 "fromBlock": "0x3",
//                 "limit": null,
//                 "toBlock": "0x1006",
//                 "topics":[[WITHDRAW_TOPIC], null, null, null]
//             }]),
//             res => json!([]);
//     ]
// }
