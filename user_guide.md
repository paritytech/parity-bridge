# using the ropsten-kovan test-bridge

a test-bridge is deployed ropsten and kovan

ropsten: `HomeBridge` contract at [0xb06807115caa6d0086b844f7ffdf9b3df92257be](https://ropsten.etherscan.io/address/0xb06807115caa6d0086b844f7ffdf9b3df92257be)

kovan: `ForeignBridge` contract at [0x93fbabdabd72c3fb0cd39fc768d72522fcd90388](http://kovan.etherscan.io/address/0x93fbabdabd72c3fb0cd39fc768d72522fcd90388)

`ForeignBridge` is an ERC20 token

this guide assumes you use [metamask](https://metamask.io/).
it also assumes that in metamask you have three accounts `address1`, `address2` and `address3` which initially have 0 ether on ropsten and kovan.

in this guide you will:

1. use the bridge to transfer ropsten ether into ERC20 tokens on kovan
2. transfer tokens around on kovan
3. use the bridge to transfer tokens on kovan back into ether on ropsten

before playing around with the bridge we need some test ether on both testnets.

## getting ropsten test ether for `address1`

in metamask set network to `ropsten` and choose `address1`

visit https://faucet.metamask.io/

click `request 1 ether from faucet`

after a couple of seconds your ether should show up in metamask!

## getting kovan test ether for `address1`

post `address2` (not `address1`) in https://gitter.im/kovan-testnet/faucet.

usually within a couple of minutes someone will respond with the hash of
a transaction that sends kovan test ether to `address2`

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

fill in form:
- `Address`: `0x93fbabdabd72c3fb0cd39fc768d72522fcd90388`
- `Token Symbol`: `BridgedEther`
- `Decimals`: `18`

click `Save`

now fill in the main `Send` form:
- `To Address`: `{address2}`
- `Amount to Send`: `100000000000000000` and
- choose `BridgedEther` in the dropdown!

click `Generate Transaction`

click `Send Transaction`

## transfer your bridged ether tokens back to ropsten ether

choose `kovan` and `address2` in metamask

open [https://mycrypto.com/#contracts](https://mycrypto.com/#contracts)

fill in form:
`Contact Address`: `0x93fbabdabd72c3fb0cd39fc768d72522fcd90388`
`ABI / JSON Interface`: paste contents of this url
http://api-kovan.etherscan.io/api?module=contract&action=getabi&address=0x93fbabdabd72c3fb0cd39fc768d72522fcd90388&format=raw

click `Access`

now down in the new `Read / Write Contract` section:

in the `Select a function` dropdown select `transferHomeViaRelay`

fill in form:
- `recipient`: `{address3}`
- `value`: `100000000000000000`
- `homeGasPrice` use `100000000000` (100 shannon)

choose `Metamask` to access your wallet

click `WRITE`

## confirm that you received ropsten ether on `address3`

choose `ropsten` and `address3` in metamask

balance should show 
