# problem

for each validator `home_address != foreign_address` or in other words
`home_address_set != foreign_address_set`.

this means we can't simply relay `foreign_address_set`
as described in
https://gist.github.com/snd/c3c9dba3b7f80678a09e1320697911b1
since it will be of no use 



# solution space

only a validator process knows both its `home_address` and its `foreign_address`.

## approach 1 (won't work)

together with the validatorSet each validator also sends its address on
`calledByBridgeOnChangeFinalizedEvent`.

`BridgedValidatorSet` collects the `home_address` and once enough
validators

this won't work because it will 

which leads to

## approach 2 (won't work)

we need all the new validators to relay the change

problem: a single rogue validator in the new set can block the entire change

### what if the validators don't relay the change

the SyncedValidatorSet will continue to 



that will cause problems

all relay transactions will be stuck

**is this acceptable?**


## approach 3 (needs more work)

a bridge-validator uses two addresses (one on each chain).

use a modified version of `ValidatorSet` that works on pairs of addresses.

or more generally allow adding additional information to
each validator.

## approach 4 (needs more work)

have another set of validators

that set is responsible for relaying changes to another validator set
from chain to chain.

chicken and egg problem.

can we do all the signing off on one chain?

and then actually do the relay

we trust 


who trusts who


`HomeBridge`


taking a step back. what do we want to accomplish.

place something on home_chain that is not trusted by

## approach

is there some previous work on relaying validator sets?

is this actually worth digging into

---

self relaying validator set

can we split the validator 

a validator set that can relay its changes to another chain
