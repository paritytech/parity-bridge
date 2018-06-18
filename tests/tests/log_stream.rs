extern crate bridge;
extern crate ethereum_types;
extern crate futures;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate tests;
extern crate web3;

use bridge::api::{log_stream, LogStreamInit, LogStreamItem};
use std::time::Duration;
use web3::types::{FilterBuilder, H160, H256, Log};

test_transport_stream! {
    name => log_stream_basic,
    init => |transport| {
        let init = LogStreamInit {
            after: 10,
            filter: FilterBuilder::default(),
            poll_interval: Duration::from_secs(0),
            request_timeout: Duration::from_secs(5),
            confirmations: 10,
        };

        log_stream(transport, Default::default(), init).take(2)
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
        req => json!([]),
        res => json!("0x1010");
    "eth_getLogs" =>
        req => json!([{
            "address": null,
            "fromBlock": "0xb",
            "limit": null,
            "toBlock": "0x1006",
            "topics": null
        }]),
        res => json!([]);
    "eth_blockNumber" =>
        req => json!([]),
        res => json!("0x1010");
    "eth_blockNumber" =>
        req => json!([]),
        res => json!("0x1011");
    "eth_getLogs" =>
        req => json!([{
            "address": null,
            "fromBlock": "0x1007",
            "limit": null,
            "toBlock": "0x1007",
            "topics": null
        }]),
        res => json!([]);
}

test_transport_stream! {
    name => log_stream_rollback,
    init => |transport| {
        let init = LogStreamInit {
            after: 10,
            filter: FilterBuilder::default(),
            poll_interval: Duration::from_secs(0),
            request_timeout: Duration::from_secs(5),
            confirmations: 10,
        };

        log_stream(transport, Default::default(), init).take(2)
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
        req => json!([]),
        res => json!("0x17");
    "eth_getLogs" =>
        req => json!([{
            "address": null,
            "fromBlock": "0xb",
            "limit": null,
            "toBlock": "0xd",
            "topics": null
        }]),
        res => json!([]);
    "eth_blockNumber" =>
        req => json!([]),
        res => json!("0x16");
    "eth_blockNumber" =>
        req => json!([]),
        res => json!("0x17");
    "eth_blockNumber" =>
        req => json!([]),
        res => json!("0x19");
    "eth_getLogs" =>
        req => json!([{
            "address": null,
            "fromBlock": "0xe",
            "limit": null,
            "toBlock": "0xf",
            "topics": null
        }]),
        res => json!([]);
}

test_transport_stream! {
    name => log_stream_rollback_before_init,
    init => |transport| {
        let init = LogStreamInit {
            after: 10,
            filter: FilterBuilder::default(),
            poll_interval: Duration::from_secs(0),
            request_timeout: Duration::from_secs(5),
            confirmations: 10,
        };

        log_stream(transport, Default::default(), init).take(1)
    },
    expected => vec![LogStreamItem {
        from: 0xb,
        to: 0xd,
        logs: vec![],
    }],
    "eth_blockNumber" =>
        req => json!([]),
        res => json!("0x13");
    "eth_blockNumber" =>
        req => json!([]),
        res => json!("0x14");
    "eth_blockNumber" =>
        req => json!([]),
        res => json!("0x17");
    "eth_getLogs" =>
        req => json!([{
            "address": null,
            "fromBlock": "0xb",
            "limit": null,
            "toBlock": "0xd",
            "topics": null
        }]),
        res => json!([]);
}

test_transport_stream! {
    name => log_stream_zero_confirmations,
    init => |transport| {
        let init = LogStreamInit {
            after: 10,
            filter: FilterBuilder::default(),
            poll_interval: Duration::from_secs(0),
            request_timeout: Duration::from_secs(5),
            confirmations: 0,
        };

        log_stream(transport, Default::default(), init).take(3)
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
        req => json!([]),
        res => json!("0x13");
    "eth_getLogs" =>
        req => json!([{
            "address": null,
            "fromBlock": "0xb",
            "limit": null,
            "toBlock": "0x13",
            "topics": null
        }]),
        res => json!([]);
    "eth_blockNumber" =>
        req => json!([]),
        res => json!("0x14");
    "eth_getLogs" =>
        req => json!([{
            "address": null,
            "fromBlock": "0x14",
            "limit": null,
            "toBlock": "0x14",
            "topics": null
        }]),
        res => json!([]);
    "eth_blockNumber" =>
        req => json!([]),
        res => json!("0x14");
    "eth_blockNumber" =>
        req => json!([]),
        res => json!("0x17");
    "eth_getLogs" =>
        req => json!([{
            "address": null,
            "fromBlock": "0x15",
            "limit": null,
            "toBlock": "0x17",
            "topics": null
        }]),
        res => json!([]);
}

test_transport_stream! {
    name => log_stream_filter_with_address,
    init => |transport| {
        let init = LogStreamInit {
            after: 11,
            filter: FilterBuilder::default().address(vec![H160([0x11u8; 20])]),
            poll_interval: Duration::from_secs(0),
            request_timeout: Duration::from_secs(5),
            confirmations: 0,
        };

        log_stream(transport, Default::default(), init).take(2)
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
        req => json!([]),
        res => json!("0x13");
    "eth_getLogs" =>
        req => json!([{
            "address": ["0x1111111111111111111111111111111111111111"],
            "fromBlock": "0xc",
            "limit": null,
            "toBlock": "0x13",
            "topics": null
        }]),
        res => json!([]);
    "eth_blockNumber" =>
        req => json!([]),
        res => json!("0x14");
    "eth_getLogs" =>
        req => json!([{
            "address":["0x1111111111111111111111111111111111111111"],
            "fromBlock": "0x14",
            "limit": null,
            "toBlock": "0x14",
            "topics": null
        }]),
        res => json!([]);
}

test_transport_stream! {
    name => log_stream_filter_with_topics,
    init => |transport| {
        let init = LogStreamInit {
            after: 11,
            filter: FilterBuilder::default().topics(Some(vec![H256([0x22; 32])]), None, None, None),
            poll_interval: Duration::from_secs(0),
            request_timeout: Duration::from_secs(5),
            confirmations: 0,
        };

        log_stream(transport, Default::default(), init).take(2)
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
        req => json!([]),
        res => json!("0x13");
    "eth_getLogs" =>
        req => json!([{
            "address": null,
            "fromBlock": "0xc",
            "limit": null,
            "toBlock": "0x13",
            "topics":[["0x2222222222222222222222222222222222222222222222222222222222222222"], null, null, null]
        }]),
        res => json!([]);
    "eth_blockNumber" =>
        req => json!([]),
        res => json!("0x14");
    "eth_getLogs" =>
        req => json!([{
            "address": null,
            "fromBlock": "0x14",
            "limit": null,
            "toBlock": "0x14",
            "topics": [["0x2222222222222222222222222222222222222222222222222222222222222222"], null, null, null]
        }]),
        res => json!([]);
}

test_transport_stream! {
    name => log_stream_get_log,
    init => |transport| {
        let init = LogStreamInit {
            after: 10,
            filter: FilterBuilder::default(),
            poll_interval: Duration::from_secs(0),
            request_timeout: Duration::from_secs(5),
            confirmations: 10,
        };

        log_stream(transport, Default::default(), init).take(1)
    },
    expected => vec![LogStreamItem {
        from: 0xb,
        to: 0x1006,
        logs: vec![Log {
            address: "0000000000000000000000000000000000000001".into(),
            topics: vec![],
            data: vec![0x10].into(),
            log_type: "".into(),
            ..Default::default()
        }],
    }],
    "eth_blockNumber" =>
        req => json!([]),
        res => json!("0x1010");
    "eth_getLogs" =>
        req => json!([{
            "address": null,
            "fromBlock": "0xb",
            "limit": null,
            "toBlock": "0x1006",
            "topics": null
        }]),
        res => json!([{
            "address": "0x0000000000000000000000000000000000000001",
            "topics": [],
            "data": "0x10",
            "type": ""
        }]);
}

test_transport_stream! {
    name => log_stream_get_multiple_logs,
    init => |transport| {
        let init = LogStreamInit {
            after: 10,
            filter: FilterBuilder::default(),
            poll_interval: Duration::from_secs(0),
            request_timeout: Duration::from_secs(5),
            confirmations: 10,
        };

        log_stream(transport, Default::default(), init).take(3)
    },
    expected => vec![LogStreamItem {
        from: 0xb,
        to: 0x1006,
        logs: vec![Log {
            address: "0000000000000000000000000000000000000001".into(),
            topics: vec![],
            data: vec![0x10].into(),
            log_type: "".into(),
            ..Default::default()
        }],
    }, LogStreamItem {
        from: 0x1007,
        to: 0x1007,
        logs: vec![],
    }, LogStreamItem {
        from: 0x1008,
        to: 0x1008,
        logs: vec![Log {
            address: "0000000000000000000000000000000000000002".into(),
            topics: vec![],
            data: vec![0x20].into(),
            log_type: "".into(),
            ..Default::default()
        }, Log {
            address: "0000000000000000000000000000000000000002".into(),
            topics: vec![],
            data: vec![0x30].into(),
            log_type: "".into(),
            ..Default::default()
        }],
    }],
    "eth_blockNumber" =>
        req => json!([]),
        res => json!("0x1010");
    "eth_getLogs" =>
        req => json!([{
            "address": null,
            "fromBlock": "0xb",
            "limit": null,
            "toBlock": "0x1006",
            "topics": null
        }]),
        res => json!([{
            "address": "0x0000000000000000000000000000000000000001",
            "topics": [],
            "data": "0x10",
            "type": ""
        }]);
    "eth_blockNumber" =>
        req => json!([]),
        res => json!("0x1011");
    "eth_getLogs" =>
        req => json!([{
            "address": null,
            "fromBlock": "0x1007",
            "limit": null,
            "toBlock": "0x1007",
            "topics": null
        }]),
        res => json!([]);
    "eth_blockNumber" =>
        req => json!([]),
        res => json!("0x1012");
    "eth_getLogs" =>
        req => json!([{
            "address": null,
            "fromBlock": "0x1008",
            "limit": null,
            "toBlock": "0x1008",
            "topics": null
        }]),
        res => json!([
            {
                "address": "0x0000000000000000000000000000000000000002",
                "topics": [],
                "data": "0x20",
                "type":""
            },
            {
                "address":"0x0000000000000000000000000000000000000002",
                "topics": [],
                "data": "0x30",
                "type": ""
            }
        ]);
}
