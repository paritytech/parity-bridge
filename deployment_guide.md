# deployment guide

this guide assumes that you are one of the authorities of
a PoA chain `foreign` and want to use the bridge to connect
`foreign` to another chain `home`.
this will create an ERC20 token on `foreign` that is backed by
ether on `home`.

since all bridge authorities use the same contracts on `foreign` and `home`
one authority has to go ahead and deploy them.

let's call this the **deploying authority**.

if the process is done correctly the other non-deploying authorities don't have to trust
the deploying authority.

upfront you must know the addresses of all authorities (`authorities`)
es well as the number of `required_signatures`

## initial deployment steps for any authority (deploying and non-deploying)

given you are an authority and 

[build and install the bridge](README.md#build)

install parity.
we tested it with [parity 1.8.10](https://github.com/paritytech/parity/releases/tag/v1.8.10)
though it should work with the latest stable release.

### start parity node that connects

start a parity node `home_node` that connects to `foreign`.

assuming `foreign = kovan`:

```
parity \
  --chain {chain name or spec}
  --ipc-path foreign.ipc
  --no-jsonrpc
  --unlock {authority_address}
  --password {path to file containing password for authority address}
```

start a parity node that connects to `foreign`.
that has the authority address unlocked.
let's call it `foreign_node`

repeat the same for `home`.

### configure the bridge

copy [integration-tests/bridge_config.toml](integration-tests/bridge_config.toml)
to a local `bridge_config.toml`.

within `bridge_config.toml` resolve/fill-in all the `ACTION REQUIRED`s.

refer to [the documentation of config options](README.md#configuration).

[if you're the **leading** authority continue here](#further-deployment-steps-for-leading-authority)

[if you're a non-leading authority continue here](#further-deployment-steps-for-non-leading-authorities)

## further deployment steps for leading authority

start the bridge by executing:

```
env RUST_LOG=info bridge --config bridge_config.toml --database bridge.db
```

it should eventually print something like this:

```
INFO:bridge: Deployed new bridge contracts
INFO:bridge:
home_contract_address = "0xebd3944af37ccc6b67ff61239ac4fef229c8f69f"
foreign_contract_address = "0xebd3944af37ccc6b67ff61239ac4fef229c8f69f"
home_deploy = 1
foreign_deploy = 1
checked_deposit_relay = 1
checked_withdraw_relay = 1
checked_withdraw_confirm = 1
```

**congratulations! the bridge has successfully deployed its contracts on both chains**

`bridge.db` should now look similar to this:

```
home_contract_address = "0xebd3944af37ccc6b67ff61239ac4fef229c8f69f"
foreign_contract_address = "0xebd3944af37ccc6b67ff61239ac4fef229c8f69f"
home_deploy = 1
foreign_deploy = 1
checked_deposit_relay = 3
checked_withdraw_relay = 4
checked_withdraw_confirm = 4
```

(verify the contracts deployed to `home_contract_address` and
`foreign_contract_address` using
https://etherscan.io/verifyContract so the other authorities
can verify that you did an honest deploy without having to trust you.)

give the `bridge.db` file to the other authorities.
for example by posting it as a gist.
the database file doesn't contain any sensitive information.

ask the other authorities to follow **this guide you're reading**.

ensure the process keeps running. else the bridge won't function.
(outside the scope of this guide, your devops team knows what to do).

## further deployment steps for non-leading authorities

you MUST receive a `bridge.db` from the leading authority.

it should look similar to this:

```
home_contract_address = "0xebd3944af37ccc6b67ff61239ac4fef229c8f69f"
foreign_contract_address = "0xebd3944af37ccc6b67ff61239ac4fef229c8f69f"
home_deploy = 1
foreign_deploy = 1
checked_deposit_relay = 3
checked_withdraw_relay = 4
checked_withdraw_confirm = 4
```

(check that the contracts deployed to
`home_contract_address` and `foreign_contract_address` are
verified on https://etherscan.io and that the source code matches
the code in the repo.)

start the bridge by executing:

```
env RUST_LOG=info bridge --config bridge_config.toml --database bridge.db
```

it should eventually print this line:

```
INFO:bridge: Starting listening to events
```

**congratulations! the bridge has successfully started and joined the other authorities**

ensure the process keeps running. else the bridge won't function.
(outside the scope of this guide, your devops team knows what to do).
