//! Badge Registry contract.
//! By Gav Wood (Ethcore), 2016.
//! Released under the Apache Licence 2.

pragma solidity ^0.4.17;

import "Owned.sol";

contract BadgeReg is Owned {
	struct Badge {
		address addr;
		bytes32 name;
		address owner;
		mapping (bytes32 => bytes32) meta;
	}

	modifier when_fee_paid { if (msg.value < fee) return; _; }
	modifier when_address_free(address _addr) { if (mapFromAddress[_addr] != 0) return; _; }
	modifier when_name_free(bytes32 _name) { if (mapFromName[_name] != 0) return; _; }
	modifier when_has_name(bytes32 _name) { if (mapFromName[_name] == 0) return; _; }
	modifier only_badge_owner(uint _id) { if (badges[_id].owner != msg.sender) return; _; }

	event Registered(bytes32 indexed name, uint indexed id, address addr);
	event Unregistered(bytes32 indexed name, uint indexed id);
	event MetaChanged(uint indexed id, bytes32 indexed key, bytes32 value);
	event AddressChanged(uint indexed id, address addr);

	function register(address _addr, bytes32 _name) payable public returns (bool) {
		return registerAs(_addr, _name, msg.sender);
	}

	function registerAs(address _addr, bytes32 _name, address _owner) payable when_fee_paid when_address_free(_addr) when_name_free(_name) public returns (bool) {
		badges.push(Badge(_addr, _name, _owner));
		mapFromAddress[_addr] = badges.length;
		mapFromName[_name] = badges.length;
		Registered(_name, badges.length - 1, _addr);
		return true;
	}

	function unregister(uint _id) only_owner public {
		Unregistered(badges[_id].name, _id);
		delete mapFromAddress[badges[_id].addr];
		delete mapFromName[badges[_id].name];
		delete badges[_id];
	}

	function setFee(uint _fee) only_owner public {
		fee = _fee;
	}

	function badgeCount() constant public returns (uint) { return badges.length; }

	function badge(uint _id) constant public returns (address addr, bytes32 name, address owner) {
		var t = badges[_id];
		addr = t.addr;
		name = t.name;
		owner = t.owner;
	}

	function fromAddress(address _addr) constant public returns (uint id, bytes32 name, address owner) {
		id = mapFromAddress[_addr] - 1;
		var t = badges[id];
		name = t.name;
		owner = t.owner;
	}

	function fromName(bytes32 _name) constant public returns (uint id, address addr, address owner) {
		id = mapFromName[_name] - 1;
		var t = badges[id];
		addr = t.addr;
		owner = t.owner;
	}

	function meta(uint _id, bytes32 _key) constant public returns (bytes32) {
		return badges[_id].meta[_key];
	}

	function setAddress(uint _id, address _newAddr) only_badge_owner(_id) when_address_free(_newAddr) public {
		var oldAddr = badges[_id].addr;
		badges[_id].addr = _newAddr;
		mapFromAddress[oldAddr] = 0;
		mapFromAddress[_newAddr] = _id;
		AddressChanged(_id, _newAddr);
	}

	function setMeta(uint _id, bytes32 _key, bytes32 _value) only_badge_owner(_id) public {
		badges[_id].meta[_key] = _value;
		MetaChanged(_id, _key, _value);
	}

	function drain() only_owner public {
		msg.sender.transfer(this.balance);
	}

	mapping (address => uint) mapFromAddress;
	mapping (bytes32 => uint) mapFromName;
	Badge[] badges;
	uint public fee = 1 ether;
}
