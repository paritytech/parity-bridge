# keeping a validator set on one chain in sync with a validator set on another chain

in other words: how to allow contracts to access a validator set on another chain?

## context

there are two chains named `home_chain` and `foreign_chain`.
that naming is taken from parity-bridge.
for this document the meaning of the names is irrelevant, meaning
one could switch the names.

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

### what triggers a relay of validator set changes?

it would be a clean solution to have validator processes listen to `ChangeFinalized`
events (see https://github.com/paritytech/contracts/blob/master/validator_contracts/MajorityList.sol).

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
of `old_validator_set` in the minutes following a change to the validator set.

### how does the relay happen?

each validator process responds to `ChangeFinalized(addresses)`
events on the `foreign_chain.ValidatorSet` contract
by calling `home_chain.BridgedValidatorSet.calledByValidatorProcessOnChangeFinalized(addresses, blockNumber)`
where `blockNumber` is the number of the block containing the `ChangeFinalized` event.

see the implementation of
[calledByValidatorProcessOnChangeFinalizedEvent](../contracts/bridged_validator_set.sol).

### how do we change the `requiredSignatureCount` in `BridgedValidatorSet`?

*future work*

### what happens if two transactions containing `ChangeFinalized` get mined in reverse order?

how do we prevent an older change from overwriting a newer that just
happened to get mined later?

**only accept changes in blocks that are >= the block we last accepted a change from**

[see the implementation](../contracts/bridged_validator_set.sol)

### what if a rogue validator tampers with the `newValidatorSet` in transit?

**prevented by hashing the validator set**

only switch to new validator set if `requiredSignaturesCount`
validators from old validator set called `calledByValidatorProcessOnChangeFinalizedEvent`
with the exact same pair of `newValidatorSet` and `blockNumber`.

[see the implementation](../contracts/bridged_validator_set.sol)

### what if one of the transactions containing call to `calledByValidatorProcessOnChangeFinalizedEvent` never gets mined on `home_chain`?

change might not be relayed in that case

add some retry functionality into validator processes

*future work*

### how do we prevent storage of `BridgedValidatorSet` from infinitely expanding?

*future work*
