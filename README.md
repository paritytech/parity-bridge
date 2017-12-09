# bridge

[![Build Status][travis-image]][travis-url]

[travis-image]: https://travis-ci.org/paritytech/parity-bridge.svg?branch=master
[travis-url]: https://travis-ci.org/paritytech/parity-bridge

Simple bridge between ValidatorSet-based parity chain (foreign) with any other Parity chain (home).

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
[home]
account = "0x006e27b6a72e1f34c626762f3c4761547aff1421"
ipc = "/Users/marek/Library/Application Support/io.parity.ethereum/jsonrpc.ipc"
required_confirmations = 0

[home.contract]
bin = "contracts/EthereumBridge.bin"

[foreign]
account = "0x006e27b6a72e1f34c626762f3c4761547aff1421"
ipc = "/Users/marek/Library/Application Support/io.parity.ethereum/jsonrpc.ipc"
required_confirmations = 0

[foreign.contract]
bin = "contracts/KovanBridge.bin"

[authorities]
accounts = [
	"0x006e27b6a72e1f34c626762f3c4761547aff1421",
	"0x006e27b6a72e1f34c626762f3c4761547aff1421",
	"0x006e27b6a72e1f34c626762f3c4761547aff1421"
]
required_signatures = 2

[transactions]
home_deploy = { gas = 500000 }
foreign_deploy = { gas = 500000 }
```

#### home options

- `home.account` - authority address on the home (**required**)
- `home.ipc` - path to home parity ipc handle (**required**)
- `home.contract.bin` - path to the compiled bridge contract (**required**)
- `home.required_confirmations` - number of confirmation required to consider transaction final on home (default: **12**)
- `home.poll_interval` - specify how often home node should be polled for changes (in seconds, default: **1**)
- `home.request_timeout` - specify request timeout (in seconds, default: **5**)

#### foreign options

- `foreign.account` - authority address on the foreign (**required**)
- `foreign.ipc` - path to foreign parity ipc handle (**required**)
- `foreign.contract.bin` - path to the compiled bridge contract (**required**)
- `foreign.required_confirmations` - number of confirmation required to consider transaction final on foreign (default: **12**)
- `foreign.poll_interval` - specify how often home node should be polled for changes (in seconds, default: **1**)
- `foreign.request_timeout` - specify request timeout (in seconds, default: **5**)


#### authorities options

- `authorities.account` - all authorities (**required**)
- `authorities.required_signatures` - number of authorities signatures required to consider action final (**required**)

#### transaction options

- `transaction.home_deploy.gas` - specify how much gas should be consumed by home contract deploy
- `transaction.home_deploy.gas_price` - specify gas price for home contract deploy
- `transaction.foreign_deploy.gas` - specify how much gas should be consumed by foreign contract deploy
- `transaction.foreign_deploy.gas_price` - specify gas price for foreign contract deploy
- `transaction.deposit_relay.gas` - specify how much gas should be consumed by deposit relay
- `transaction.deposit_relay.gas_price` - specify gas price for deposit relay
- `transaction.withdraw_confirm.gas` - specify how much gas should be consumed by withdraw confirm
- `transaction.withdraw_confirm.gas_price` - specify gas price for withdraw confirm
- `transaction.withdraw_relay.gas` - specify how much gas should be consumed by withdraw relay
- `transaction.withdraw_relay.gas_price` - specify gas price for withdraw relay

### database file format

```toml
home_contract_address = "0x49edf201c1e139282643d5e7c6fb0c7219ad1db7"
foreign_contract_address = "0x49edf201c1e139282643d5e7c6fb0c7219ad1db8"
home_deploy = 100
foreign_deploy = 101
checked_deposit_relay = 120
checked_withdraw_relay = 121
checked_withdraw_confirm = 121
```

**all fields are required**

- `home_contract_address` - address of the bridge contract on home chain
- `foreign_contract_address` - address of the bridge contract on foreign chain
- `home_deploy` - block number at which home contract has been deployed
- `foreign_deploy` - block number at which foreign contract has been deployed
- `checked_deposit_relay` - number of the last block for which an authority has relayed deposits to the foreign
- `checked_withdraw_relay` - number of the last block for which an authority has relayed withdraws to the home
- `checked_withdraw_confirm` - number of the last block for which an authirty has confirmed withdraw

### example run

```
./target/debug/bridge --config examples/config.toml --database db.toml
```

- example run requires a parity instance running
- this parity instance can be started by running `examples/parity_start.sh`
- it connects to this parity instance twice. one connection treats the node as `home`, whereas the other as `foreign`
- by default, parity tries to unlock account generates from seedphrase `this is sparta` - `0x006e27b6a72e1f34c626762f3c4761547aff1421`
- this is just an example. the 'real world' bridge needs to connect to the two different parity instances

### deposit

![deposit](./res/deposit.png)

### withdraw

![withdraw](./res/withdraw.png)
