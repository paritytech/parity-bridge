// a validator set that is synchronized from another chain
// using the validator set itself to validate synchronization
// (draft, don't use, needs tests, audit, static analysis)
contract BridgedValidatorSet {
    address[] public validatorSet;
    // maps hash of a requested new validator set (+ blockNumber) 
    // to the addresses of validators that have signed off on the change
    mapping (bytes32 => address[]) changeRequests;
    uint256 requiredSignatures;
    uint256 blockNumberOfLastSyncedChange;

    function BridgedValidatorSet(address[] initialValidatorSet, uint256 requiredSignaturesCount) {
        validatorSet = initialValidatorSet;
        requiredSignaturesCount = requiredSignaturesCount;
        // initially accept change from any block number
        blockNumberOfLastSyncedChange = 0;
    }

    function getValidators() constant returns (address[]) {
        return validatorSet;
    }

    function calledByBridgeOnChangeFinalizedEvent(address[] newValidatorSet, uint256 blockNumberOfChange) {
        // only senders that are currently validators can call this function
        require(validatorSet.contains(msg.sender));
        // ensure that we don't go back to a previous change if
        // transactions get mined out of order.
        // treat transactions within the same block as having the same priority.
        // could add transactionIndex to distinguish transactions within block.
        require(blockNumberOfLastSyncedChange <= blockNumberOfChange);

        // unique hash for this change
        bytes32 changeHash = sha3(newValidatorSet, blockNumberOfChange);
        // prevent misbehaving bridge processes from
        // calling this function twice with the same change request
        require(!changeRequests[changeHash].contains(msg.sender));

        // remember that this validator has signed off on the change
        changeRequests[changeHash].push(msg.sender);

        if (changeRequests[changeHash].length == requiredSignatures) {
            // enough validators from current validator set
            // have signed off (called this function) on the exact
            // same (ensured by changeHash) new validator set
            validatorSet = newValidatorSet;
            blockNumberOfLastSyncedChange = blockNumberOfChange;
            // collect garbage
            delete changeRequests[changeHash];
            ChangeFinalized(newValidatorSet);
        }
    }

    event ChangeFinalized(address[] current_set);
}
