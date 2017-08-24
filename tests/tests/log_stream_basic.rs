extern crate futures;
extern crate web3;
extern crate bridge;
#[macro_use]
extern crate tests;

use std::time::Duration;
use web3::types::FilterBuilder;
use bridge::api::{LogStreamInit, log_stream, LogStreamItem};

test_transport_stream! {
	name => log_stream_basic,
	init => |transport| {
		let init = LogStreamInit {
			after: 10,
			filter: FilterBuilder::default(),
			poll_interval: Duration::from_secs(0),
			confirmations: 10,
		};

		log_stream(transport, init).take(2)
	},
	expected => vec![LogStreamItem {
		from: 0xb,
		to: 0x1006,
		logs: vec![],
	}, LogStreamItem {
		from: 0x1007,
		to: 0x1007,
		logs: vec![],
	}],
	"eth_blockNumber" =>
		req => r#"[]"#,
		res => r#""0x1010""#;
	"eth_getLogs" =>
		req => r#"[{"address":null,"fromBlock":"0xb","limit":null,"toBlock":"0x1006","topics":null}]"#,
		res => r#"[]"#;
	"eth_blockNumber" =>
		req => r#"[]"#,
		res => r#""0x1010""#;
	"eth_blockNumber" =>
		req => r#"[]"#,
		res => r#""0x1011""#;
	"eth_getLogs" =>
		req => r#"[{"address":null,"fromBlock":"0x1007","limit":null,"toBlock":"0x1007","topics":null}]"#,
		res => r#"[]"#;
}
