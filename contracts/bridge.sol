pragma solidity ^0.4.17;


/// general helpers.
/// `internal` so they get compiled into contracts using them.
library Helpers {
    /// returns whether `array` contains `value`.
    function addressArrayContains(address[] array, address value) internal pure returns (bool) {
        for (uint i = 0; i < array.length; i++) {
            if (array[i] == value) {
                return true;
            }
        }
        return false;
    }

    // returns the digits of `inputValue` as a string.
    // example: `uintToString(12345678)` returns `"12345678"`
    function uintToString(uint inputValue) internal pure returns (string) {
        // figure out the length of the resulting string
        uint length = 0;
        uint currentValue = inputValue;
        do {
            length++;
            currentValue /= 10;
        } while (currentValue != 0);
        // allocate enough memory
        bytes memory result = new bytes(length);
        // construct the string backwards
        uint i = length - 1;
        currentValue = inputValue;
        do {
            result[i--] = byte(48 + currentValue % 10);
            currentValue /= 10;
        } while (currentValue != 0);
        return string(result);
    }
}


/// Library used only to test Helpers library via rpc calls
library HelpersTest {
    function addressArrayContains(address[] array, address value) public pure returns (bool) {
        return Helpers.addressArrayContains(array, value);
    }

    function uintToString(uint256 inputValue) public pure returns (string str) {
        return Helpers.uintToString(inputValue);
    }
}


// helpers for message signing.
// `internal` so they get compiled into contracts using them.
library MessageSigning {
    function recoverAddressFromSignedMessage(bytes signature, bytes message) internal pure returns (address) {
        require(signature.length == 65);
        bytes32 r;
        bytes32 s;
        bytes1 v;
        // solium-disable-next-line security/no-inline-assembly
        assembly {
            r := mload(add(signature, 0x20))
            s := mload(add(signature, 0x40))
            v := mload(add(signature, 0x60))
        }
        return ecrecover(hashMessage(message), uint8(v), r, s);
    }

    function hashMessage(bytes message) internal pure returns (bytes32) {
        bytes memory prefix = "\x19Ethereum Signed Message:\n";
        return keccak256(prefix, Helpers.uintToString(message.length), message);
    }
}


/// Library used only to test MessageSigning library via rpc calls
library MessageSigningTest {
    function recoverAddressFromSignedMessage(bytes signature, bytes message) public pure returns (address) {
        return MessageSigning.recoverAddressFromSignedMessage(signature, message);
    }
}


library Message {
    // layout of message :: bytes:
    // offset  0: 32 bytes :: uint (little endian) - message length
    // offset 32: 20 bytes :: address - recipient address
    // offset 52: 32 bytes :: uint (little endian) - value
    // offset 84: 32 bytes :: bytes32 - transaction hash

    // bytes 1 to 32 are 0 because message length is stored as little endian.
    // mload always reads 32 bytes.
    // so we can and have to start reading recipient at offset 20 instead of 32.
    // if we were to read at 32 the address would contain part of value and be corrupted.
    // when reading from offset 20 mload will read 12 zero bytes followed
    // by the 20 recipient address bytes and correctly convert it into an address.
    // this saves some storage/gas over the alternative solution
    // which is padding address to 32 bytes and reading recipient at offset 32.
    // for more details see discussion in:
    // https://github.com/paritytech/parity-bridge/issues/61

    function getRecipient(bytes message) internal pure returns (address) {
        address recipient;
        // solium-disable-next-line security/no-inline-assembly
        assembly {
            recipient := mload(add(message, 20))
        }
        return recipient;
    }

    function getValue(bytes message) internal pure returns (uint) {
        uint value;
        // solium-disable-next-line security/no-inline-assembly
        assembly {
            value := mload(add(message, 52))
        }
        return value;
    }

    function getTransactionHash(bytes message) internal pure returns (bytes32) {
        bytes32 hash;
        // solium-disable-next-line security/no-inline-assembly
        assembly {
            hash := mload(add(message, 84))
        }
        return hash;
    }
}


/// Library used only to test Message library via rpc calls
library MessageTest {
    function getRecipient(bytes message) public pure returns (address) {
        return Message.getRecipient(message);
    }

    function getValue(bytes message) public pure returns (uint) {
        return Message.getValue(message);
    }

    function getTransactionHash(bytes message) public pure returns (bytes32) {
        return Message.getTransactionHash(message);
    }
}


