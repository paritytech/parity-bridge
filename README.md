# bridge

[![Build Status][travis-image]][travis-url]

[travis-image]: https://travis-ci.org/paritytech/parity-bridge.svg?branch=master
[travis-url]: https://travis-ci.org/paritytech/parity-bridge

simple bridge between ethereum foundation and kovan networks

### build

```
cargo build -p bridge-cli --release
```

### cli options

```
Ethereum-Kovan bridge.
    Copyright 2017 Parity Technologies (UK) Limited

Usage:
    bridge --config <config> --database <database>
    bridge -h | --help

Options:
    -h, --help           Display help message and exit.
```

- `--config` - location of the configuration file. configuration file must exist
- `--database` - location of the database file. if there is no file at specified location, new bridge contracts will be deployed and new database will be created

### configuration [file example](./examples/config.toml)

```toml
[mainnet]
account = "0x006e27b6a72e1f34c626762f3c4761547aff1421"
ipc = "/Users/marek/Library/Application Support/io.parity.ethereum/jsonrpc.ipc"
required_confirmations = 0

[mainnet.contract]
bin = "contracts/EthereumBridge.bin"

[testnet]
account = "0x006e27b6a72e1f34c626762f3c4761547aff1421"
ipc = "/Users/marek/Library/Application Support/io.parity.ethereum/jsonrpc.ipc"
required_confirmations = 0

[testnet.contract]
bin = "contracts/KovanBridge.bin"

[authorities]
accounts = [
	"0x006e27b6a72e1f34c626762f3c4761547aff1421",
	"0x006e27b6a72e1f34c626762f3c4761547aff1421",
	"0x006e27b6a72e1f34c626762f3c4761547aff1421"
]
required_signatures = 2

[transactions]
mainnet_deploy = { gas = 500000 }
testnet_deploy = { gas = 500000 }
```

#### mainnet options

- `mainnet.account` - authority address on the mainnet (**required**)
- `mainnet.ipc` - path to mainnet parity ipc handle (**required**)
- `mainnet.contract.bin` - path to the compiled bridge contract (**required**)
- `mainnet.required_confirmations` - number of confirmation required to consider transaction final on mainnet (default: **12**)
- `mainnet.poll_interval` - specify how often mainnet node should be polled for changes (in seconds, default: **1**)
- `mainnet.request_timeout` - specify request timeout (in seconds, default: **5**)

#### testnet options

- `testnet.account` - authority address on the testnet (**required**)
- `testnet.ipc` - path to testnet parity ipc handle (**required**)
- `testnet.contract.bin` - path to the compiled bridge contract (**required**)
- `testnet.required_confirmations` - number of confirmation required to consider transaction final on testnte (default: **12**)
- `testnet.poll_interval` - specify how often mainnet node should be polled for changes (in seconds, default: **1**)
- `testnet.request_timeout` - specify request timeout (in seconds, default: **5**)


#### authorities options

- `authorities.account` - all authorities (**required**)
- `authorities.required_signatures` - number of authorities signatures required to consider action final (**required**)

#### transaction options

- `transaction.mainnet_deploy.gas` - specify how much gas should be consumed by mainnet contract deploy
- `transaction.mainnet_deploy.gas_price` - specify gas price for mainnet contract deploy
- `transaction.testnet_deploy.gas` - specify how much gas should be consumed by testnet contract deploy
- `transaction.testnet_deploy.gas_price` - specify gas price for mainnet contract deploy
- `transaction.deposit_relay.gas` - specify how much gas should be consumed by deposit relay
- `transaction.deposit_relay.gas_price` - specify gas price for deposit relay
- `transaction.withdraw_confirm.gas` - specify how much gas should be consumed by withdraw confirm
- `transaction.withdraw_confirm.gas_price` - specify gas price for withdraw confirm
- `transaction.withdraw_relay.gas` - specify how much gas should be consumed by withdraw relay
- `transaction.withdraw_relay.gas_price` - specify gas price for withdraw relay

### database file format

```toml
mainnet_contract_address = "0x49edf201c1e139282643d5e7c6fb0c7219ad1db7"
testnet_contract_address = "0x49edf201c1e139282643d5e7c6fb0c7219ad1db8"
mainnet_deploy = 100
testnet_deploy = 101
checked_deposit_relay = 120
checked_withdraw_relay = 121
checked_withdraw_confirm = 121
```

**all fields are required**

- `mainnet_contract_address` - address of the bridge contract on mainnet
- `testnet_contract_address` - address of the bridge contract on testnet
- `mainnet_deploy` - block number at which mainnet contract has been deployed
- `testnet_deploy` - block number at which testnet contract has been deployed
- `checked_deposit_relay` - number of the last block for which an authority has relayed deposits to the testnet
- `checked_withdraw_relay` - number of the last block for which an authority has relayed withdraws to the mainnet
- `checked_withdraw_confirm` - number of the last block for which an authirty has confirmed withdraw 

### example run

// TODO

### deposit

![deposit](./res/deposit.png)

### withdraw

![withdraw](./res/withdraw.png)
