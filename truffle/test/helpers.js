// solidity Helpers library
var Helpers = artifacts.require("HelpersTest");
// testing helpers
var helpers = require("./helpers/helpers");

contract("Helpers", function(accounts) {
  it("`addressArrayContains` should function correctly", function() {
    var addresses = accounts.slice(0, 3);
    var otherAddress = accounts[3];
    var library;
    return Helpers.new().then(function(instance) {
      library = instance;

      return library.addressArrayContains.call([], otherAddress);
    }).then(function(result) {
      assert.equal(result, false, "should return false for empty array");

      return library.addressArrayContains.call([otherAddress], otherAddress);
    }).then(function(result) {
      assert.equal(result, true, "should return true for singleton array containing value");

      return library.addressArrayContains.call([addresses[0]], addresses[1]);
    }).then(function(result) {
      assert.equal(result, false, "should return false for singleton array not containing value");

      return library.addressArrayContains.call(addresses, addresses[0]);
    }).then(function(result) {
      assert.equal(result, true);

      return library.addressArrayContains.call(addresses, addresses[1]);
    }).then(function(result) {
      assert.equal(result, true);

      return library.addressArrayContains.call(addresses, addresses[2]);
    }).then(function(result) {
      assert.equal(result, true);

      return library.addressArrayContains.call(addresses, otherAddress);
    }).then(function(result) {
      assert.equal(result, false);
    })
  })

  it("`uintToString` should convert int to string", function() {
    var numbersFrom1To100 = helpers.range(1, 101);
    var library;
    return Helpers.new().then(function(instance) {
      library = instance;

      return library.uintToString.call(0)
    }).then(function(result) {
      assert.equal(result, "0");

      return Promise.all(numbersFrom1To100.map(function(number) {
        return library.uintToString.call(number);
      }));
    }).then(function(result) {
      assert.deepEqual(result, numbersFrom1To100.map(function(number) {
        return number.toString();
      }), "should convert numbers from 1 to 100 correctly");

      return library.uintToString.estimateGas(1);
    }).then(function(result) {
      console.log("estimated gas cost of Helpers.uintToString(1)", result);

      return library.uintToString.call(1234)
    }).then(function(result) {
      assert.equal(result, "1234");

      return library.uintToString.call(12345678)
    }).then(function(result) {
      assert.equal(result, "12345678");

      return library.uintToString.estimateGas(12345678)
    }).then(function(result) {
      console.log("estimated gas cost of Helpers.uintToString(12345678)", result);

      return library.uintToString.call(web3.toBigNumber("131242344353464564564574574567456"));
    }).then(function(result) {
      assert.equal(result, "131242344353464564564574574567456");
    })
  })

  it("`hasEnoughValidSignatures` should pass for 1 required signature", function() {
    var library;
    var signature;
    var requiredSignatures = 1;
    var authorities = [accounts[0], accounts[1]];
    var recipientAccount = accounts[2];
    var value = web3.toBigNumber(web3.toWei(1, "ether"));
    var homeGasPrice = web3.toBigNumber(10000);
    var message = helpers.createMessage(recipientAccount, value, "0x1045bfe274b88120a6b1e5d01b5ec00ab5d01098346e90e7c7a3c9b8f0181c80", homeGasPrice);

    return Helpers.new().then(function(instance) {
      library = instance;
    }).then(function(result) {
      return helpers.sign(authorities[0], message);
    }).then(function(result) {
      signature = result;
      var vrs = helpers.signatureToVRS(signature);

	  return library.hasEnoughValidSignatures.call(
		message,
        [vrs.v],
        [vrs.r],
        [vrs.s],
		authorities,
		requiredSignatures
	  ).then(function(result) {
		assert(result, "should return true");
	  })
	})
  })

  it("`verifySignatures` should pass for multiple signatures", function() {
    var library;
    var signatures = [];
    var requiredSignatures = 3;
    var authorities = [accounts[0], accounts[1], accounts[2]];
    var recipientAccount = accounts[3];
    var value = web3.toBigNumber(web3.toWei(1, "ether"));
    var homeGasPrice = web3.toBigNumber(10000);
    var message = helpers.createMessage(recipientAccount, value, "0x1045bfe274b88120a6b1e5d01b5ec00ab5d01098346e90e7c7a3c9b8f0181c80", homeGasPrice);

    return Helpers.new().then(function(instance) {
      library = instance;
    }).then(function(result) {

      return helpers.sign(authorities[0], message);
    }).then(function(result) {
      signatures[0] = result;

      return helpers.sign(authorities[1], message);
    }).then(function(result) {
      signatures[1] = result;

      return helpers.sign(authorities[2], message);
    }).then(function(result) {
      signatures[2] = result;

	  var vrs = [];
      vrs[0] = helpers.signatureToVRS(signatures[0]);
      vrs[1] = helpers.signatureToVRS(signatures[1]);
      vrs[2] = helpers.signatureToVRS(signatures[2]);

	  return library.hasEnoughValidSignatures.call(
		message,
        [vrs[0].v, vrs[1].v, vrs[2].v],
        [vrs[0].r, vrs[1].r, vrs[2].r],
        [vrs[0].s, vrs[1].s, vrs[2].s],
		authorities,
		requiredSignatures
	  ).then(function(result) {
		assert(result, "should return true");
	  })
	})
  })

  it("`verifySignatures` should fail for signature for other message", function() {
    var library;
    var signature;
    var requiredSignatures = 1;
    var authorities = [accounts[0], accounts[1]];
    var recipientAccount = accounts[2];
    var value = web3.toBigNumber(web3.toWei(1, "ether"));
    var homeGasPrice = web3.toBigNumber(10000);
    var homeGasPrice2 = web3.toBigNumber(100);
    var message = helpers.createMessage(recipientAccount, value, "0x1045bfe274b88120a6b1e5d01b5ec00ab5d01098346e90e7c7a3c9b8f0181c80", homeGasPrice);
    var message2 = helpers.createMessage(recipientAccount, value, "0x1045bfe274b88120a6b1e5d01b5ec00ab5d01098346e90e7c7a3c9b8f0181c80", homeGasPrice2);

    return Helpers.new().then(function(instance) {
      library = instance;
    }).then(function(result) {
      return helpers.sign(authorities[0], message);
    }).then(function(result) {
      signature = result;
      var vrs = helpers.signatureToVRS(signature);

	  return library.hasEnoughValidSignatures.call(
		message2,
        [vrs.v],
        [vrs.r],
        [vrs.s],
		authorities,
		requiredSignatures
      ).then(function(result) {
        assert.equal(result, false, "should return false");
	  })
	})
  })

  it("`verifySignatures` should fail if signer not in addresses", function() {
    var library;
    var signature;
    var requiredSignatures = 1;
    var authorities = [accounts[0], accounts[1]];
    var recipientAccount = accounts[2];
    var value = web3.toBigNumber(web3.toWei(1, "ether"));
    var homeGasPrice = web3.toBigNumber(10000);
    var message = helpers.createMessage(recipientAccount, value, "0x1045bfe274b88120a6b1e5d01b5ec00ab5d01098346e90e7c7a3c9b8f0181c80", homeGasPrice);

    return Helpers.new().then(function(instance) {
      library = instance;
    }).then(function(result) {
      return helpers.sign(accounts[3], message);
    }).then(function(result) {
      signature = result;
      var vrs = helpers.signatureToVRS(signature);

	  return library.hasEnoughValidSignatures.call(
		message,
        [vrs.v],
        [vrs.r],
        [vrs.s],
		authorities,
		requiredSignatures
      ).then(function(result) {
        assert.equal(result, false, "should return false");
	  })
	})
  })

  it("`verifySignatures` should fail for not enough signatures", function() {
    var library;
    var signature;
    var requiredSignatures = 2;
    var authorities = [accounts[0], accounts[1]];
    var recipientAccount = accounts[2];
    var value = web3.toBigNumber(web3.toWei(1, "ether"));
    var homeGasPrice = web3.toBigNumber(10000);
    var message = helpers.createMessage(recipientAccount, value, "0x1045bfe274b88120a6b1e5d01b5ec00ab5d01098346e90e7c7a3c9b8f0181c80", homeGasPrice);

    return Helpers.new().then(function(instance) {
      library = instance;
    }).then(function(result) {
      return helpers.sign(authorities[0], message);
    }).then(function(result) {
      signature = result;
      var vrs = helpers.signatureToVRS(signature);

	  return library.hasEnoughValidSignatures.call(
		message,
        [vrs.v],
        [vrs.r],
        [vrs.s],
		authorities,
		requiredSignatures
      ).then(function(result) {
        assert.equal(result, false, "should return false");
	  })
	})
  })

  it("`verifySignatures` should fail for duplicated signature", function() {
    var library;
    var signature;
    var requiredSignatures = 2;
    var authorities = [accounts[0], accounts[1]];
    var recipientAccount = accounts[2];
    var value = web3.toBigNumber(web3.toWei(1, "ether"));
    var homeGasPrice = web3.toBigNumber(10000);
    var message = helpers.createMessage(recipientAccount, value, "0x1045bfe274b88120a6b1e5d01b5ec00ab5d01098346e90e7c7a3c9b8f0181c80", homeGasPrice);

    return Helpers.new().then(function(instance) {
      library = instance;
    }).then(function(result) {
      return helpers.sign(authorities[0], message);
    }).then(function(result) {
      signature = result;
      var vrs = helpers.signatureToVRS(signature);

	  return library.hasEnoughValidSignatures.call(
		message,
        [vrs.v, vrs.v],
        [vrs.r, vrs.r],
        [vrs.s, vrs.r],
		authorities,
		requiredSignatures
      ).then(function(result) {
        assert.equal(result, false, "should return false");
	  })
	})
  })
})
