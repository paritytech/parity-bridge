var HomeBridge = artifacts.require("HomeBridge");
var helpers = require("./helpers/helpers");

contract('HomeBridge', function(accounts) {
  it("should deploy contract", function() {
    var meta;
    var requiredSignatures = 1;
    var authorities = [accounts[0], accounts[1]];
    var estimatedGasCostOfWithdraw = 0;

    return HomeBridge.new(
      requiredSignatures,
      authorities,
      estimatedGasCostOfWithdraw
    ).then(function(instance) {
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
    return HomeBridge.new(3, authorities, 0).then(function(_) {
      assert(false, "Contract should fail to deploy");
    }, function(err) {
      // do nothing
    })
  })

  it("should create deposit event", function() {
    var meta;
    var requiredSignatures = 1;
    var authorities = [accounts[0], accounts[1]];
    var estimatedGasCostOfWithdraw = 0;
    let userAccount = accounts[2];
    let value = web3.toWei(1, "ether");

    return HomeBridge.new(
      requiredSignatures,
      authorities,
      estimatedGasCostOfWithdraw
    ).then(function(instance) {
      meta = instance;
      return meta.sendTransaction({
        value: value,
        from: userAccount
      })
    }).then(function(result) {
      assert.equal(1, result.logs.length, "Exactly one event should have been created");
      assert.equal("Deposit", result.logs[0].event, "Event name should be Deposit");
      assert.equal(userAccount, result.logs[0].args.recipient, "Event recipient should be transaction sender");
      assert.equal(value, result.logs[0].args.value, "Event value should match deposited ether");
    })
  })

  it("isMessageValueSufficientToCoverRelay should work correctly", function() {
    var homeBridge;
    var gasPrice;
    var requiredSignatures = 1;
    var authorities = [accounts[0], accounts[1]];
    var estimatedGasCostOfWithdraw = web3.toBigNumber(100000);
    var userAccount = accounts[2];
    var recipientAccount = accounts[3];
    var estimatedWeiCostOfWithdraw;
    var transactionHash = "0x1045bfe274b88120a6b1e5d01b5ec00ab5d01098346e90e7c7a3c9b8f0181c80";

    return HomeBridge.new(
      requiredSignatures,
      authorities,
      estimatedGasCostOfWithdraw
    ).then(function(instance) {
      homeBridge = instance;
      // do a test transaction so we can figure out the gasPrice
      return homeBridge.sendTransaction({
        value: 1,
        from: userAccount
      })
    }).then(function(result) {
      return web3.eth.getTransaction(result.tx);
    }).then(function(tx) {
      // getting the gas price dynamically instead of hardcoding it
      // (which would require less code)
      // is required because solidity-coverage sets it to 1
      // and the usual default is 100000000000
      gasPrice = tx.gasPrice;
      estimatedWeiCostOfWithdraw = gasPrice.times(estimatedGasCostOfWithdraw);

      return homeBridge.getWithdrawRelayCost();
    }).then(function(result) {
      assert(result.equals(estimatedWeiCostOfWithdraw), "getWithdrawRelayCost should return correct value");

      var message = helpers.createMessage(recipientAccount, estimatedWeiCostOfWithdraw, transactionHash);
      return homeBridge.isMessageValueSufficientToCoverRelay(message);
    }).then(function(result) {
      assert.equal(result, false, "exactly estimatedWeiCostOfWithdraw is not sufficient value");

      var message = helpers.createMessage(recipientAccount, estimatedWeiCostOfWithdraw.plus(1), transactionHash);
      return homeBridge.isMessageValueSufficientToCoverRelay(message);
    }).then(function(result) {
      assert.equal(result, true, "estimatedWeiCostOfWithdraw + 1 is sufficient value");
    })
  })

  it("should allow correct withdraw without recipient paying for gas", function() {
    var homeBridge;
    var signature;
    var requiredSignatures = 1;
    var authorities = [accounts[0], accounts[1]];
    var estimatedGasCostOfWithdraw = 0;
    var userAccount = accounts[2];
    var recipientAccount = accounts[3];
    var value = web3.toBigNumber(web3.toWei(1, "ether"));
    var message = helpers.createMessage(recipientAccount, value, "0x1045bfe274b88120a6b1e5d01b5ec00ab5d01098346e90e7c7a3c9b8f0181c80");

    return HomeBridge.new(
      requiredSignatures,
      authorities,
      estimatedGasCostOfWithdraw
    ).then(function(instance) {
      homeBridge = instance;

      // "charge" HomeBridge so we can withdraw later
      return homeBridge.sendTransaction({
        value: value,
        from: userAccount
      })
    }).then(function(result) {
      return helpers.sign(authorities[0], message);
    }).then(function(result) {
      signature = result;
      var vrs = helpers.signatureToVRS(signature);

      return homeBridge.withdraw.estimateGas(
        [vrs.v],
        [vrs.r],
        [vrs.s],
        message,
        {from: userAccount, gasPrice: 1000}
      );
    }).then(function(result) {
      console.log("estimated gas cost of HomeBridge.withdraw =", result);

      var vrs = helpers.signatureToVRS(signature);
      return homeBridge.withdraw(
        [vrs.v],
        [vrs.r],
        [vrs.s],
        message,
        {from: userAccount, gasPrice: 1000}
      );
    }).then(function(result) {
      assert.equal(1, result.logs.length, "Exactly one event should be created");
      assert.equal("Withdraw", result.logs[0].event, "Event name should be Withdraw");
      assert.equal(recipientAccount, result.logs[0].args.recipient, "Event recipient should match recipient in message");
      assert(value.equals(result.logs[0].args.value), "Event value should match value in message");
    })
  })

  it("should allow correct withdraw with recipient paying caller for gas", function() {
    var homeBridge;
    var initialBalances;
    var signature;
    var requiredSignatures = 1;
    var authorities = [accounts[0], accounts[1]];
    var estimatedGasCostOfWithdraw = web3.toBigNumber(100000);
    var actualGasCostOfWithdraw;
    var gasPrice;
    var transactionResult;
    var relayCost;
    var relayerAccount = accounts[2];
    var recipientAccount = accounts[3];
    var chargerAccount = accounts[4];
    var value = web3.toBigNumber(web3.toWei(1, "ether"));
    var message = helpers.createMessage(recipientAccount, value, "0x1045bfe274b88120a6b1e5d01b5ec00ab5d01098346e90e7c7a3c9b8f0181c80");

    return HomeBridge.new(
      requiredSignatures,
      authorities,
      estimatedGasCostOfWithdraw
    ).then(function(instance) {
      homeBridge = instance;

      return helpers.getBalances(accounts);
    }).then(function(result) {
      initialBalances = result;

      // "charge" HomeBridge so we can withdraw later
      return homeBridge.sendTransaction({
        value: value,
        from: chargerAccount,
      })
    }).then(function(result) {
      return helpers.sign(authorities[0], message);
    }).then(function(result) {
      signature = result;
      var vrs = helpers.signatureToVRS(signature);

      return homeBridge.withdraw(
        [vrs.v],
        [vrs.r],
        [vrs.s],
        message,
        { from: relayerAccount, gasPrice: 1000 }
      );
    }).then(function(result) {
      transactionResult = result;
      actualGasCostOfWithdraw = web3.toBigNumber(result.receipt.gasUsed);
      return web3.eth.getTransaction(result.tx);
    }).then(function(tx) {
      // getting the gas price dynamically instead of hardcoding it
      // (which would require less code)
      // is required because solidity-coverage sets it to 1
      // and the usual default is 100000000000
      gasPrice = tx.gasPrice;
      relayCost = gasPrice.times(estimatedGasCostOfWithdraw);

      assert.equal(1, transactionResult.logs.length, "Exactly one event should be created");
      assert.equal("Withdraw", transactionResult.logs[0].event, "Event name should be Withdraw");
      assert.equal(recipientAccount, transactionResult.logs[0].args.recipient, "Event recipient should match recipient in message");
      assert(value.minus(relayCost).equals(transactionResult.logs[0].args.value), "Event value should match value in message minus relay cost");

      return helpers.getBalances(accounts);
    }).then(function(balances) {
      let actualWeiCostOfWithdraw = actualGasCostOfWithdraw.times(gasPrice);
      assert(
        balances[recipientAccount].equals(
          initialBalances[recipientAccount].plus(value.minus(relayCost))),
        "Recipient received value minus relay cost");
      assert(
        balances[relayerAccount].equals(
          initialBalances[relayerAccount]
            .minus(actualWeiCostOfWithdraw)
            .plus(relayCost)),
        "Relayer received relay cost");
    })
  })

  it("should revert withdraw if value is insufficient to cover costs", function() {
    var homeBridge;
    var initialBalances;
    var signature;
    var requiredSignatures = 1;
    var authorities = [accounts[0], accounts[1]];
    var estimatedGasCostOfWithdraw = web3.toBigNumber(100000);
    var relayerAccount = accounts[2];
    var recipientAccount = accounts[3];
    var chargerAccount = accounts[4];
    var value = estimatedGasCostOfWithdraw;
    var message = helpers.createMessage(recipientAccount, value, "0x1045bfe274b88120a6b1e5d01b5ec00ab5d01098346e90e7c7a3c9b8f0181c80");

    return HomeBridge.new(
      requiredSignatures,
      authorities,
      estimatedGasCostOfWithdraw
    ).then(function(instance) {
      homeBridge = instance;

      return helpers.getBalances(accounts);
    }).then(function(result) {
      initialBalances = result;

      // "charge" HomeBridge so we can withdraw later
      return homeBridge.sendTransaction({
        value: value,
        from: chargerAccount,
      })
    }).then(function(result) {
      return helpers.sign(authorities[0], message);
    }).then(function(result) {
      signature = result;
      var vrs = helpers.signatureToVRS(signature);

      return homeBridge.withdraw(
        [vrs.v],
        [vrs.r],
        [vrs.s],
        message,
        { from: relayerAccount, gasPrice: 1000 }
      );
    }).then(function(result) {
      assert(false, "withdraw if value <= estimatedGasCostOfWithdraw should fail");
    }, function (err) {
      // nothing
    })
  })

  it("should allow second withdraw with different transactionHash but same recipient and value", function() {
    var homeBridge;
    var requiredSignatures = 1;
    var authorities = [accounts[0], accounts[1]];
    let estimatedGasCostOfWithdraw = 0;
    var userAccount = accounts[2];
    var recipientAccount = accounts[3];
    var value = web3.toBigNumber(web3.toWei(1, "ether"));
    var message1 = helpers.createMessage(recipientAccount, value, "0x1045bfe274b88120a6b1e5d01b5ec00ab5d01098346e90e7c7a3c9b8f0181c80");
    var message2 = helpers.createMessage(recipientAccount, value, "0x038c79eb958a13aa71996bac27c628f33f227288bd27d5e157b97e55e08fd2b3");

    return HomeBridge.new(
      requiredSignatures,
      authorities,
      estimatedGasCostOfWithdraw
    ).then(function(instance) {
      homeBridge = instance;
      // "charge" HomeBridge so we can withdraw later
      return homeBridge.sendTransaction({
        value: value.times(2),
        from: userAccount
      })
    }).then(function(result) {
      return helpers.sign(authorities[0], message1);
    }).then(function(signature) {
      var vrs = helpers.signatureToVRS(signature);
      return homeBridge.withdraw(
        [vrs.v],
        [vrs.r],
        [vrs.s],
        message1,
        {from: authorities[0], gasPrice: 1000}
      );
    }).then(function(result) {
      assert.equal(1, result.logs.length, "Exactly one event should be created");
      assert.equal("Withdraw", result.logs[0].event, "Event name should be Withdraw");
      assert.equal(recipientAccount, result.logs[0].args.recipient, "Event recipient should match recipient in message");
      assert(value.equals(result.logs[0].args.value), "Event value should match value in message");

      return helpers.sign(authorities[0], message2);
    }).then(function(signature) {
      var vrs = helpers.signatureToVRS(signature);
      return homeBridge.withdraw(
        [vrs.v],
        [vrs.r],
        [vrs.s],
        message2,
        {from: authorities[0], gasPrice: 1000}
      );
    }).then(function(result) {
      assert.equal(1, result.logs.length, "Exactly one event should be created");
      assert.equal("Withdraw", result.logs[0].event, "Event name should be Withdraw");
      assert.equal(recipientAccount, result.logs[0].args.recipient, "Event recipient should match recipient in message");
      assert(value.equals(result.logs[0].args.value), "Event value should match value in message");
    })
  })

  it("should not allow second withdraw (replay attack) with same transactionHash but different recipient and value", function() {
    var homeBridge;
    var requiredSignatures = 1;
    var authorities = [accounts[0], accounts[1]];
    var estimatedGasCostOfWithdraw = 0;
    var userAccount = accounts[2];
    var recipientAccount = accounts[3];
    var value = web3.toBigNumber(web3.toWei(1, "ether"));
    var message1 = helpers.createMessage(recipientAccount, value, "0x1045bfe274b88120a6b1e5d01b5ec00ab5d01098346e90e7c7a3c9b8f0181c80");
    var message2 = helpers.createMessage(recipientAccount, value, "0x1045bfe274b88120a6b1e5d01b5ec00ab5d01098346e90e7c7a3c9b8f0181c80");

    return HomeBridge.new(
      requiredSignatures,
      authorities,
      estimatedGasCostOfWithdraw
    ).then(function(instance) {
      homeBridge = instance;
      // "charge" HomeBridge so we can withdraw later
      return homeBridge.sendTransaction({
        value: value.times(2),
        from: userAccount
      })
    }).then(function(result) {
      return helpers.sign(authorities[0], message1);
    }).then(function(signature) {
      var vrs = helpers.signatureToVRS(signature);
      return homeBridge.withdraw(
        [vrs.v],
        [vrs.r],
        [vrs.s],
        message1,
        {from: authorities[0], gasPrice: 1000}
      );
    }).then(function(result) {
      assert.equal(1, result.logs.length, "Exactly one event should be created");
      assert.equal("Withdraw", result.logs[0].event, "Event name should be Withdraw");
      assert.equal(recipientAccount, result.logs[0].args.recipient, "Event recipient should match recipient in message");
      assert(value.equals(result.logs[0].args.value), "Event value should match value in message");

      return helpers.sign(authorities[0], message2);
    }).then(function(signature) {
      var vrs = helpers.signatureToVRS(signature);
      return homeBridge.withdraw(
        [vrs.v],
        [vrs.r],
        [vrs.s],
        message2,
        {from: authorities[0], gasPrice: 1000}
      );
    }).then(function(result) {
      assert(false, "should fail");
    }, function (err) {
      // nothing
    })
  })

  it("withdraw without funds on HomeBridge should fail", function() {
    var homeBridge;
    var signature;
    var requiredSignatures = 1;
    var authorities = [accounts[0], accounts[1]];
    var estimatedGasCostOfWithdraw = 0;
    var userAccount = accounts[2];
    var recipientAccount = accounts[3];
    var value = web3.toBigNumber(web3.toWei(1, "ether"));
    var message = helpers.createMessage(recipientAccount, value, "0x1045bfe274b88120a6b1e5d01b5ec00ab5d01098346e90e7c7a3c9b8f0181c80");

    return HomeBridge.new(
      requiredSignatures,
      authorities,
      estimatedGasCostOfWithdraw
    ).then(function(instance) {
      homeBridge = instance;
      return helpers.sign(authorities[0], message);
    }).then(function(result) {
      signature = result;
      var vrs = helpers.signatureToVRS(signature);
      return homeBridge.withdraw(
        [vrs.v],
        [vrs.r],
        [vrs.s],
        message.substr(0, 83),
        {from: authorities[0], gasPrice: 1000}
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
    var requiredSignatures = 1;
    var authorities = [accounts[0], accounts[1]];
    var estimatedGasCostOfWithdraw = 0;
    var userAccount = accounts[2];
    var recipientAccount = accounts[3];
    var value = web3.toBigNumber(web3.toWei(1, "ether"));
    var message = helpers.createMessage(recipientAccount, value, "0x1045bfe274b88120a6b1e5d01b5ec00ab5d01098346e90e7c7a3c9b8f0181c80");
    // make message too short
    message = message.substr(0, 83);

    return HomeBridge.new(
      requiredSignatures,
      authorities,
      estimatedGasCostOfWithdraw
    ).then(function(instance) {
      homeBridge = instance;

      // "charge" HomeBridge so we can withdraw later
      return homeBridge.sendTransaction({
        value: value,
        from: userAccount
      })
    }).then(function(result) {
      return helpers.sign(authorities[0], message);
    }).then(function(result) {
      signature = result;
      var vrs = helpers.signatureToVRS(signature);

      return homeBridge.withdraw(
        [vrs.v],
        [vrs.r],
        [vrs.s],
        message,
        // anyone can call withdraw (provided they have the message and required signatures)
        {from: userAccount, gasPrice: 1000}
      );
    }).then(function(result) {
      assert(false, "withdraw should fail");
    }, function(err) {
    })
  })

  it("withdraw should fail if not enough signatures are provided", function() {
    var homeBridge;
    var signature;
    var requiredSignatures = 2;
    var authorities = [accounts[0], accounts[1]];
    var estimatedGasCostOfWithdraw = 0;
    var userAccount = accounts[2];
    var recipientAccount = accounts[3];
    var value = web3.toBigNumber(web3.toWei(1, "ether"));
    var message = helpers.createMessage(recipientAccount, value, "0x1045bfe274b88120a6b1e5d01b5ec00ab5d01098346e90e7c7a3c9b8f0181c80");

    return HomeBridge.new(
      requiredSignatures,
      authorities,
      estimatedGasCostOfWithdraw
    ).then(function(instance) {
      homeBridge = instance;

      // "charge" HomeBridge so we can withdraw later
      return homeBridge.sendTransaction({
        value: value,
        from: userAccount
      })
    }).then(function(result) {
      return helpers.sign(authorities[0], message);
    }).then(function(result) {
      signature = result;
      var vrs = helpers.signatureToVRS(signature);

      return homeBridge.withdraw(
        [vrs.v],
        [vrs.r],
        [vrs.s],
        message,
        // anyone can call withdraw (provided they have the message and required signatures)
        {from: userAccount, gasPrice: 1000}
      );
    }).then(function(result) {
      assert(false, "should fail");
    }, function(err) {
    })
  })

  it("withdraw should fail if duplicate signature is provided", function() {
    var homeBridge;
    var signature;
    var requiredSignatures = 2;
    var authorities = [accounts[0], accounts[1]];
    var estimatedGasCostOfWithdraw = 0;
    var userAccount = accounts[2];
    var recipientAccount = accounts[3];
    var value = web3.toBigNumber(web3.toWei(1, "ether"));
    var message = helpers.createMessage(recipientAccount, value, "0x1045bfe274b88120a6b1e5d01b5ec00ab5d01098346e90e7c7a3c9b8f0181c80");

    return HomeBridge.new(
      requiredSignatures,
      authorities,
      estimatedGasCostOfWithdraw
    ).then(function(instance) {
      homeBridge = instance;

      // "charge" HomeBridge so we can withdraw later
      return homeBridge.sendTransaction({
        value: value,
        from: userAccount
      })
    }).then(function(result) {
      return helpers.sign(authorities[0], message);
    }).then(function(result) {
      signature = result;
      var vrs = helpers.signatureToVRS(signature);

      return homeBridge.withdraw(
        [vrs.v, vrs.v],
        [vrs.r, vrs.r],
        [vrs.s, vrs.s],
        message,
        // anyone can call withdraw (provided they have the message and required signatures)
        {from: userAccount, gasPrice: 1000}
      );
    }).then(function(result) {
      assert(false, "should fail");
    }, function(err) {
    })
  })
})