contract HomeBridge {
    /// Number of authorities signatures required to withdraw the money.
    ///
    /// Must be lesser than number of authorities.
    uint public requiredSignatures;

    /// The gas cost of calling `HomeBridge.withdraw`.
    ///
    /// Is subtracted from `value` on withdraw.
    /// recipient pays the relaying authority for withdraw.
    /// this shuts down attacks that exhaust authorities funds on home chain.
    uint public estimatedGasCostOfWithdraw;

    /// Contract authorities.
    address[] public authorities;

    /// Used foreign transaction hashes.
    mapping (bytes32 => bool) withdraws;

    /// Event created on money deposit.
    event Deposit (address recipient, uint value);

    /// Event created on money withdraw.
    event Withdraw (address recipient, uint value);

    /// Multisig authority validation
    modifier allAuthorities(uint8[] v, bytes32[] r, bytes32[] s, bytes message) {
        var hash = MessageSigning.hashMessage(message);
        var used = new address[](requiredSignatures);

        require(requiredSignatures <= v.length);

        for (uint i = 0; i < requiredSignatures; i++) {
            var a = ecrecover(hash, v[i], r[i], s[i]);
            require(Helpers.addressArrayContains(authorities, a));
            require(!Helpers.addressArrayContains(used, a));
            used[i] = a;
        }
        _;
    }

    /// Constructor.
    function HomeBridge(
        uint requiredSignaturesParam,
        address[] authoritiesParam,
        uint estimatedGasCostOfWithdrawParam
    ) public
    {
        require(requiredSignaturesParam != 0);
        require(requiredSignaturesParam <= authoritiesParam.length);
        requiredSignatures = requiredSignaturesParam;
        authorities = authoritiesParam;
        estimatedGasCostOfWithdraw = estimatedGasCostOfWithdrawParam;
    }

    /// Should be used to deposit money.
    function () public payable {
        Deposit(msg.sender, msg.value);
    }

    /// to be called by authorities to check
    /// whether they withdraw message should be relayed or whether it
    /// is too low to cover the cost of calling withdraw and can be ignored
    function isMessageValueSufficientToCoverRelay(bytes message) public view returns (bool) {
        return Message.getValue(message) > getWithdrawRelayCost();
    }

    /// an upper bound to the cost of relaying a withdraw by calling HomeBridge.withdraw
    function getWithdrawRelayCost() public view returns (uint) {
        return estimatedGasCostOfWithdraw * tx.gasprice;
    }

    /// Used to withdraw money from the contract.
    ///
    /// message contains:
    /// withdrawal recipient (bytes20)
    /// withdrawal value (uint)
    /// foreign transaction hash (bytes32) // to avoid transaction duplication
    ///
    /// NOTE that anyone can call withdraw provided they have the message and required signatures!
    function withdraw(uint8[] v, bytes32[] r, bytes32[] s, bytes message) public allAuthorities(v, r, s, message) {
        require(message.length == 84);
        address recipient = Message.getRecipient(message);
        uint value = Message.getValue(message);
        bytes32 hash = Message.getTransactionHash(message);

        // The following two statements guard against reentry into this function.
        // Duplicated withdraw or reentry.
        require(!withdraws[hash]);
        // Order of operations below is critical to avoid TheDAO-like re-entry bug
        withdraws[hash] = true;

        // this fails if `value` is not even enough to cover the relay cost.
        // Authorities simply IGNORE withdraws where `value` can’t relay cost.
        // Think of it as `value` getting burned entirely on the relay with no value left to pay out the recipient.
        require(isMessageValueSufficientToCoverRelay(message));

        uint estimatedWeiCostOfWithdraw = getWithdrawRelayCost();

        // charge recipient for relay cost
        uint valueRemainingAfterSubtractingCost = value - estimatedWeiCostOfWithdraw;

        // pay out recipient
        recipient.transfer(valueRemainingAfterSubtractingCost);

        // refund relay cost to relaying authority
        msg.sender.transfer(estimatedWeiCostOfWithdraw);

        Withdraw(recipient, valueRemainingAfterSubtractingCost);
    }
}


