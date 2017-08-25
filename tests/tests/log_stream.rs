extern crate futures;
extern crate web3;
extern crate bridge;
#[macro_use]
extern crate tests;

use std::time::Duration;
use web3::types::{FilterBuilder, H160, H256};
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

test_transport_stream! {
	name => log_stream_rollback,
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
		to: 0xd,
		logs: vec![],
	}, LogStreamItem {
		from: 0xe,
		to: 0xf,
		logs: vec![],
	}],
	"eth_blockNumber" =>
		req => r#"[]"#,
		res => r#""0x17""#;
	"eth_getLogs" =>
		req => r#"[{"address":null,"fromBlock":"0xb","limit":null,"toBlock":"0xd","topics":null}]"#,
		res => r#"[]"#;
	"eth_blockNumber" =>
		req => r#"[]"#,
		res => r#""0x16""#;
	"eth_blockNumber" =>
		req => r#"[]"#,
		res => r#""0x17""#;
	"eth_blockNumber" =>
		req => r#"[]"#,
		res => r#""0x19""#;
	"eth_getLogs" =>
		req => r#"[{"address":null,"fromBlock":"0xe","limit":null,"toBlock":"0xf","topics":null}]"#,
		res => r#"[]"#;
}

test_transport_stream! {
	name => log_stream_rollback_before_init,
	init => |transport| {
		let init = LogStreamInit {
			after: 10,
			filter: FilterBuilder::default(),
			poll_interval: Duration::from_secs(0),
			confirmations: 10,
		};

		log_stream(transport, init).take(1)
	},
	expected => vec![LogStreamItem {
		from: 0xb,
		to: 0xd,
		logs: vec![],
	}],
	"eth_blockNumber" =>
		req => r#"[]"#,
		res => r#""0x13""#;
	"eth_blockNumber" =>
		req => r#"[]"#,
		res => r#""0x14""#;
	"eth_blockNumber" =>
		req => r#"[]"#,
		res => r#""0x17""#;
	"eth_getLogs" =>
		req => r#"[{"address":null,"fromBlock":"0xb","limit":null,"toBlock":"0xd","topics":null}]"#,
		res => r#"[]"#;
}

test_transport_stream! {
	name => log_stream_zero_confirmations,
	init => |transport| {
		let init = LogStreamInit {
			after: 10,
			filter: FilterBuilder::default(),
			poll_interval: Duration::from_secs(0),
			confirmations: 0,
		};

		log_stream(transport, init).take(3)
	},
	expected => vec![LogStreamItem {
		from: 0xb,
		to: 0x13,
		logs: vec![],
	}, LogStreamItem {
		from: 0x14,
		to: 0x14,
		logs: vec![],
	}, LogStreamItem {
		from: 0x15,
		to: 0x17,
		logs: vec![],
	}],
	"eth_blockNumber" =>
		req => r#"[]"#,
		res => r#""0x13""#;
	"eth_getLogs" =>
		req => r#"[{"address":null,"fromBlock":"0xb","limit":null,"toBlock":"0x13","topics":null}]"#,
		res => r#"[]"#;
	"eth_blockNumber" =>
		req => r#"[]"#,
		res => r#""0x14""#;
	"eth_getLogs" =>
		req => r#"[{"address":null,"fromBlock":"0x14","limit":null,"toBlock":"0x14","topics":null}]"#,
		res => r#"[]"#;
	"eth_blockNumber" =>
		req => r#"[]"#,
		res => r#""0x14""#;
	"eth_blockNumber" =>
		req => r#"[]"#,
		res => r#""0x17""#;
	"eth_getLogs" =>
		req => r#"[{"address":null,"fromBlock":"0x15","limit":null,"toBlock":"0x17","topics":null}]"#,
		res => r#"[]"#;
}

test_transport_stream! {
	name => log_stream_filter_with_address,
	init => |transport| {
		let init = LogStreamInit {
			after: 11,
			filter: FilterBuilder::default().address(vec![H160([0x11u8; 20])]),
			poll_interval: Duration::from_secs(0),
			confirmations: 0,
		};

		log_stream(transport, init).take(2)
	},
	expected => vec![LogStreamItem {
		from: 0xc,
		to: 0x13,
		logs: vec![],
	}, LogStreamItem {
		from: 0x14,
		to: 0x14,
		logs: vec![],
	}],
	"eth_blockNumber" =>
		req => r#"[]"#,
		res => r#""0x13""#;
	"eth_getLogs" =>
		req => r#"[{"address":["0x1111111111111111111111111111111111111111"],"fromBlock":"0xc","limit":null,"toBlock":"0x13","topics":null}]"#,
		res => r#"[]"#;
	"eth_blockNumber" =>
		req => r#"[]"#,
		res => r#""0x14""#;
	"eth_getLogs" =>
		req => r#"[{"address":["0x1111111111111111111111111111111111111111"],"fromBlock":"0x14","limit":null,"toBlock":"0x14","topics":null}]"#,
		res => r#"[]"#;
}

test_transport_stream! {
	name => log_stream_filter_with_topics,
	init => |transport| {
		let init = LogStreamInit {
			after: 11,
			filter: FilterBuilder::default().topics(Some(vec![H256([0x22; 32])]), None, None, None),
			poll_interval: Duration::from_secs(0),
			confirmations: 0,
		};

		log_stream(transport, init).take(2)
	},
	expected => vec![LogStreamItem {
		from: 0xc,
		to: 0x13,
		logs: vec![],
	}, LogStreamItem {
		from: 0x14,
		to: 0x14,
		logs: vec![],
	}],
	"eth_blockNumber" =>
		req => r#"[]"#,
		res => r#""0x13""#;
	"eth_getLogs" =>
		req => r#"[{"address":null,"fromBlock":"0xc","limit":null,"toBlock":"0x13","topics":[["0x2222222222222222222222222222222222222222222222222222222222222222"],null,null,null]}]"#,
		res => r#"[]"#;
	"eth_blockNumber" =>
		req => r#"[]"#,
		res => r#""0x14""#;
	"eth_getLogs" =>
		req => r#"[{"address":null,"fromBlock":"0x14","limit":null,"toBlock":"0x14","topics":[["0x2222222222222222222222222222222222222222222222222222222222222222"],null,null,null]}]"#,
		res => r#"[]"#;
}
