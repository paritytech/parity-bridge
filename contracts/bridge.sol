pragma solidity ^0.4.11;

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

contract EthereumBridge {
    using Authorities for address[];
    
    /// Number of authorities signatures required to withdraw the money.
    ///
    /// Must be lesser than number of authorities.
    uint requiredSignatures;
    
    /// Contract authorities.
    address[] authorities;
    
    /// Used kovan transaction hashes.
    mapping (bytes32 => bool) withdraws;
    
    /// Event created on money deposit.
    event Deposit (address, uint);
    
    /// Event created on money withdraw.
    event Withdraw (address, uint);
    
    /// Multisig authority validation
    modifier allAuthorities (uint8[] v, bytes32[] r, bytes32[] s, bytes message) {
        var hash = sha3(message);
        var used = new address[](requiredSignatures);
        
        if (v.length < requiredSignatures) {
            throw;
        }
        
        for (uint i = 0; i < requiredSignatures; i++) {
            var a = ecrecover(hash, v[i], r[i], s[i]);
            if (!authorities.contains(a) || used.contains(a)) {
                throw;
            }
            used[i] = a;
        }
        _;
    }
    
    /// Constructor.
    function EthereumBridge (uint n, address[] a) {
        if (requiredSignatures > a.length) {
            throw;
        }
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
            recipient := mload(message)
            value := mload(add(message, 0x32))
            hash := mload(add(message, 0x64))
        }
        
        // Duplicated withdraw
        if (withdraws[hash]) {
            throw;
        }
        
        // Order of operations below is critical to avoid TheDAO-like bug
        withdraws[hash] = true;
        recipient.transfer(value);
        Withdraw(recipient, value);
    }
    
    /// Used to elect new authorities.
    ///
    // message contains:
    // new requiredSignatures (uint)
    // new number of authorities (uint)
    // new authorities (bytes20)
    function reelect (uint8[] v, bytes32[] r, bytes32[] s, bytes message) allAuthorities(v, r, s, message) {
        uint newRequiredSignatures;
        uint newAuthoritiesNumber;
        address addressPtr;
        
        assembly {
            newRequiredSignatures := mload(message)
            newAuthoritiesNumber := mload(add(message, 0x32))
        }

        if (newRequiredSignatures > newAuthoritiesNumber) {
            throw;
        }

        authorities.truncate(newAuthoritiesNumber);

        for (uint i = 0; i < newAuthoritiesNumber; i++) {
            assembly {
                let offset := add(0x64, mul(0x32, i))
                addressPtr := mload(add(message, offset))
            }
            authorities[i] = addressPtr;
        }
    }
}

contract KovanBridge {
    using Authorities for address[];
    
    struct SignaturesCollection {
        /// Signed message.
        bytes message;
        /// Authorities who signed signature.
        address[] signed;
        uint8[] r;
        bytes32[] s;
        bytes32[] v;
    }
    
    /// Number of authorities signatures required to withdraw the money.
    ///
    /// Must be lesser than number of authorities.
    uint requiredSignatures;
    
    /// Contract authorities.
    address[] authorities;
    
    /// Ether balances
    mapping (address => uint) balances;
    
    /// Pending deposits and authorities who confirmed them
    mapping (bytes32 => address[]) deposits;
    
    /// Pending signatures and authorities who confirmed them
    mapping (bytes32 => SignaturesCollection) signatures;
    
    /// Event created on money deposit.
    event Deposit(address, uint);
    
    /// Event created on money withdraw.
    event Withdraw(address, uint);
    
    /// Event created on money transfer
    event Transfer(address from, address to, uint);
    
    /// Collected signatures which should be relayed to ehtereum chain.
    event CollectedSignatures(bytes message, uint8[] r, bytes32[] s, bytes32[] v);
    
    /// Constructor.
    function KovanBridge(uint n, address[] a) {
        if (requiredSignatures > a.length) {
            throw;
        }
        requiredSignatures = n;
        authorities = a;
    }

    /// Multisig authority validation
    modifier onlyAuthority () {
        if (!authorities.contains(msg.sender)) {
            throw;
        }
        _;
    }
    
    /// Used to deposit money to the contract.
    ///
    /// message contains:
    /// deposit recipient (bytes20)
    /// deposit value (uint)
    /// mainnet transaction hash (bytes32) // to avoid transaction duplication
    function deposit (address recipient, uint value, bytes32 hash) onlyAuthority() {
        // Duplicated deposits
        if (deposits[hash].contains(msg.sender)) {
            throw;
        }

        deposits[hash].push(msg.sender);
        // TODO: this may cause troubles if requriedSignatures len is changed
        if (deposits[hash].length == requiredSignatures) {
            balances[recipient] += value;
            Deposit(recipient, value);
        }
    }
    
    /// Used to transfer money between accounts
    function transfer (address recipient, uint value, bool externalTransfer) {
        if (balances[msg.sender] < value) {
            throw;
        }
        
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
    function collectSignatures (uint8 r, bytes32 s, bytes32 v, bytes message) onlyAuthority() {
        var hash = sha3(message);
        
        // Duplicated signatures
        if (signatures[hash].signed.contains(msg.sender)) {
            throw;
        }
        signatures[hash].message = message;
        signatures[hash].signed.push(msg.sender);
        signatures[hash].r.push(r);
        signatures[hash].s.push(s);
        signatures[hash].v.push(v);
    
        // TODO: this may cause troubles if requriedSignatures len is changed
        if (signatures[hash].signed.length == requiredSignatures) {
            CollectedSignatures(message, signatures[hash].r, signatures[hash].s, signatures[hash].v);
        }
    }
}