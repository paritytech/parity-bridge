# using an already deployed bridge system

a bridge is deployed for testing purposes between ropsten and kovan.

`HomeBridge` contract at [0xb06807115caa6d0086b844f7ffdf9b3df92257be](https://ropsten.etherscan.io/address/0xb06807115caa6d0086b844f7ffdf9b3df92257be) on ropsten

`ForeignBridge` contract at [0x93fbabdabd72c3fb0cd39fc768d72522fcd90388](http://kovan.etherscan.io/address/0x93fbabdabd72c3fb0cd39fc768d72522fcd90388) on kovan
(is an ERC20 token: )

this guide assumes you use [metamask](https://metamask.io/)
and in metamask have three accounts `address1`, `address2` and `address3` which initially have 0 ether
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

choose `ropsten` and `account1` in metamask.

send `0.1` ether (`100000000000000000` wei) to the `HomeBridge`
contract at `0xb06807115caa6d0086b844f7ffdf9b3df92257be`

the transaction should show up on https://ropsten.etherscan.io/address/0xb06807115caa6d0086b844f7ffdf9b3df92257be

[what if the transaction is reverted?](troubleshooting_guide.md)

**the bridge is now relaying the deposit to kovan**

after ~1 minute a transaction should show up on
https://kovan.etherscan.io/address/0x93fbabdabd72c3fb0cd39fc768d72522fcd90388

*TODO what if the transaction doesn't show up?*

## check your token balance on kovan

visit https://kovan.etherscan.io/address/{insert-address1-here}#tokentxns

on the bottom you should see a recent (last couple seconds) transfer
from `0x0000000000000000000000000000000000000000` (minting)
over `100000000000000000` tokens.

in the `View Tokens` dropdown to the right you should
see `0x93fbabdabd72c3fb0cd39fc768d72522fcd90388` and `100000000000000000`.

*TODO what if i don't?*

## transfer ERC20 tokens on kovan to another address

choose `kovan` and `address1` in metamask

visit https://mycrypto.com/#send-transaction and choose `MetaMask`

in the bottom right click on `Add Custom Token`

fill in
`Address`: `0x93fbabdabd72c3fb0cd39fc768d72522fcd90388`
`Token Symbol`: `BridgedEther`
`Decimals`: `18`
and click `Save`

now fill in the main `Send` form:
`To Address`: `{address2}`
`Amount to Send`: `100000000000000000` and
choose `BridgedEther` in the dropdown!

click `Generate Transaction`

click `Send Transaction`

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
