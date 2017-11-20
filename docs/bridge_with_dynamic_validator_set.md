# modifications required to bridge in order to make it work with dynamic validator sets

the parity-bridge currently (2017-11-16 / [740601530096c261695ba9216b5e9d5f871a229f](https://github.com/paritytech/parity-bridge/commit/740601530096c261695ba9216b5e9d5f871a229f))
has a hardcoded list of authorities.

we want to switch to a dynamic validator set represented on `foreign_chain` by
a contract implementing the `ValidatorSet` interface.

the approach laid out in this document depends on the fact that a `BridgedValidatorSet`,
is present on `home_chain` and has validator processes running
as described in [bridged_validator_set.md](bridged_validator_set.md),

this means that contracts on `home_chain` can call `home_chain.BridgedValidatorSet.getValidatorSet()`
to obtain the newest finalized version of `foreign_chain.ValidatorSet.getValidatorSet()`.

## naive approach

say we naively modify `HomeBridge` and `ForeignBridge` (https://github.com/paritytech/parity-bridge/blob/master/contracts/bridge.sol)
such that every time they would have previously accessed their
`authorities` storage they now call `home_chain.BridgedValidatorSet.getValidatorSet()` (`HomeBridge`)
and `foreign_chain.ValidatorSet.getValidatorSet()` (`ForeignBridge`).

## problematic edge case

if a

this won't work



transaction gets stuck.

## solution for bridge deposit

### detect whether validator set has changed during deposit

send `validatorSetHash = sha3(home_chain.BridgedValidatorSet.getValidatorSet())`
with the `home_chain.HomeBridge.Deposit` event.

bridge processes call `foreign_chain.ForeignBridge.deposit(..., validatorSetHash)`.

inside `foreign_chain.ForeignBridge.deposit`:

obtain the newest validator set: `validatorSet = BridgedValidatorSet.getValidatorSet()`.

if there are enough signatures only commit to the change (change balances) if
`sha3(validatorSet) == validatorSetHash` i.e. validator set hasn't changed
during transport.

else retry.

### retry if validator set has change during deposit

`ForeignBridge` emits a
`RetryDeposit(recipient, value, originalTransactionHash, validatorSetHash)`
event with the intention of collecting signatures again from the newest
`validatorSet` (`sha3(validatorSet) == validatorSetHash`).

`RetryDeposit` is picked up by bridge processes.

bridge processes trust `ForeignBridge`.

bridge processes then call `ForeignBridge.deposit` again.

`ForeignBridge.deposit` only modifies balance once enough signatures
are collected and if the validator set still matches (see previous section).
else it retries again.

start a retry every time not enough signatures are collected.

---

remove modifier `onlyAuthority`
instead just collect information about it
in order to be able to distinguish misbehaving validators from

retry until we have `requiredSignatures` verifiable signatures from 
the validator set that is current validator set of ForeignChain.

remember those 

once we have n signatures that belong to the same validator

if there's n signatures and the validator set hash matches and
we can verify all signatures we got then allow it

otherwise it should be in a state where it will eventually retry the transaction

emit `RetryDeposit` event. picked up by validators.
by that point in time the `ValidatorChange` should be completed.

calls `HomeChain.retryDeposit(transactionHash)`:
first come first serve of validators.
but validator must be in authority set.
required that transaction has already been deposited once.
this makes the `HomeChainContract` emit a `Deposit` event again
which is picked up by the bridges

**this doesn't work because HomeChain can't access & check
transactionHash which opens up DOS vector.**

retry only works for things that have already made it to the other side.
initialize retry from the other side.

conditions for retry:
certain age?

retry 

this can be repeated indefinitely but only once per validator_set_hash.

make things unique by tuple `transaction_hash, validator_set_hash`.

**both old and new validator set must have more than requiredSignatures members.
otherwise it will be stuck**

working backwards from that.

`ForeignBridgeContract` only accepts final operation once.
flagmap.

## problem: how to retry relay-transaction of foreign_chain -> home_chain (withdrawal) relay-transaction

`HomeBridgeContract` only accepts final operation once.

if all preconditions are met.

retryExternalTransfer(transactionHash)

emits withdraw event again

retry by hash

transactionHash is not known on the foreign_chain.
that is a problem since it would open up spamming the contract.

## problem: what if the validator that was selected to relay the final operation dies or goes rouge

the operation gets stuck.

possible solution: timeout?
add public function to retry stuck transactions on both sides.
if its your transaction you can call it.
call it automatically on certain conditions.

alternatively: only authorities can unstuck transactions.

remember sender. only sender can unstuck transactions?
that puts the burden on the sender which is bad UX.

add ability to retry relay-transactions.

### retry

originating chain has proof that retry is required

the event is the proof since it can only be emitted by a function

must stay in function state that is protected by `onlyAuthority`

ForeignChain.retryDeposit(transactionHash) ->
RetryDeposit(transactionHash, validatorSetHash)
-> bridges ->
ForeignChain.retryDepositConfirm(signature, transactionHash)

it does not even need to get back to the other side
it just needs to go to the validators and then back to the originating chain

if retryDepositConfirm has collected enough signatures and the validatorSetHash
is still the same then the deposit goes through

now all the 

**in general just calling out to the validators and collecting n signatures
is enough proof**

if the validator set differs then the new validator set is not yet on the
HomeChain 
different in both directions.

how can we wait until the new validator set is there?

use a validator set nonce instead of a hash

retry until they match???

hoping that in the meantime the bridges have transfered the new validator set
from foreign to home

use the delay in transport as waiting

if in the meantime we are good in using both the new and old validator set
then we can simplify it
