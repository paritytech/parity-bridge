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

## withdrawal

`foreign_chain.ForeignBridge.transfer(recipient, value)`
emits `Withdraw(current arguments..., trustedAuthoritiesHash, validatorSet)` where
`validatorSet = ValidatorSet.getValidators()`
and trustedAuthoritiesHash

`lastRelayedValidatorSet` and then we call that

the one final transaction

what if that chosen authority goes

ability to retry after really long block number increase to prevent ddos

send an actual confirm back to be absolutely sure

until actual confirm has been sent it is possible to retry

## edge case: misbehaving authorities dont update

this could make it stuck

expire only if no newer versions

## incentive to keep it up to date

## assumption: old authority still has majority of behaving

## unstucking of stuck for other reasons
