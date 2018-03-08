# using the ropsten-kovan test-bridge

a test-bridge is deployed between ropsten and kovan.

ropsten: `HomeBridge` contract at [0xD8f166F41A198981688C68FbDC5F0e32180b7D6E](https://ropsten.etherscan.io/address/0xD8f166F41A198981688C68FbDC5F0e32180b7D6E)

kovan: `ForeignBridge` contract at [0xDf40eFE7BCFae751b47D4dBe26Ed457fa335cbFe](https://kovan.etherscan.io/address/0xDf40eFE7BCFae751b47D4dBe26Ed457fa335cbFe)
which is an [ERC20-token](https://kovan.etherscan.io/token/0xDf40eFE7BCFae751b47D4dBe26Ed457fa335cbFe).

this guide assumes you use [metamask](https://metamask.io/).
it also assumes that in metamask you have three accounts `address1`, `address2` and `address3` which initially have 0 ether on ropsten and kovan.

in this guide you will:

1. use the bridge to transfer ropsten ether into ERC20 tokens on kovan
2. transfer tokens around on kovan
3. use the bridge to transfer tokens on kovan back into ether on ropsten

before playing around with the bridge you need some test ether on both testnets.

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
contract at `0xD8f166F41A198981688C68FbDC5F0e32180b7D6E`

the transaction should show up on
https://ropsten.etherscan.io/address/0xD8f166F41A198981688C68FbDC5F0e32180b7D6E

[what if the transaction is reverted?](troubleshooting_guide.md)

**the bridge is now relaying the deposit to kovan**

after ~1 minute a transaction should show up on
https://kovan.etherscan.io/address/0xDf40eFE7BCFae751b47D4dBe26Ed457fa335cbFe

[what if the transaction doesn't show up?](troubleshooting_guide.md)

[what if the transaction is reverted?](troubleshooting_guide.md)

## check your token balance on kovan

https://kovan.etherscan.io/token/0xDf40eFE7BCFae751b47D4dBe26Ed457fa335cbFe#balances

your `address1` should show up as a token holder holding `0.1`

[what if i don't see it?](troubleshooting_guide.md)

## transfer ERC20 tokens on kovan to another address

choose `kovan` and `address1` in metamask

visit https://mycrypto.com/#send-transaction 

choose `Kovan` as the network in the upper right corner.

in mycrypto connect to `MetaMask`

in the bottom right click on `Add Custom Token`

fill in form:
- `Address`: `0xDf40eFE7BCFae751b47D4dBe26Ed457fa335cbFe`
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
- `homeGasPrice` use `5000000000` (5 shannon/gwei, [source](https://ropsten-stats.parity.io/))

choose `Metamask` to access your wallet

click `WRITE`

## confirm that you received ropsten ether on `address3`

choose `ropsten` and `address3` in metamask

balance should show 
