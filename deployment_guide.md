# deployment guide

this guide assumes that you are one of the authorities of
a PoA chain `foreign` and want to use the bridge to connect
`foreign` to another chain `home`.

since the authorities one authority has to go ahead and
deploy

we call this the leading authority

if the process is done correctly the other non-leading authorities don't have to trust
the leading authority.

## deployment guide for the leading authority

install parity 1.8.6 on the machine

setup a config file

it should show something like this
that it

confirmations????

also need to compile the contracts

look into the database file

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

verify the contracts deployed to `home_contract_address` and
`foreign_contract_address` using
https://etherscan.io/verifyContract so the other authorities
can verify that you did an honest deploy without having to trust you.

[start the bridge process](

give the database file to the other authorities.
for example by posting it as a gist.
the database file doesn't contain any sensitive information.

ask the other authorities to follow [this guide]()

ensure the process keeps running. else the bridge won't function.
(outside the scope of this guide, your devops team knows what to do).

## start the bridge




## the bridge requires two

see ... for an example config used for our tests

and recommended defaults

the database file is either used or created.

if the database file doesn't exist then one is created.

the bridge deploys the smart contracts

## one authority

the bridge processes keep some state in a database

## deployment guide for all the other non-leading authorities

you'll receive a `bridge.db` from the leading authority.

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

https://etherscan.io/address/0x4802d26384bcaf94a41108d55d5d13500dea8e61#code

verify that the source code matches the one on
https://kovan.etherscan.io/address/0x22bb16f791927197111c9a19d0b491b10c8d0e07#code


if you want to be extra sure.

the other authorities should then check the source code
of both verified contracts to ensure
source code

spread the database file to the other authorities

that is mostly 
