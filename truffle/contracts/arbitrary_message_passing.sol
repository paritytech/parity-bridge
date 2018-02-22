pragma solidity ^0.4.17;


contract MainBridge {
    event Send(address sender, address receiver, bytes data);

	function MainBridge() public {

	}

    function send(address _receiver, bytes _data) public {
        Send(msg.sender, _receiver, _data);
    }
}

contract SideBridge {
    event Receive(address sender, address receiver, bytes data, bool success);

    function receive(address sender, address receiver, bytes data) public {
        // TODO check that msg.sender is authority
        // no way to now/authorize sender with .call
        // do we even need to
        // could do it through a mapping in SideBridge
        // that would require a call though to establish the mapping.
        // could force creation of mapping.
        // or the contract could look up the mapping in the bridge contract
        // to check for the sender.
        // hmm, yes!
        // so no need to
        // or everyone can call deploy_proxy and pay the gas to deploy the
        // proxy. it's a one way proxy anyway.
        // 1. have contract on main
        // 2. SideBridge.deploy_proxy
        // 3. deploy contract on side that only accepts from 2.
        // looking at it from a user perspective.
        // PROBLEM: deploying proxy contracts on main is probably quite expensive

		// having to implement an interface and to manually parse message sucks

		// though a "calling" and proxy deploying bridge could be built
		// on top of that.
		// though not the other way around.
		// so it is more generic.


        // TODO possibly replace by assembler if needed
        var success = receiver.call(data);
        Receive(sender, receiver, data, success);
    }
}

contract MainExample {
	address public main_bridge_address;

	function MainExample(address _main_bridge_address) public {
		main_bridge_address = _main_bridge_address;
	}

	function sig() public pure returns (bytes4) {
        return bytes4(keccak256("times(uint256,uint256)"));
	}

	function payload(uint256 a, uint256 b) public pure returns (bytes) {
        var _sig = sig();

		var result = new bytes(68);

		assembly {
			mstore(add(result, 32), _sig)
			mstore(add(result, 36), a)
			mstore(add(result, 68), b)
		}

		return result;
	}

    function something(address _receiver, uint256 a, uint256 b) public {
        MainBridge(main_bridge_address).send(_receiver, payload(a, b));
    }
}

contract SideExample {
    event Times(uint256 a, uint256 b, uint256 result);

    function times(uint256 a, uint256 b) public returns (uint256) {
        Times(a, b, a * b);
    }
}
