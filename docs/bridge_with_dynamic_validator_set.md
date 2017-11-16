# modifications required to bridge in order to make it work with dynamic validator sets

the parity-bridge currently (2017-11-16 / [740601530096c261695ba9216b5e9d5f871a229f](https://github.com/paritytech/parity-bridge/commit/740601530096c261695ba9216b5e9d5f871a229f))
has a hardcoded list of authorities.

we want to switch to a dynamic validator set represented on `foreign_chain` by
a contract implementing the `ValidatorSet` interface.

the approach laid out in this document depends on a `BridgedValidatorSet`,
as described in [bridged_validator_set.md](bridged_validator_set.md),
is present and synced on `home_chain`.

this means that contracts on `home_chain` can call `home_chain.BridgedValidatorSet.getValidatorSet()`
to obtain the newest synced version of `foreign_chain.ValidatorSet.getValidatorSet()`.

## problem

## how do we detect that the validator set has changed

## problem: validator set has changed while relay-transaction from foreign_chain to home_chain was in flight

### possible solution 1

`ForeignBridgeContract` and `HomeBridgeContract` keep a hash of the addresses
of the current validator set.
hash is either computed every time the validator set changes (to save gas)
or computed every time it's needed.
that hash is sent with every relay transaction (deposit, message).

on the last operation that will settle the relay-operation on the target chain
that hash is compared with the hash of the current validator set.

if the hashes match then the relay-operation is completed.
if the hashes differ then the relay-operation is retried.

i'd prefer a hash.
it uses more gas every time but it 

#### required decision on edge case

it matters whether the 

## problem: how to retry relay-transaction of home_chain -> foreign_chain (deposit) relay-transaction

### possible solution 1

to `ForeignBridgeContract.deposit()`
add param `validatorSetHash`

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
