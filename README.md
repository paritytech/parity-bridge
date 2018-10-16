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

parity-bridge is currently an
[ERC20 token](https://github.com/ethereum/EIPs/blob/master/EIPS/eip-20.md)
contract on one ethereum-based blockchain that is backed by ether on **another** ethereum-based blockchain.

eventually parity-bridge will be able to pass arbitrary messages between
two ethereum-based blockchains.
in the future you'll be able to build the current ether-ERC20 bridge and any other
cross-chain application on top of the message passing bridge.

currently users can convert ether
on one chain into the same amount of ERC20 tokens on the other and back.
the bridge securely relays these conversions.

**the bridge can mitigate scaling issues:**
by deploying a [proof-of-authority](https://paritytech.github.io/wiki/Proof-of-Authority-Chains.html)
network and bridging it to the Ethereum Foundation network ('mainnet') users can convert their mainnet ether
into ERC20 tokens on the PoA chain
and there transfer them with much lower transaction fees,
faster block times and unaffected by mainnet congestion.

the users can withdraw their tokens worth of ether on the mainnet at any point.

parity is using the bridge project to prototype
the system that will eventually connect ethereum and other non-parachains to
[polkadot](https://polkadot.io/).

### current functionality

the bridge connects two chains `main` and `side`.

when users deposit ether into the `MainBridge` contract on `main`
they get the same amount of ERC20 tokens on `side`.

[they can use `SideBridge` as they would use any ERC20 token.](https://github.com/ethereum/EIPs/blob/master/EIPS/eip-20.md)

to convert their `side` ERC20 into ether on `main`
users can always call `SideBridge.transferMainViaRelay(mainRecipientAddress, value, mainGasPrice)`.

`side` is assumed to use PoA (proof of authority) consensus.
relays between the chains happen in a byzantine fault tolerant way using the authorities of `side`.

### high level explanation of main ether -> side ERC20 relay

`sender` deposits `value` into `MainBridge`.
the `MainBridge` fallback function emits `Deposit(sender, value)`.

for each `Deposit` event on `MainBridge` every authority executes
`SideBridge.deposit(sender, value, transactionHash)`.

once there are `SideBridge.requiredSignatures` such transactions
with identical arguments and from distinct authorities then
`SideBridge.balanceOf(sender)` is increased by `value`.

### high level explanation of side ERC20 -> main ether relay

`sender` executes `SideBridge.transferMainViaRelay(recipient, value, mainGasPrice)`
which checks and reduces `SideBridge.balances(sender)` by `value` and emits `SideBridge.Withdraw(recipient, value, mainGasPrice)`.

for every `SideBridge.Withdraw`, every bridge authority creates a message containing
`value`, `recipient` and the `transactionHash` of the transaction referenced by the `SideBridge.Withdraw` event;
signs that message and executes `SideBridge.submitSignature(signature, message)`.
this collection of signatures is on `side` because transactions are free for the authorities on `side`,
but not free on `main`.

once `SideBridge.requiredSignatures` signatures by distinct authorities are collected
a `SideBridge.CollectedSignatures(authorityThatSubmittedLastSignature, messageHash)` event is emitted.

everyone (usually `authorityThatSubmittedLastSignature`) can then call `SideBridge.message(messageHash)` and
`SideBridge.signature(messageHash, 0..requiredSignatures)`
to look up the message and signatures and execute `MainBridge.withdraw(vs, rs, ss, message)`
and complete the withdraw.

`MainBridge.withdraw(vs, rs, ss, message)` recovers the addresses from the signatures,
checks that enough authorities in its authority list have signed and
finally transfers `value` ether ([minus the relay gas costs](#recipient-pays-relay-cost-to-relaying-authority))
to `recipient`.

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
- `estimated_gas_cost_of_withdraw` - an upper bound on the gas a transaction to `MainBridge.withdraw` consumes
  - currently recommended value: `"200000"`
  - must be a string because the `toml` crate can't parse numbers greater max i64
  - run [tools/estimate_gas_costs.sh](tools/estimate_gas_costs.sh) to compute an estimate
  - see [recipient pays relay cost to relaying authority](#recipient-pays-relay-cost-to-relaying-authority) for why this config option is needed
- `max_total_main_contract_balance` - reject deposits that would increase `MainBridge.balance` beyond this value
  - security feature:
    - limits the total amount of main/mainnet ether that can be lost
      if the bridge is faulty or compromised in any way!
  - set to `"0"` to disable.
  - recommended for test deployment: 10 ether = `"10000000000000000000"`
  - must be a string because the `toml` crate can't parse numbers greater max i64
    and this value frequently is greater
- `max_single_deposit_value` - reject deposits whose `msg.value` is higher than this value
  - security feature
  - set to 0 to disable
  - recommended for test deployment: 1 ether = `"1000000000000000000"`
  - must be a string because the `toml` crate can't parse numbers greater max i64
    and this value frequently is greater

#### main options

- `main.http` - path to the http socket of a parity node that has `main.account` unlocked
- `main.contract.bin` - path to the compiled `MainBridge` contract
    - required for initial deployment
    - run [tools/compile_contracts.sh](tools/compile_contracts.sh) to compile contracts into dir `compiled_contracts`
    - then set this to `compiled_contracts/MainBridge.bin`
- `main.required_confirmations` - number of confirmations required to consider transaction final on `main.http`
  - *optional,* default: **12**
- `main.poll_interval` - specify how frequently (seconds) `main.http` should be polled for changes
  - *optional,* default: **1**
- `main.request_timeout` - how many seconds to wait for responses from `main.http` before timing out
  - *optional,* default: **5**

#### side options

- `side.http` - path to the http socket of a parity node that has `side.account` unlocked
- `side.contract.bin` - path to the compiled `SideBridge` contract
    - required for initial deployment
    - run [tools/compile_contracts.sh](tools/compile_contracts.sh) to compile contracts into dir `compiled_contracts`
    - then set this to `compiled_contracts/SideBridge.bin`
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
main_deploy = 100
side_deploy = 101
checked_deposit_relay = 120
checked_withdraw_relay = 121
checked_withdraw_confirm = 121
```

**all fields are required**

- `main_contract_address` - address of the bridge contract on main chain
- `side_contract_address` - address of the bridge contract on side chain
- `main_deploy` - block number at which main contract has been deployed
- `side_deploy` - block number at which side contract has been deployed
- `checked_deposit_relay` - number of the last block for which an authority has relayed deposits to the side
- `checked_withdraw_relay` - number of the last block for which an authority has relayed withdraws to the main
- `checked_withdraw_confirm` - number of the last block for which an authority has confirmed withdraw

### deployment and run

[read our deployment guide](deployment_guide.md)

### deposit

![deposit](./res/deposit.png)

### withdraw

![withdraw](./res/withdraw.png)

### recipient pays relay cost to relaying authority

a bridge `authority` has to pay for gas (`cost`) to execute `MainBridge.withdraw` when
withdrawing `value` from `side` chain to `main` chain.
`value - cost` is transferred to the `recipient`. `cost` is transferred to the `authority`
executing `MainBridge.withdraw`.
the `recipient` pays the relaying `authority` for the execution of the transaction.
that shuts down an attack that enabled exhaustion of authorities funds on `main`.

read on for a more thorough explanation.

parity-bridge connects a value-bearing ethereum blockchain `main`
(initially the ethereum foundation chain)
to a non-value-bearing PoA ethereum blockchain `side` (initially the kovan testnet).

value-bearing means that the ether on that chain has usable value in the sense that
in order to obtain it one has to either mine it (trade in electricity)
or trade in another currency.
non-value-bearing means that one can easily obtain a large amount of ether
on that chain for free.
through a faucet in the case of testnets for example.

the bridge authorities are also the validators of the `side` PoA chain.
transactions by the authorities are therefore free (gas price = 0) on `side`.

to execute a transaction on `main` a bridge authority has to spend ether to
pay for the gas.

this opened up an attack where a malicious user could
deposit a very small amount of wei on `MainBridge`, get it relayed to `SideBridge`,
then spam `SideBridge.transferMainViaRelay` with `1` wei withdraws.
it would cost the attacker very little `main` chain wei and essentially
free `side` testnet wei to cause the authorities to spend orders of magnitude more wei
to relay the withdraw to `main` by executing `MainBridge.withdraw`.
an attacker was able to exhaust bridge authorities funds on `main`.

to shut down this attack `MainBridge.withdraw` was modified so
`value - cost` is transferred to the `recipient` and `cost` is transferred to the `authority`
doing the relay.
this way the `recipient` pays the relaying `authority` for the execution of the `withdraw` transaction.

relayers can set the gas price for `MainBridge.withdraw`.
they could set a very high gas price resulting in a very high `cost` through which they could burn large portions of `value`.
to shut down this attack the `mainGasPrice` param was added to `SideBridge.transferMainViaRelay`.
end users have control over the cost/latency tradeoff of their relay transaction through the `mainGasPrice`.
relayers have to set gas price to `mainGasPrice` when calling `MainBridge.withdraw`.
the `recipient` for `value` is the exception and can freely choose any gas price.
see https://github.com/paritytech/parity-bridge/issues/112 for more details.

`MainBridge.withdraw` is currently the only transaction bridge authorities execute on `main`.
care must be taken to secure future functions that bridge authorities will execute
on `main` in similar ways.
