# keeping a validator set on one chain in sync with a validator set on another chain

in other words: how to allow contracts to access a validator set on another chain?

## context

there are two chains `home_chain` and `foreign_chain`.
the names are just to distinguish the chains below and have no special meaning
for now.

an implementation of the `ValidatorSet` contract interface is deployed to `foreign_chain`.
the implementation could be [MajorityList](https://github.com/paritytech/contracts/blob/master/validator_contracts/MajorityList.sol) for example.

`n` validator processes are running.

each validator process has a `validator_address` and the private key for it.

each validator process is connected to the `home_chain` through
a parity node that has that validators `validator_address` unlocked.

each validator process is connected to the `foreign_chain` through
a parity node that has that validators `validator_address` unlocked.

in order for the approach laid out in this document to work,
a validator process must use/unlock the same
address on on both `home_chain` and `foreign_chain`.

shouldn't usually be a problem.
if using the same address isn't possible for some reason then a more complicated
approach is needed.
see [varying_home_and_foreign_addresses.md](varying_home_and_foreign_addresses.md)
for some thoughts on that.

`ValidatorSet.getValidators() == validator_address for each validator that is considered trustworthy`

## core problem

**how do we make an always up to date version of `foreign_chain.ValidatorSet.getValidators()`
available to contracts on the `home_chain`?**

## solution space

### who stores the synced validator set on `home_chain`?

the relayed validator set could be stored directly in a contract using it
(`HomeBridge` https://github.com/paritytech/parity-bridge/blob/master/contracts/bridge.sol for example).

alternatively it could be stored in a seperate `BridgedValidatorSet` contract.
the latter solution adds one more contract but keeps concerns seperate
(doesn't clutter `HomeBridge` with validator set relay logic) and allows
reuse of the bridged validator set by other contracts.
i'd prefer a dedicated contract.

[here's a draft for such a `BridgedValidatorSet` contract](../contracts/bridged_validator_set.sol)

it requires no changes to the `ValidatorSet` deployed on `foreign_chain`!

### the addresses of the validator set on foreign and home are different

only the bridge-validator processes know both addresses.

together with the validatorSet each bridge validator also sends its address on
`calledByBridgeOnChangeFinalizedEvent`

not that easy since we'd only get the addresses of the validators that
have signed off on the change.

the


change to a new validator set can only happen if the whole validator set
is able to relay the change.

or should we modify the validator set contract on the foreign_chain?
special validator set contract that makes bridging easier

### what triggers a relay of validator set changes?

it would be a clean solution to have validator processes listen to `ChangeFinalized`
events (see https://github.com/paritytech/contracts/blob/master/validator_contracts/MajorityList.sol).

`ChangeFinalized` currently (2017-11-14) doesn't exist in the `ValidatorSet` interface (https://github.com/paritytech/contracts/blob/111fe5c4ce1ddd10a0f1a68a02602697676a6ff7/validator_contracts/MajorityList.sol#L3 (https://github.com/paritytech/contracts/blob/111fe5c4ce1ddd10a0f1a68a02602697676a6ff7/validator_contracts/MajorityList.sol#L3).
it only exists in the implementations https://github.com/paritytech/contracts/blob/111fe5c4ce1ddd10a0f1a68a02602697676a6ff7/validator_contracts/MajorityList.sol#L22.
it should probably get moved into `ValidatorSet`.

### who relays the validator set?

if the validator set on `foreign_chain` has changed from `old_validator_set`
to `new_validator_set` we
still have to rely on `old_validator_set` to relay the changes
since `old_validator_set` is the only thing `SyncedValidatorSet` contract trusts at that point
in time.

this should not be a problem if we are sure we can still trust the majority
of the old validator set in the minutes following a change to the validator set.

### how does the relay happen?

each bridge process responds to `ChangeFinalized(addresses)`
events on the `foreign_chain.ValidatorSet` contract
by calling `home_chain.SyncedValidatorSet.calledByBridgeOnChangeFinalized(addresses)`.

see the implementation of `SyncedValidatorSet.calledByBridgeOnChangeFinalized`
above for further details.

### syncing required signatures

it might be required to change requiredSignatures

### what happens out of order

nonce ensures that validator set never gets overwritten
by previous version because of 

## edge cases

### what if a rogue validator tampers with the validatorSet in transit

prevented by hash
only if 

checks the integrity of the actually transmitted validator set

only switch to new validator set if requiredSignatures from old validator set
with identical validator set and nonce and nonce > lastCommitedNonce


### what if a rogue validator is the last one to and the hash

### what if transaction never gets mined

### garbage collection

### transaction order

the transaction

the second change gets more signatures later

one could introduce a nonce

nonce

TODO flesh out this section

----

a change we could make is to only have validators that are both in the old and new
validator set sign off on the 

but that would leave open the possibility of some subset of validators
colluding and 

`ForeignBridgeContract` can check it as well

`ForeignBridgeContract` and the validators working together
can we make it that the 

need to use both initialize change and commit change

### possible solution 2

make `foreign_chain` refresh validator set on every action
and if it has changed make it update its internal validator set
and make it initialize the relay to `home_chain`
through an event that is listened to by the old and new validators.

in what order to change validator set:

start bridge process for new validators.
change validator set.
have the change get picked up by the new validator set.

the relay must be consistent

the only information about authority that the `HomeBridgeContract` has
is the old validator set.

have the old validator set cooperate to relay changes to itself.
sounds risky.

we trusted the old validator set.
we still mostly trust it.

`HomeBridgeContract` only trusts n signatures of it's current validator set.
that means only n signatures of its current validator set can convince it to trust
something else.

without using that piece of information a set of fake validators
could fabricate requests to HomeBridge and take over as validators.
they could only take over on HomeBridge and not ForeignBridge.
so relay-transactions would still fail since the sets would not match up.
but they could DOS the bridge.

**question: is this correct and safe?**


## assumptions and definitions

`relay-transaction` means a transaction from one chain to another chain.
(*question: how do you call that?*)

`in-flight` means beginning when 
and ending when the 

with 

```
HomeBridgeContract.()
-> Deposit event
1->* bridges
n->1 ForeignBridgeContract.deposit()
```

and `ForeignBridgeContract.deposit` is the final transaction.
but only if n signatures were collected.

`home_chain` is an ethereum blockchain.

`foreign_chain` is an ethereum blockchain.

`ValidatorSetContract` ([MajorityList](https://github.com/paritytech/contracts/blob/master/validator_contracts/MajorityList.sol)) is deployed to `foreign_chain`.
`ForeignBridgeContract` is deployed to `foreign_chain`.

`HomeBridgeContract` is deployed to `home_chain`.

n bridge processes are running:

each bridge process represents an authority / a validator with a signing key

each bridge process is connected to a parity node that is
connected and has the validator account unlocked.
node that is connected to the foreign_chain

the addresses are in the validator set.

it starts out with the vali

## problem: how to relay changes to foreign_chain.ValidatorSetContract.validatorsList to home_chain.HomeBridge and foreign_chain.ForeignBridge

make bridges listen for `ValidatorSet.ChangeFinalized` events on the
foreign_chain.

there is the slight problem that

if `ValidatorSet.ChangeFinalized` occurs the old-bridges all send the new validator
set to `ForeignChainContract` and `HomeChainContract`.

## invariants

the only authority that HomeBridgeContract and ForeignBridgeContract
are aware of is their current set of authority addresses.


that the majority of the old validator set is still

