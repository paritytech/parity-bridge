# new possibly simpler approach to make bridge work with dynamic validator sets

keep the hashes of the last validator sets
sends validator set if hash of validator set

if the last validator set 



## abstract

trust previous authority sets within a time window

`block.number`

keep hashes of trusted authority sets in `home_chain.HomeBridge`.
expire them.

all messages back and forth as well as additional message on
validator set change contain both the hash of a previously trusted
authority set as well as the newest authority set if it differs.

transport of authority set only 

transport authorities set with each message

don't need any specific message relay.

## starting context

`HomeBridge` contract is deployed on `home_chain`

the hash of the initial set of authorities is in `trustedAuthoritySetHashes`

## deposit

`home_chain.HomeBridge.()` is called and emits
`Deposit(sender, value, trustedAuthoritiesHash)` where
`trustedAuthoritiesHash = sha3(authorities)` is a new argument to `Deposit`.

bridge processes pick up `Deposit` event and call
`foreign_chain.ForeignBridge.deposit(current arguments..., trustedAuthoritiesHash)`

trusted authority sets

add `trustedAuthoritiesHash` to hash.

check that we have `trustedAuthoritiesHash` in trusted authorities
and that the current authority is within that set.

modify `onlyAuthority`

**invariant:** never any authority hash in `HomeBridge` that
hasn't previously been on `foreign_chain` since they only pass in one direction.

now if 

## can we do without an actual copy of the validator set on `home_chain`?

we could have `home_chain` offload all the work to foreign_bridge

validators could proove a transaction as stuck.

on the withdrawal we need the validator set on `home_chain` to
verify the final transfer.

## a validator set signs off on a change

## accept multiple recent validator sets

## we can only send if we have proof the other side has authorities

## withdrawal

`foreign_chain.ForeignBridge.transfer(recipient, value)`
emits `Withdraw(current arguments..., trustedAuthoritiesHash, validatorSet)` where
`validatorSet = ValidatorSet.getValidators()`
and trustedAuthoritiesHash

we have to rely on the other side having a certain
validatorSet

last confirmed received validator set

confirmed received = either n blocks deep or explicit confirmation.

**or outside entity that somehow has proof of confirmation!!!!**

set received:

that we are sure has been transmitted

`lastRelayedValidatorSet` and then we call that

the one final transaction

what if that chosen authority goes

ability to retry after really long block number increase to prevent ddos

send an actual confirm back to be absolutely sure

until actual confirm has been sent it is possible to retry

## edge case: misbehaving authorities dont update

this could make it stuck

expire only if no newer versions

## add incentive to keep it up to date

authorities can be assumed to have natural incentive to behave

## assumption: old authority still has majority of behaving

## unstucking of stuck for other reasons


## why do we need bridge process to relay changes to validator set?

otherwise the validator set on `home_bridge` will grow stale.

that's not a problem for deposits.
because the newest validator set will pick up on the `Deposit` and
`ForeignBridge.deposit()` will only accept authorities from newest validator set.

is it a problem with withdraw?

**the final bridge process that calls `HomeBridge.withdraw()` must be
trusted by `HomeBridge`**

that trust relationship might change

## validator processes have access to both chains

they view the whole picture (except for the other validators)

they also do the relay of the validator set

put more logic into validator processes



## relay transaction state machine across the whole system

### deposit


