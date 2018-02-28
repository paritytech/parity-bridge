# using an already deployed bridge system

a bridge is deployed for testing purposes between ropsten and kovan.

`HomeBridge` contract at [0xb06807115caa6d0086b844f7ffdf9b3df92257be](https://ropsten.etherscan.io/address/0xb06807115caa6d0086b844f7ffdf9b3df92257be) on ropsten

`ForeignBridge` contract at [0x93fbabdabd72c3fb0cd39fc768d72522fcd90388](http://kovan.etherscan.io/address/0x93fbabdabd72c3fb0cd39fc768d72522fcd90388) on kovan
(is an ERC20 token: )

this guide assumes you use [metamask](https://metamask.io/)
and have `account1_address`, `account2_address` and `account3_address` which initially have 0 ether
on ropsten and kovan.

## getting ropsten test ether

in metamask set network to `ropsten` and choose `account1_address`

visit https://faucet.metamask.io/

click `request 1 ether from faucet`

after a couple of seconds your ether should show up in metamask!

## getting kovan test ether

post `account2_address` (not `account1_address`) in https://gitter.im/kovan-testnet/faucet.

usually within a couple of minutes someone will respond with the hash of
a transaction that sends kovan test ether to `account1_address`

## ropsten ether -> kovan tokens

send some ropsten ether `value` from `account1_address` to `0xb06807115caa6d0086b844f7ffdf9b3df92257be`.

transaction should show up on https://ropsten.etherscan.io/address/0xb06807115caa6d0086b844f7ffdf9b3df92257be

*the bridge is now relaying the deposit to kovan*

after ~1 minute a transaction should show up on
https://kovan.etherscan.io/address/0x93fbabdabd72c3fb0cd39fc768d72522fcd90388

query balance for your `address` to verify you've received `value`:
https://kovan.etherscan.io/address/0x93fbabdabd72c3fb0cd39fc768d72522fcd90388#readContract

## transfer ERC20 tokens on kovan to another address

https://mycrypto.com/#send-transaction

choose `kovan` and `account1_address` in metamask

visit https://mycrypto.com/#send-transaction

in the bottom right click on `Add Custom Token`

fill in
`Address`: `0x93fbabdabd72c3fb0cd39fc768d72522fcd90388`
`Token Symbol`: `BridgedEther`
`Decimals`: `18`
and click `Save`

fill in the main `Send` form:
`To Address`: `address2`
`Amount to Send`: `TODO` and choose `BridgedEther` in the dropdown!

click `Generate Transaction`



use
https://kovan.etherscan.io/address/0x93fbabdabd72c3fb0cd39fc768d72522fcd90388
as you would use any ERC20 token

if you have metamask installed
you can use 

the ABI is here:
http://api-kovan.etherscan.io/api?module=contract&action=getabi&address=0x93fbabdabd72c3fb0cd39fc768d72522fcd90388&format=raw

add custom token

https://kovan.etherscan.io/token/0x93fbabdabd72c3fb0cd39fc768d72522fcd90388

## transfer your bridged ether tokens back to ropsten ether

open [https://mycrypto.com/#contracts](https://mycrypto.com/#contracts)


click `Access`

paste into `address` field:
`0x93fbabdabd72c3fb0cd39fc768d72522fcd90388`

paste contents of this url into `ABI` field:
http://api-kovan.etherscan.io/api?module=contract&action=getabi&address=0x93fbabdabd72c3fb0cd39fc768d72522fcd90388&format=raw

in the dropdown select function `transferHomeViaRelay`.

fill in
`recipient`: `address3`

choose an address you control as  `recipient`

fill in `value`

for `homeGasPrice` use `100000000000` (100 shannon)
