pragma solidity ^0.4.15;

library Authorities {
    function contains (address[] self, address value) internal returns (bool) {
        for (uint i = 0; i < self.length; i++) {
            if (self[i] == value) {
                return true;
            }
        }
        return false;
    }

    function truncate (address[] storage self, uint len) internal {
        for (uint i = len; i < self.length; i++) {
            delete self[i];
        }
        self.length = len;
    }
}

/// Library used only to test Signer library via rpc calls
library SignerTest {
    function signer (bytes signature, bytes message) constant returns (address) {
        return Signer.signer(signature, message);
    }
}

library Utils {
    function toString (uint256 v) internal returns (string str) {
        // it is used only for small numbers
        bytes memory reversed = new bytes(8);
        uint i = 0;
        while (v != 0) {
            uint remainder = v % 10;
            v = v / 10;
            reversed[i++] = byte(48 + remainder);
        }
        bytes memory s = new bytes(i);
        for (uint j = 0; j < i; j++) {
            s[j] = reversed[i - j - 1];
        }
        str = string(s);
    }
}

library Signer {
    function signer (bytes signature, bytes message) internal returns (address) {
        require(signature.length == 65);
        bytes32 r;
        bytes32 s;
        bytes1 v;
        assembly {
            r := mload(add(signature, 0x20))
            s := mload(add(signature, 0x40))
            v := mload(add(signature, 0x60))
        }
        return ecrecover(hash(message), uint8(v), r, s);
    }

    function hash (bytes message) internal returns (bytes32) {
        bytes memory prefix = "\x19Ethereum Signed Message:\n";
        return sha3(prefix, Utils.toString(message.length), message);
    }
}

contract EthereumBridge {
    using Authorities for address[];

    /// Number of authorities signatures required to withdraw the money.
    ///
    /// Must be lesser than number of authorities.
    uint public requiredSignatures;

    /// Contract authorities.
    address[] public authorities;

    /// Used kovan transaction hashes.
    mapping (bytes32 => bool) withdraws;

    /// Event created on money deposit.
    event Deposit (address recipient, uint value);

    /// Event created on money withdraw.
    event Withdraw (address recipient, uint value);

    /// Multisig authority validation
    modifier allAuthorities (uint8[] v, bytes32[] r, bytes32[] s, bytes message) {
        var hash = Signer.hash(message);
        var used = new address[](requiredSignatures);

        require(requiredSignatures <= v.length);

        for (uint i = 0; i < requiredSignatures; i++) {
            var a = ecrecover(hash, v[i], r[i], s[i]);
            require(authorities.contains(a));
            require(!used.contains(a));
            used[i] = a;
        }
        _;
    }

    /// Constructor.
    function EthereumBridge (uint n, address[] a) {
        require(n != 0);
        require(n <= a.length);
        requiredSignatures = n;
        authorities = a;
    }

    /// Should be used to deposit money.
    function () payable {
        Deposit(msg.sender, msg.value);
    }

    /// Used to withdrawn money from the contract.
    ///
    /// message contains:
    /// withdrawal recipient (bytes20)
    /// withdrawal value (uint)
    /// kovan transaction hash (bytes32) // to avoid transaction duplication
    function withdraw (uint8[] v, bytes32[] r, bytes32[] s, bytes message) allAuthorities(v, r, s, message) {
        address recipient;
        uint value;
        bytes32 hash;
        assembly {
            recipient := mload(add(message, 0x20))
            value := mload(add(message, 0x40))
            hash := mload(add(message, 0x60))
        }

        // Duplicated withdraw
        require(!withdraws[hash]);

        // Order of operations below is critical to avoid TheDAO-like bug
        withdraws[hash] = true;
        recipient.transfer(value);
        Withdraw(recipient, value);
    }
}

contract KovanBridge {
    using Authorities for address[];

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

    /// Collected signatures which should be relayed to ethereum chain.
    event CollectedSignatures(address authority, bytes32 messageHash);

    /// Constructor.
    function KovanBridge(uint n, address[] a) {
        require(n != 0);
        require(n <= a.length);
        requiredSignatures = n;
        authorities = a;
    }

    /// Multisig authority validation
    modifier onlyAuthority () {
        require(authorities.contains(msg.sender));
        _;
    }

    /// Used to deposit money to the contract.
    ///
    /// deposit recipient (bytes20)
    /// deposit value (uint)
    /// mainnet transaction hash (bytes32) // to avoid transaction duplication
    function deposit (address recipient, uint value, bytes32 transactionHash) onlyAuthority() {
        // Protection from misbehaing authority
        var hash = sha3(recipient, value, transactionHash);

        // Duplicated deposits
        require(!deposits[hash].contains(msg.sender));

        deposits[hash].push(msg.sender);
        // TODO: this may cause troubles if requriedSignatures len is changed
        if (deposits[hash].length == requiredSignatures) {
            balances[recipient] += value;
            Deposit(recipient, value);
        }
    }

    /// Used to transfer money between accounts
    function transfer (address recipient, uint value, bool externalTransfer) {
        require(balances[msg.sender] >= value);

        balances[msg.sender] -= value;
        if (externalTransfer) {
            Withdraw(recipient, value);
        } else {
            balances[recipient] += value;
            Transfer(msg.sender, recipient, value);
        }
    }

    /// Should be used as sync tool
    ///
    /// Message is a message that should be relayed to main chain once authorities sign it.
    ///
    /// for withdraw message contains:
    /// withdrawal recipient (bytes20)
    /// withdrawal value (uint)
    /// kovan transaction hash (bytes32) // to avoid transaction duplication
    function submitSignature (bytes signature, bytes message) onlyAuthority() {
        // Validate submited signatures
        require(Signer.signer(signature, message) == msg.sender);

        // Valid withdraw message must have 84 bytes
        require(message.length == 84);
        var hash = sha3(message);

        // Duplicated signatures
        require(!signatures[hash].signed.contains(msg.sender));
        signatures[hash].message = message;
        signatures[hash].signed.push(msg.sender);
        signatures[hash].signatures.push(signature);

        // TODO: this may cause troubles if requriedSignatures len is changed
        if (signatures[hash].signed.length == requiredSignatures) {
            CollectedSignatures(msg.sender, hash);
        }
    }

    /// Get signature
    function signature (bytes32 hash, uint index) constant returns (bytes) {
        return signatures[hash].signatures[index];
    }

    /// Get message
    function message (bytes32 hash) constant returns (bytes) {
        return signatures[hash].message;
    }
}
