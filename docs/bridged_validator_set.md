# keeping a validator set on one chain in sync with a validator set on another chain

allowing contracts to access a validator set on another chain

## context

there are two chains named `home_chain` and `foreign_chain`.
that naming is taken from https://github.com/paritytech/parity-bridge
for this document the special meaning of the names is irrelevant. one could switch the names.

an implementation of the `ValidatorSet` contract interface is deployed to `foreign_chain`.
the implementation could be [MajorityList](https://github.com/paritytech/contracts/blob/master/validator_contracts/MajorityList.sol) for example.

`n` validator processes are running.

each validator process has a `validator_address` and the private key for it.

each validator process is connected to `home_chain` through
a parity node that has that validators `validator_address` unlocked.

each validator process is connected to `foreign_chain` through
a parity node that has that validators `validator_address` unlocked.

in order for the approach laid out in this document to work,
a validator process must use/unlock the same
address on on both `home_chain` and `foreign_chain`.

that shouldn't usually be a problem.
if using the same address isn't possible for some reason then a more complicated approach is needed.
see [varying_home_and_foreign_addresses.md](varying_home_and_foreign_addresses.md)
for thoughts on that.

## core problem

**how do we make an always fairly recent but finalized version of `foreign_chain.ValidatorSet.getValidators()`
available to contracts on the `home_chain`?**

"fairly recent but finalized" meaning: lagging behind `foreign_chain.ValidatorSet.getValidators()` by whatever time it takes
to be certain about finality on `foreign_chain`.

## solution space

### who stores the synced validator set on `home_chain`?

the relayed validator set could be stored directly in a contract using it.
`HomeBridge` https://github.com/paritytech/parity-bridge/blob/master/contracts/bridge.sol for example.

alternatively it could be stored in a seperate `BridgedValidatorSet` contract.
this solution adds one more contract but keeps concerns seperate.
it doesn't clutter `HomeBridge`, or other processes using it, with validator set relay logic.
that keeps contracts simple and makes less room for bugs.

any contracts on `home_chain` could reuse the `BridgedValidatorSet`.

i'd prefer a dedicated `BridgedValdiatorSet` contract for those reasons.

[here's a draft for such a `BridgedValidatorSet` contract](../contracts/bridged_validator_set.sol)

it requires no changes to the `ValidatorSet` deployed on `foreign_chain`!

goal is that most of the time:
`foreign_bridge.ValidatorSet.getValidators() == home_bridge.BridgedValidatorSet.getValidators()`
except when `foreign_bridge.ValidatorSet.getValidators()`
changed and is not yet finalized.

### what triggers a relay of validator set changes?

a simple solution is to have validator processes listen to `ChangeFinalized`
events. see https://github.com/paritytech/contracts/blob/master/validator_contracts/MajorityList.sol.

`ChangeFinalized` currently (2017-11-14) doesn't exist in the `ValidatorSet` interface: https://github.com/paritytech/contracts/blob/111fe5c4ce1ddd10a0f1a68a02602697676a6ff7/validator_contracts/MajorityList.sol#L3

it only exists in the implementations: https://github.com/paritytech/contracts/blob/111fe5c4ce1ddd10a0f1a68a02602697676a6ff7/validator_contracts/MajorityList.sol#L22.
it should probably get moved into `ValidatorSet`.

### who relays the validator set?

if the validator set on `foreign_chain` has changed from `old_validator_set`
to `new_validator_set` we
still have to rely on `old_validator_set` to relay the changes
since `old_validator_set` is the only thing the `BridgedValidatorSet` contract trusts at that point
in time.

this should not be a problem if we are sure we can still trust the majority
of `old_validator_set` in the time it takes to fully relay (finalized) the validator set to `home_chain`.

### how does the relay happen?

each validator process responds to `ChangeFinalized(addresses)`
events on the `foreign_chain.ValidatorSet` contract
by calling `home_chain.BridgedValidatorSet.calledByValidatorProcessOnChangeFinalized(addresses, blockNumber)`
where `blockNumber` is the number of the block containing the `ChangeFinalized` event.

see the implementation of
[calledByValidatorProcessOnChangeFinalizedEvent](../contracts/bridged_validator_set.sol).

### how do we ensure that only validator set changes that are final on `foreign_chain` get relayed?

this is crucial!

validator processes could only do the relay for `ChangeFinalized` events
in blocks that have n (20 for example) confirmations.

alternatively validator processes could reach out to some entity providing
finality information
(rob mentioned that).

### how do we change the `requiredSignatureCount` in `BridgedValidatorSet`?

along the lines of:
having `old_required_signature_count` validator processes
sign off on the change and modifying contract to work with non-constant `requiredSignatureCount`.

*easy future work*

### what happens if two transactions containing `ChangeFinalized` get mined in reverse order?

how do we prevent an older change from overwriting a newer that just
happened to get mined later?

**only accept changes in blocks that are >= the block we last accepted a change from**

[see the implementation](../contracts/bridged_validator_set.sol)

alternatively a nonce could be used. that would require modification of
`foreign_chain.ValidatorSet` though. loosing the nice property of not having to touch that
contract for the validator set bridge to work.

### what if a rogue validator tampers with the `newValidatorSet` in transit?

**prevented by hashing the validator set**

only switch to new validator set if `requiredSignaturesCount`
validators from old validator set called `calledByValidatorProcessOnChangeFinalizedEvent`
with the exact same pair of `newValidatorSet` and `blockNumber`.

[see the implementation](../contracts/bridged_validator_set.sol)

### what if one of the transactions containing call to `calledByValidatorProcessOnChangeFinalizedEvent` never gets mined on `home_chain`?

change might never get relayed in that case.
then until the next change (might take a very long time):
`foreign_bridge.ValidatorSet.getValidators() != home_bridge.BridgedValidatorSet.getValidators()`

other bridging systems that depend on `foreign_chain.ValidatorSet` and `home_chain.BridgedValidatorSet`
will usually check that the validator set hasn't changed during their
own bridging process. such systems would be stuck while
`foreign_bridge.ValidatorSet.getValidators() != home_bridge.BridgedValidatorSet.getValidators()`

we have to add some retry functionality into validator processes!

*future work*

### how do we prevent storage of `BridgedValidatorSet` from infinitely expanding?

*future work*