contract ForeignBridge {
    struct SignaturesCollection {
        /// Signed message.
        bytes message;
        /// Authorities who signed the message.
        address[] signed;
        /// Signaturs
        bytes[] signatures;
    }

    /// Number of authorities signatures required to withdraw the money.
    ///
    /// Must be lesser than number of authorities.
    uint public requiredSignatures;

    // part of ERC20 spec
    uint public totalSupply;

    /// Contract authorities.
    address[] public authorities;

    /// Ether balances
    mapping (address => uint) public balances;

    /// Pending deposits and authorities who confirmed them
    mapping (bytes32 => address[]) deposits;

    /// Pending signatures and authorities who confirmed them
    mapping (bytes32 => SignaturesCollection) signatures;

    /// Event created on money deposit.
    event Deposit(address recipient, uint value);

    /// Event created on money withdraw.
    event Withdraw(address recipient, uint value);

    /// Event created on money transfer
    event Transfer(address from, address to, uint value);

    /// Collected signatures which should be relayed to home chain.
    event CollectedSignatures(address authority, bytes32 messageHash);

    /// Constructor.
    function ForeignBridge(
        uint requiredSignaturesParam,
        address[] authoritiesParam
    ) public
    {
        require(requiredSignaturesParam != 0);
        require(requiredSignaturesParam <= authoritiesParam.length);
        requiredSignatures = requiredSignaturesParam;
        authorities = authoritiesParam;
    }

    // part of ERC20 spec
    function balanceOf(address tokenOwner) public constant returns (uint) {
        return balances[tokenOwner];
    }

    /// Multisig authority validation
    modifier onlyAuthority() {
        require(Helpers.addressArrayContains(authorities, msg.sender));
        _;
    }

    /// Used to deposit money to the contract.
    ///
    /// deposit recipient (bytes20)
    /// deposit value (uint)
    /// mainnet transaction hash (bytes32) // to avoid transaction duplication
    function deposit(address recipient, uint value, bytes32 transactionHash) public onlyAuthority() {
        // Protection from misbehaing authority
        var hash = keccak256(recipient, value, transactionHash);

        // Duplicated deposits
        require(!Helpers.addressArrayContains(deposits[hash], msg.sender));

        deposits[hash].push(msg.sender);
        // TODO: this may cause troubles if requiredSignatures len is changed
        if (deposits[hash].length == requiredSignatures) {
            balances[recipient] += value;
            totalSupply += value;
            Deposit(recipient, value);
        }
    }

    /// Transfer `value` from `msg.sender`s local balance (on `foreign` chain) to `recipient` on `home` chain.
    ///
    /// immediately decreases `msg.sender`s local balance.
    /// emits a `Withdraw` event which will be picked up by the bridge authorities.
    /// bridge authorities will then sign off (by calling `submitSignature`) on a message containing `value`,
    /// `recipient` and the `hash` of the transaction on `foreign` containing the `Withdraw` event.
    /// once `requiredSignatures` are collected a `CollectedSignatures` event will be emitted.
    /// an authority will pick up `CollectedSignatures` an call `HomeBridge.withdraw`
    /// which transfers `value - relayCost` to `recipient` completing the transfer.
    function transferHomeViaRelay(address recipient, uint value) public {
        require(balances[msg.sender] >= value);
        // fails if value == 0, or if there is an overflow
        require(balances[recipient] + value > balances[recipient]);

        balances[msg.sender] -= value;
        totalSupply -= value;
        Withdraw(recipient, value);
    }

    /// Transfer `value` to `recipient` on this `foreign` chain.
    ///
    /// does not affect `home` chain. does not do a relay.
    function transferLocal(address recipient, uint value) public {
        require(balances[msg.sender] >= value);
        // fails if value == 0, or if there is an overflow
        require(balances[recipient] + value > balances[recipient]);

        balances[msg.sender] -= value;
        balances[recipient] += value;
        Transfer(msg.sender, recipient, value);
    }

    /// Should be used as sync tool
    ///
    /// Message is a message that should be relayed to main chain once authorities sign it.
    ///
    /// for withdraw message contains:
    /// withdrawal recipient (bytes20)
    /// withdrawal value (uint)
    /// foreign transaction hash (bytes32) // to avoid transaction duplication
    function submitSignature(bytes signature, bytes message) public onlyAuthority() {
        // Validate submited signatures
        require(MessageSigning.recoverAddressFromSignedMessage(signature, message) == msg.sender);

        // Valid withdraw message must have 84 bytes
        require(message.length == 84);
        var hash = keccak256(message);

        // Duplicated signatures
        require(!Helpers.addressArrayContains(signatures[hash].signed, msg.sender));
        signatures[hash].message = message;
        signatures[hash].signed.push(msg.sender);
        signatures[hash].signatures.push(signature);

        // TODO: this may cause troubles if requiredSignatures len is changed
        if (signatures[hash].signed.length == requiredSignatures) {
            CollectedSignatures(msg.sender, hash);
        }
    }

    /// Get signature
    function signature(bytes32 hash, uint index) public view returns (bytes) {
        return signatures[hash].signatures[index];
    }

    /// Get message
    function message(bytes32 hash) public view returns (bytes) {
        return signatures[hash].message;
    }
}
