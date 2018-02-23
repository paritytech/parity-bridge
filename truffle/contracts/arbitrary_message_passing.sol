pragma solidity ^0.4.17;


contract MainBridge {
    event Send(address sender, address receiver, bytes data);

    function send(address _receiver, bytes _data) public {
        Send(msg.sender, _receiver, _data);
    }
}

contract SideBridge {
    event Receive(address sender, address receiver, bytes data, bool success);

    function receive(address sender, address receiver, bytes data) public {
        var success = receiver.call(data);
        Receive(sender, receiver, data, success);
    }
}

contract MainExample {
	address public main_bridge_address;

	function MainExample(address _main_bridge_address) public {
		main_bridge_address = _main_bridge_address;
	}

    function something(address _receiver, uint256 a, uint256 b) public {
		// need to build `data` manually for now.
		// there will be a handy function for it in the future:
		// https://github.com/ethereum/solidity/issues/1707
        var func_sig = bytes4(keccak256("times(uint256,uint256)"));
		var data = new bytes(68);

		assembly {
			// skip first 32 bytes which encode length of `bytes data`
			mstore(add(data, 32), func_sig)
			mstore(add(data, 36), a)
			mstore(add(data, 68), b)
		}

        MainBridge(main_bridge_address).send(_receiver, data);
    }
}

contract SideExample {
    event Times(uint256 a, uint256 b, uint256 result);

    function times(uint256 a, uint256 b) public returns (uint256) {
        Times(a, b, a * b);
    }
}
