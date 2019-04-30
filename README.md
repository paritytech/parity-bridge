# bridge

**DISCLAIMER:** *we recommend not using the bridge in "production" (to bridge significant amounts) just yet.
it's missing a code audit and should still be considered alpha. we can't rule out that there are bugs that might result in loss of the bridged amounts.
we'll update this disclaimer once that changes*

[![Join the chat at https://gitter.im/paritytech/parity-bridge](https://badges.gitter.im/paritytech/parity-bridge.svg)](https://gitter.im/paritytech/parity-bridge?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)

[![Build Status][travis-image]][travis-url]
[![Solidity Coverage Status][coveralls-image]][coveralls-url] (contracts only)

[travis-image]: https://travis-ci.org/paritytech/parity-bridge.svg?branch=master
[travis-url]: https://travis-ci.org/paritytech/parity-bridge
[coveralls-image]: https://coveralls.io/repos/github/paritytech/parity-bridge/badge.svg?branch=master
[coveralls-url]: https://coveralls.io/github/paritytech/parity-bridge?branch=master

- bridge is able to pass arbitrary messages between two ethereum-based blockchains.

developers can create bridge extensions to send and receive messages on either blockchain.

**the bridge can mitigate scaling issues:**
by deploying a [proof-of-authority](https://paritytech.github.io/wiki/Proof-of-Authority-Chains.html)
network and bridging it to the Ethereum Foundation network ('mainnet') users can pass arbitrary messages between the chains, which can be used to pass information to contracts on foreign chains with much lower transaction fees,
faster block times, that are unaffected by mainnet congestion.

Users can pass messages back to the mainnet chain at any time over the same bridge.

parity is using the bridge project to prototype
the system that will eventually connect ethereum and other non-parachains to
[polkadot](https://polkadot.io/).

### current functionality

the bridge connects two chains `main` and `side`.

when users send messages to the [`Main` or `Side`](https://github.com/parity-contracts/bridge/blob/4fb552894bfc09cbe69dfcf2bca16d2d8b393a0a/contracts/bridge.sol#L161) contracts on `main` or `side` respectively, using `relayMessage(bytes calldata data, address recipient)`,
the data is passed to any [`BridgeRecipient`](https://github.com/parity-contracts/bridge/blob/4fb552894bfc09cbe69dfcf2bca16d2d8b393a0a/contracts/bridge.sol#L31) contract on the other chain that may process it as it wishes.


`side` is assumed to use PoA (proof of authority) consensus.
relays between the chains happen in a byzantine fault tolerant way using the authorities of `side`.

### high level explanation of main ether -> side ERC20 relay

`sender` calls `Main.relayMessage(data, recipient)`.
the `relayMessage` function emits `RelayMessage( messageID, sender, recipient)`.

for each `relayMessage` event on `Main` every authority executes
`Side.acceptMessage(transactionHash, data, sender, recipient)`.

once there are `Side.requiredSignatures` such transactions
with identical arguments and from distinct authorities then `AcceptedMessage(messageID, sender, recipient)` is emitted and `acceptMessage(data, sender)` is called on `recipient`.

### high level explanation of side ERC20 -> main ether relay

`sender` executes `Side.relayMessage(data, recipient)`
which emits `Side.RelayMessage(messageID, sender, recipient)`.

for every `Side.RelayMessage`, every bridge authority creates a message containing the `transactionHash`, `message_id`, `sender`, and `recipient` of the transaction referenced by the `Side.RelayMessage` event;
signs that message and executes `Side.submitSignedMessage(signature, message)`.
this collection of signatures is on `side` because transactions are free for the authorities on `side`,
but not free on `main`.

once `Side.requiredSignatures` signatures by distinct authorities are collected
a `Side.SignedMessage(authorityThatSubmittedLastSignature, messageHash)` event is emitted.

everyone (usually `authorityThatSubmittedLastSignature`) can then call `Side.message(messageHash)` and
`Side.signature(messageHash, 0..requiredSignatures)`
to look up the message and signatures and execute `Main.acceptMessage(vs, rs, ss, transactionHash, data, sender, recipient)`
and relay the message.

`Main.acceptMessage` recovers the addresses from the signatures,
checks that enough authorities in its authority list have signed and
finally `AcceptedMessage(messageID, sender, recipient)` is emitted and `acceptMessage(data, sender)` is called on `recipient`.

### run truffle smart contract tests

requires `yarn` to be `$PATH`. [installation instructions](https://yarnpkg.com/lang/en/docs/install/)

```
cd truffle
yarn test
```

### build

requires `rust` and `cargo`: [installation instructions.](https://www.rust-lang.org/en-US/install.html)

requires `solc`: [installation instructions.](https://solidity.readthedocs.io/en/develop/installing-solidity.html)

assuming you've cloned the bridge (`git clone git@github.com:paritytech/parity-bridge.git`)
and are in the project directory (`cd parity-bridge`) run:

```
cargo build -p parity-bridge -p parity-bridge-deploy --release
```

to install, copy `target/release/parity-bridge` and `target/release/parity-bridge-deploy` into a folder that's in your `$PATH`.

### configuration

the bridge is configured through a configuration file.

here's an example configuration file: [integration-tests/bridge_config.toml](integration-tests/bridge_config.toml)

following is a detailed explanation of all config options.
all fields are required unless marked with *optional*.

#### options

- `address` - address of this bridge authority on `main` and `side` chain

#### main options

- `main.http` - path to the http socket of a parity node that has `main.account` unlocked
- `main.contract.bin` - path to the compiled `Main` contract
    - required for initial deployment
    - run [tools/compile_contracts.sh](tools/compile_contracts.sh) to compile contracts into dir `compiled_contracts`
    - then set this to `compiled_contracts/Main.bin`
- `main.required_confirmations` - number of confirmations required to consider transaction final on `main.http`
  - *optional,* default: **12**
- `main.poll_interval` - specify how frequently (seconds) `main.http` should be polled for changes
  - *optional,* default: **1**
- `main.request_timeout` - how many seconds to wait for responses from `main.http` before timing out
  - *optional,* default: **5**

#### side options

- `side.http` - path to the http socket of a parity node that has `side.account` unlocked
- `side.contract.bin` - path to the compiled `Side` contract
    - required for initial deployment
    - run [tools/compile_contracts.sh](tools/compile_contracts.sh) to compile contracts into dir `compiled_contracts`
    - then set this to `compiled_contracts/Side.bin`
- `side.required_confirmations` - number of confirmations required to consider transaction final on `side.http`
  - *optional,* default: **12**
- `side.poll_interval` - specify how frequently (seconds) `side.http` should be polled for changes
  - *optional,* default: **1**
- `side.request_timeout` - how many seconds to wait for responses from `side.http` before timing out
  - *optional,* default: **5**

#### authorities options

- `authorities.account` - array of addresses of authorities
- `authorities.required_signatures` - number of authorities signatures required to consider action final

#### transaction options

`gas` and `gas_price` to use for the specific transactions.
these are all **optional** and default to `0`.

look into the `[transactions]` section in [integration-tests/bridge_config.toml](integration-tests/bridge_config.toml)
for recommendations on provided `gas`.

##### these happen on `main`:

- `transaction.main_deploy.gas`
- `transaction.main_deploy.gas_price`
- `transaction.withdraw_relay.gas`
- `transaction.withdraw_relay.gas_price`

##### these happen on `side`:

- `transaction.side_deploy.gas`
- `transaction.side_deploy.gas_price`
- `transaction.deposit_relay.gas`
- `transaction.deposit_relay.gas_price`
- `transaction.withdraw_confirm.gas`
- `transaction.withdraw_confirm.gas_price`

### database file format

```toml
main_contract_address = "0x49edf201c1e139282643d5e7c6fb0c7219ad1db7"
side_contract_address = "0x49edf201c1e139282643d5e7c6fb0c7219ad1db8"
main_deployed_at_block = 100
side_deployed_at_block = 101
last_main_to_side_sign_at_block = 121
last_side_to_main_signatures_at_block = 122
last_side_to_main_sign_at_block = 122
```

**all fields are required**

- `main_contract_address` - address of the bridge contract on main chain
- `side_contract_address` - address of the bridge contract on side chain
- `main_deployed_at_block` - block number at which main contract has been deployed
- `side_deployed_at_block` - block number at which side contract has been deployed
- `last_main_to_side_sign_at_block` - number of the last block for which an authority has relayed signatures to the side
- `last_side_to_main_signatures_at_block` - number of the last block for which an authority has relayed signatures to the main
- `last_side_to_main_sign_at_block` - number of the last block for which an authority has confirmed messages relayed to main

### deployment and run

[read our deployment guide](deployment_guide.md)

### deposit

![deposit](./res/deposit.png)

### withdraw

![withdraw](./res/withdraw.png)


### considerations for relaying messages to main

a bridge `authority` has to pay for gas (`cost`) to execute `Main.acceptMessage` when
sending a message from the `side` chain to the `main` chain. When creating `BridgeRecipient`s, it is prudent to keep this cost in mind.

parity-bridge connects a value-bearing ethereum blockchain `main`
(initially the ethereum foundation chain)
to a non-value-bearing PoA ethereum blockchain `side` (initially the kovan testnet).

value-bearing means that the ether on that chain has usable value in the sense that
in order to obtain it one has to either mine it (trade in electricity)
or trade in another currency.
non-value-bearing means that one can easily obtain a large amount of ether
on that chain for free.
through a faucet in the case of testnets for example.

the bridge authorities *should also be* the validators of the `side` PoA chain.
transactions by the authorities are therefore free (gas price = 0) on `side`.

to execute a transaction on `main` a bridge authority has to spend ether to
pay for the gas.

this opens up an attack where a malicious user could spam `Side.relayMessage`.
It would cost the attacker no `main` chain wei and essentially
free `side` testnet wei to cause the authorities to spend significant amounts of wei
to relay the message to `main` by executing `Main.acceptMessage`.
an attacker is able to exhaust bridge authorities funds on `main`.

To shut down this attack, a whitelist of approved `recipient`s should be employed for `main`.

Another method that may be used to mitigate potential abuse of authorities on `main` is to encourage users of the bridge to call `Main.acceptMessage` themselves (by collecting the message and its signatures from `side`) spending their own gas, instead of mandating that validators spend their gas.
