var HomeBridge = artifacts.require("HomeBridge");
var helpers = require("./helpers/helpers");

contract('HomeBridge', function(accounts) {
  it("should deploy contract", function() {
    var meta;
    var requiredSignatures = 1;
    var authorities = [accounts[0], accounts[1]];

    return HomeBridge.new(requiredSignatures, authorities).then(function(instance) {
      meta = instance;
      return meta.requiredSignatures.call();
    }).then(function(result) {
      assert.equal(requiredSignatures, result, "Contract has invalid number of requiredSignatures");
      return Promise.all(authorities.map((_, index) => meta.authorities.call(index)));
    }).then(function(result) {
      assert.deepEqual(authorities, result, "Contract has invalid authorities");
    })
  })

  it("should fail to deploy contract with not enough required signatures", function() {
    var authorities = [accounts[0], accounts[1]];
    return HomeBridge.new(0, authorities).then(function(_) {
      assert(false, "Contract should fail to deploy");
    }, function(err) {
      // do nothing
    })
  })

  it("should fail to deploy contract with too many signatures", function() {
    var authorities = [accounts[0], accounts[1]];
    return HomeBridge.new(3, authorities).then(function(_) {
      assert(false, "Contract should fail to deploy");
    }, function(err) {
      // do nothing
    })
  })

  it("should create deposit event", function() {
    var meta;
    var requiredSignatures = 1;
    var authorities = [accounts[0], accounts[1]];
    let user_account = accounts[2];
    let value = web3.toWei(1, "ether");

    return HomeBridge.new(requiredSignatures, authorities).then(function(instance) {
      meta = instance;
      return meta.sendTransaction({
        value: value,
        from: user_account
      })
    }).then(function(result) {
      assert.equal(1, result.logs.length, "Exactly one event should have been created");
      assert.equal("Deposit", result.logs[0].event, "Event name should be Deposit");
      assert.equal(user_account, result.logs[0].args.recipient, "Event recipient should be transaction sender");
      assert.equal(value, result.logs[0].args.value, "Event value should match deposited ether");
    })
  })

  function createMessage(recipient, value, transactionHash) {
    web3._extend.utils.isBigNumber(value);
    recipient = helpers.strip0x(recipient);
    assert.equal(recipient.length, 20 * 2);

    transactionHash = helpers.strip0x(transactionHash);
    assert.equal(transactionHash.length, 32 * 2);

    var value = helpers.strip0x(helpers.bigNumberToPaddedBytes32(value));
    assert.equal(value.length, 64);
    var message = "0x" + recipient + value + transactionHash;
    var expectedMessageLength = (20 + 32 + 32) * 2 + 2;
    assert.equal(message.length, expectedMessageLength);
    return message;
  }

  it("should allow correct withdraw", function() {
    var homeBridge;
    var signature;
    var message;
    var requiredSignatures = 1;
    var authorities = [accounts[0], accounts[1]];
    var user_account = accounts[2];
    var recipient_account = accounts[3];
    var value = web3.toBigNumber(web3.toWei(1, "ether"));

    return HomeBridge.new(requiredSignatures, authorities).then(function(instance) {
      homeBridge = instance;
      // "charge" HomeBridge so we can withdraw later
      return homeBridge.sendTransaction({
        value: value,
        from: user_account
      })
    }).then(function(result) {
      message = createMessage(recipient_account, value, "0x1045bfe274b88120a6b1e5d01b5ec00ab5d01098346e90e7c7a3c9b8f0181c80");
      return helpers.sign(authorities[0], message);
    }).then(function(result) {
      signature = result;
      var vrs = helpers.signatureToVRS(signature);

      return homeBridge.withdraw.estimateGas(
        [vrs.v],
        [vrs.r],
        [vrs.s],
        message,
        {from: authorities[0]}
      );
    }).then(function(result) {
      console.log("estimated gas cost of HomeBridge.withdraw =", result);

      var vrs = helpers.signatureToVRS(signature);
      return homeBridge.withdraw(
        [vrs.v],
        [vrs.r],
        [vrs.s],
        message,
        {from: authorities[0]}
      );
    }).then(function(result) {
      assert.equal(1, result.logs.length, "Exactly one event should be created");
      assert.equal("Withdraw", result.logs[0].event, "Event name should be Withdraw");
      assert.equal(recipient_account, result.logs[0].args.recipient, "Event recipient should match recipient in message");
      assert(value.equals(result.logs[0].args.value), "Event value should match value in message");
    })
  })

  it("should allow second withdraw with different transactionHash but same recipient and value", function() {
    var homeBridge;
    var message1;
    var message2;
    var requiredSignatures = 1;
    var authorities = [accounts[0], accounts[1]];
    var user_account = accounts[2];
    var recipient_account = accounts[3];
    var value = web3.toBigNumber(web3.toWei(1, "ether"));

    return HomeBridge.new(requiredSignatures, authorities).then(function(instance) {
      homeBridge = instance;
      // "charge" HomeBridge so we can withdraw later
      return homeBridge.sendTransaction({
        value: value.times(2),
        from: user_account
      })
    }).then(function(result) {
      message1 = createMessage(recipient_account, value, "0x1045bfe274b88120a6b1e5d01b5ec00ab5d01098346e90e7c7a3c9b8f0181c80");
      return helpers.sign(authorities[0], message1);
    }).then(function(signature) {
      var vrs = helpers.signatureToVRS(signature);
      return homeBridge.withdraw(
        [vrs.v],
        [vrs.r],
        [vrs.s],
        message1,
        {from: authorities[0]}
      );
    }).then(function(result) {
      assert.equal(1, result.logs.length, "Exactly one event should be created");
      assert.equal("Withdraw", result.logs[0].event, "Event name should be Withdraw");
      assert.equal(recipient_account, result.logs[0].args.recipient, "Event recipient should match recipient in message");
      assert(value.equals(result.logs[0].args.value), "Event value should match value in message");

      message2 = createMessage(recipient_account, value, "0x038c79eb958a13aa71996bac27c628f33f227288bd27d5e157b97e55e08fd2b3");
      return helpers.sign(authorities[0], message2);
    }).then(function(signature) {
      var vrs = helpers.signatureToVRS(signature);
      return homeBridge.withdraw(
        [vrs.v],
        [vrs.r],
        [vrs.s],
        message2,
        {from: authorities[0]}
      );
    }).then(function(result) {
      assert.equal(1, result.logs.length, "Exactly one event should be created");
      assert.equal("Withdraw", result.logs[0].event, "Event name should be Withdraw");
      assert.equal(recipient_account, result.logs[0].args.recipient, "Event recipient should match recipient in message");
      assert(value.equals(result.logs[0].args.value), "Event value should match value in message");
    })
  })

  it("withdraw without funds on HomeBridge should fail", function() {
    var homeBridge;
    var signature;
    var message;
    var requiredSignatures = 1;
    var authorities = [accounts[0], accounts[1]];
    var user_account = accounts[2];
    var recipient_account = accounts[3];
    var value = web3.toBigNumber(web3.toWei(1, "ether"));

    return HomeBridge.new(requiredSignatures, authorities).then(function(instance) {
      homeBridge = instance;
      message = createMessage(recipient_account, value, "0x1045bfe274b88120a6b1e5d01b5ec00ab5d01098346e90e7c7a3c9b8f0181c80");
      return helpers.sign(authorities[0], message);
    }).then(function(result) {
      signature = result;
      var vrs = helpers.signatureToVRS(signature);
      return homeBridge.withdraw(
        [vrs.v],
        [vrs.r],
        [vrs.s],
        message.substr(0, 83),
        {from: authorities[0]}
      );
    }).then(function(result) {
      assert(false, "should fail");
    }, function (err) {
      // nothing
    })
  })

  it("should not allow withdraw with message.length != 84", function() {
    var homeBridge;
    var signature;
    var message;
    var requiredSignatures = 1;
    var authorities = [accounts[0], accounts[1]];
    var user_account = accounts[2];
    var recipient_account = accounts[3];
    var value = web3.toBigNumber(web3.toWei(1, "ether"));

    return HomeBridge.new(requiredSignatures, authorities).then(function(instance) {
      homeBridge = instance;
      // "charge" HomeBridge so we can withdraw later
      return homeBridge.sendTransaction({
        value: value,
        from: user_account
      })
    }).then(function(result) {
      message = createMessage(recipient_account, value, "0x1045bfe274b88120a6b1e5d01b5ec00ab5d01098346e90e7c7a3c9b8f0181c80");
      return helpers.sign(authorities[0], message);
    }).then(function(result) {
      signature = result;
      var vrs = helpers.signatureToVRS(signature);
      return homeBridge.withdraw(
        [vrs.v],
        [vrs.r],
        [vrs.s],
        // change message length to 83
        message.substr(0, 83),
        {from: authorities[0]}
      );
    }).then(function(result) {
      assert(false, "should fail");
    }, function (err) {
      // nothing
    })
  })
})
