var MainBridge = artifacts.require("MainBridge");
var helpers = require("./helpers/helpers");

function newMainBridge(options) {
  if (options.estimatedGasCostOfWithdraw === undefined) {
    options.estimatedGasCostOfWithdraw = 0;
  }
  if (options.maxTotalMainContractBalance === undefined) {
    options.maxTotalMainContractBalance = 0;
  }
  if (options.maxSingleDepositValue === undefined) {
    options.maxSingleDepositValue = 0;
  }
  return MainBridge.new(
    options.requiredSignatures,
    options.authorities,
    options.estimatedGasCostOfWithdraw,
    options.maxTotalMainContractBalance,
    options.maxSingleDepositValue
  )
}

contract('MainBridge', function(accounts) {
  it("should deploy contract", function() {
    var meta;
    var authorities = [accounts[0], accounts[1]];
    var requiredSignatures = 1;

    return newMainBridge({
      requiredSignatures: requiredSignatures,
      authorities: authorities,
    }).then(function(instance) {
      meta = instance;

      return web3.eth.getTransactionReceipt(instance.transactionHash);
    }).then(function(transaction) {
      console.log("estimated gas cost of MainBridge deploy =", transaction.gasUsed);

      return meta.requiredSignatures.call();
    }).then(function(result) {
      assert.equal(requiredSignatures, result, "Contract has invalid number of requiredSignatures");

      return Promise.all(authorities.map((_, index) => meta.authorities.call(index)));
    }).then(function(result) {
      assert.deepEqual(authorities, result, "Contract has invalid authorities");

      return meta.isMainBridgeContract.call();
    }).then(function(result) {
      assert.equal(result, true)
    })
  })

  it("should fail to deploy contract with not enough required signatures", function() {
    var authorities = [accounts[0], accounts[1]];
    return newMainBridge({
      requiredSignatures: 0,
      authorities: authorities,
    })
      .then(function() {
        assert(false, "Contract should fail to deploy");
      }, helpers.ignoreExpectedError)
  })

  it("should fail to deploy contract with too many signatures", function() {
    var authorities = [accounts[0], accounts[1]];
    return newMainBridge({
      requiredSignatures: 3,
      authorities: authorities,
    })
      .then(function() {
        assert(false, "Contract should fail to deploy");
      }, helpers.ignoreExpectedError)
  })

  it("should create deposit event", function() {
    var meta;
    var requiredSignatures = 1;
    var authorities = [accounts[0], accounts[1]];
    let userAccount = accounts[2];
    let value = web3.toWei(1, "ether");

    return newMainBridge({
      requiredSignatures: requiredSignatures,
      authorities: authorities,
    }).then(function(instance) {
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

      return web3.eth.getTransactionReceipt(result.tx);
    }).then(function(transaction) {
      console.log("estimated gas cost of MainBridge fallback function =", transaction.gasUsed);
    })
  })

  it("deposit should fail if maxSingleDepositValue exceeded", function() {
    var meta;
    var requiredSignatures = 1;
    var authorities = [accounts[0], accounts[1]];
    let userAccount = accounts[2];
    let maxSingleDepositValue = web3.toWei(1, "ether");

    return newMainBridge({
      requiredSignatures: requiredSignatures,
      authorities: authorities,
      maxSingleDepositValue: maxSingleDepositValue,
    }).then(function(instance) {
      meta = instance;
      return meta.sendTransaction({
        value: maxSingleDepositValue,
        from: userAccount
      })
    }).then(function() {
      return meta.sendTransaction({
        value: maxSingleDepositValue + 1,
        from: userAccount
      })
        .then(function() {
          assert(false, "should fail")
        }, helpers.ignoreExpectedError)
    })
  })

  it("deposit should fail if maxTotalMainContractBalance exceeded", function() {
    var meta;
    var requiredSignatures = 1;
    var authorities = [accounts[0], accounts[1]];
    let userAccount = accounts[2];
    let maxTotalMainContractBalance = web3.toWei(2, "ether");

    return newMainBridge({
      requiredSignatures: requiredSignatures,
      authorities: authorities,
      maxTotalMainContractBalance: maxTotalMainContractBalance,
    }).then(function(instance) {
      meta = instance;
      return meta.sendTransaction({
        value: web3.toWei(1, "ether"),
        from: userAccount
      })
    }).then(function(instance) {
      return meta.sendTransaction({
        value: web3.toWei(1, "ether"),
        from: userAccount
      })
    }).then(function() {
      return meta.sendTransaction({
        value: 1,
        from: userAccount
      })
        .then(function() {
          assert(false, "should fail")
        }, helpers.ignoreExpectedError)
    })
  })

  it("should allow correct withdraw without recipient paying for gas", function() {
    var mainBridge;
    var signature;
    var authorities = [accounts[0], accounts[1]];
    var userAccount = accounts[2];
    var recipientAccount = accounts[3];
    var value = web3.toBigNumber(web3.toWei(1, "ether"));
    var mainGasPrice = web3.toBigNumber(0);
    var transactionHash = "0x1045bfe274b88120a6b1e5d01b5ec00ab5d01098346e90e7c7a3c9b8f0181c80";
    var message = helpers.createMessage(recipientAccount, value, transactionHash, mainGasPrice);

    return newMainBridge({
      requiredSignatures: 1,
      authorities: authorities,
    }).then(function(instance) {
      mainBridge = instance;

      // "charge" MainBridge so we can withdraw later
      return mainBridge.sendTransaction({
        value: value,
        from: userAccount
      })
    }).then(function(result) {
      return helpers.sign(authorities[0], message);
    }).then(function(result) {
      signature = result;
      var vrs = helpers.signatureToVRS(signature);

      return mainBridge.withdraw.estimateGas(
        [vrs.v],
        [vrs.r],
        [vrs.s],
        message,
        {from: userAccount, gasPrice: mainGasPrice}
      );
    }).then(function(result) {
      console.log("estimated gas cost of MainBridge.withdraw =", result);

      var vrs = helpers.signatureToVRS(signature);
      return mainBridge.withdraw(
        [vrs.v],
        [vrs.r],
        [vrs.s],
        message,
        {from: userAccount, gasPrice: mainGasPrice}
      );
    }).then(function(result) {
      assert.equal(1, result.logs.length, "Exactly one event should be created");
      assert.equal("Withdraw", result.logs[0].event, "Event name should be Withdraw");
      assert.equal(recipientAccount, result.logs[0].args.recipient, "Event recipient should match recipient in message");
      assert(value.equals(result.logs[0].args.value), "Event value should match value in message");
      assert.equal(transactionHash, result.logs[0].args.transactionHash);
    })
  })

  it("should allow correct withdraw with recipient paying caller for gas", function() {
    var mainBridge;
    var initialBalances;
    var signature;
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
    var mainGasPrice = web3.toBigNumber(10000);
    var transactionHash = "0x1045bfe274b88120a6b1e5d01b5ec00ab5d01098346e90e7c7a3c9b8f0181c80";
    var message = helpers.createMessage(recipientAccount, value, transactionHash, mainGasPrice);

    return newMainBridge({
      requiredSignatures: 1,
      authorities: authorities,
      estimatedGasCostOfWithdraw: estimatedGasCostOfWithdraw
    }).then(function(instance) {
      mainBridge = instance;

      return helpers.getBalances(accounts);
    }).then(function(result) {
      initialBalances = result;

      // "charge" MainBridge so we can withdraw later
      return mainBridge.sendTransaction({
        value: value,
        from: chargerAccount,
      })
    }).then(function(result) {
      return helpers.sign(authorities[0], message);
    }).then(function(result) {
      signature = result;
      var vrs = helpers.signatureToVRS(signature);

      return mainBridge.withdraw(
        [vrs.v],
        [vrs.r],
        [vrs.s],
        message,
        { from: relayerAccount, gasPrice: mainGasPrice }
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
      assert.equal(transactionHash, transactionResult.logs[0].args.transactionHash);

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

  it("withdraw should fail if gas price != requested gas price", function() {
    var mainBridge;
    var signature;
    var authorities = [accounts[0], accounts[1]];
    var userAccount = accounts[2];
    var recipientAccount = accounts[3];
    var value = web3.toBigNumber(web3.toWei(1, "ether"));
    var requestedGasPrice = web3.toBigNumber(100);
    var usedGasPrice = web3.toBigNumber(1000);
    var message = helpers.createMessage(recipientAccount, value, "0x1045bfe274b88120a6b1e5d01b5ec00ab5d01098346e90e7c7a3c9b8f0181c80", requestedGasPrice);

    return newMainBridge({
      requiredSignatures: 1,
      authorities: authorities,
      estimatedGasCostOfWithdraw: 0,
    }).then(function(instance) {
      mainBridge = instance;

      // "charge" MainBridge so we can withdraw later
      return mainBridge.sendTransaction({
        value: value,
        from: userAccount
      })
    }).then(function(result) {
      return helpers.sign(authorities[0], message);
    }).then(function(result) {
      signature = result;
      var vrs = helpers.signatureToVRS(signature);

      return mainBridge.withdraw(
        [vrs.v],
        [vrs.r],
        [vrs.s],
        message,
        {from: userAccount, gasPrice: usedGasPrice}
      ).then(function() {
        assert(false, "withdraw should fail if used gas price != requested gas price");
      }, helpers.ignoreExpectedError)
    })
  })

  it("withdraw should succeed if gas price != requested gas price but sender is receiver", function() {
    var mainBridge;
    var signature;
    var authorities = [accounts[0], accounts[1]];
    var userAccount = accounts[2];
    var recipientAccount = accounts[3];
    var value = web3.toBigNumber(web3.toWei(1, "ether"));
    var requestedGasPrice = web3.toBigNumber(100);
    var usedGasPrice = web3.toBigNumber(1000);
    var message = helpers.createMessage(recipientAccount, value, "0x1045bfe274b88120a6b1e5d01b5ec00ab5d01098346e90e7c7a3c9b8f0181c80", requestedGasPrice);

    return newMainBridge({
      requiredSignatures: 1,
      authorities: authorities,
      estimatedGasCostOfWithdraw: 0,
    }).then(function(instance) {
      mainBridge = instance;

      // "charge" MainBridge so we can withdraw later
      return mainBridge.sendTransaction({
        value: value,
        from: userAccount
      })
    }).then(function(result) {
      return helpers.sign(authorities[0], message);
    }).then(function(result) {
      signature = result;
      var vrs = helpers.signatureToVRS(signature);

      return mainBridge.withdraw(
        [vrs.v],
        [vrs.r],
        [vrs.s],
        message,
        {from: recipientAccount, gasPrice: usedGasPrice}
      )
    })
  })

  it("should revert withdraw if value is insufficient to cover costs", function() {
    var mainBridge;
    var initialBalances;
    var signature;
    var authorities = [accounts[0], accounts[1]];
    var estimatedGasCostOfWithdraw = web3.toBigNumber(100000);
    var relayerAccount = accounts[2];
    var recipientAccount = accounts[3];
    var chargerAccount = accounts[4];
    var value = estimatedGasCostOfWithdraw;
    var mainGasPrice = web3.toBigNumber(10000);
    var message = helpers.createMessage(recipientAccount, value, "0x1045bfe274b88120a6b1e5d01b5ec00ab5d01098346e90e7c7a3c9b8f0181c80", mainGasPrice);

    return newMainBridge({
      requiredSignatures: 1,
      authorities: authorities,
      estimatedGasCostOfWithdraw: estimatedGasCostOfWithdraw,
    }).then(function(instance) {
      mainBridge = instance;

      return helpers.getBalances(accounts);
    }).then(function(result) {
      initialBalances = result;

      // "charge" MainBridge so we can withdraw later
      return mainBridge.sendTransaction({
        value: value,
        from: chargerAccount,
      })
    }).then(function(result) {
      return helpers.sign(authorities[0], message);
    }).then(function(result) {
      signature = result;
      var vrs = helpers.signatureToVRS(signature);

      return mainBridge.withdraw(
        [vrs.v],
        [vrs.r],
        [vrs.s],
        message,
        { from: relayerAccount, gasPrice: mainGasPrice }
      ).then(function() {
        assert(false, "withdraw if value <= estimatedGasCostOfWithdraw should fail");
      }, helpers.ignoreExpectedError)
    })
  })

  it("should allow second withdraw with different transactionHash but same recipient and value", function() {
    var mainBridge;
    var authorities = [accounts[0], accounts[1]];
    var userAccount = accounts[2];
    var recipientAccount = accounts[3];
    var value = web3.toBigNumber(web3.toWei(1, "ether"));
    var mainGasPrice = web3.toBigNumber(10000);
    var transactionHash1 = "0x1045bfe274b88120a6b1e5d01b5ec00ab5d01098346e90e7c7a3c9b8f0181c80";
    var transactionHash2 = "0x038c79eb958a13aa71996bac27c628f33f227288bd27d5e157b97e55e08fd2b3";
    var message1 = helpers.createMessage(recipientAccount, value, transactionHash1, mainGasPrice);
    var message2 = helpers.createMessage(recipientAccount, value, transactionHash2, mainGasPrice);

    return newMainBridge({
      requiredSignatures: 1,
      authorities: authorities,
      estimatedGasCostOfWithdraw: 0,
    }).then(function(instance) {
      mainBridge = instance;
      // "charge" MainBridge so we can withdraw later
      return mainBridge.sendTransaction({
        value: value.times(2),
        from: userAccount
      })
    }).then(function(result) {
      return helpers.sign(authorities[0], message1);
    }).then(function(signature) {
      var vrs = helpers.signatureToVRS(signature);
      return mainBridge.withdraw(
        [vrs.v],
        [vrs.r],
        [vrs.s],
        message1,
        {from: authorities[0], gasPrice: mainGasPrice}
      );
    }).then(function(result) {
      assert.equal(1, result.logs.length, "Exactly one event should be created");
      assert.equal("Withdraw", result.logs[0].event, "Event name should be Withdraw");
      assert.equal(recipientAccount, result.logs[0].args.recipient, "Event recipient should match recipient in message");
      assert(value.equals(result.logs[0].args.value), "Event value should match value in message");
      assert.equal(transactionHash1, result.logs[0].args.transactionHash);

      return helpers.sign(authorities[0], message2);
    }).then(function(signature) {
      var vrs = helpers.signatureToVRS(signature);
      return mainBridge.withdraw(
        [vrs.v],
        [vrs.r],
        [vrs.s],
        message2,
        {from: authorities[0], gasPrice: mainGasPrice}
      );
    }).then(function(result) {
      assert.equal(1, result.logs.length, "Exactly one event should be created");
      assert.equal("Withdraw", result.logs[0].event, "Event name should be Withdraw");
      assert.equal(recipientAccount, result.logs[0].args.recipient, "Event recipient should match recipient in message");
      assert(value.equals(result.logs[0].args.value), "Event value should match value in message");
      assert.equal(transactionHash2, result.logs[0].args.transactionHash);
    })
  })

  it("should not allow second withdraw (replay attack) with same transactionHash but different recipient and value", function() {
    var mainBridge;
    var authorities = [accounts[0], accounts[1]];
    var userAccount = accounts[2];
    var recipientAccount = accounts[3];
    var value = web3.toBigNumber(web3.toWei(1, "ether"));
    var mainGasPrice = web3.toBigNumber(10000);
    var message1 = helpers.createMessage(recipientAccount, value, "0x1045bfe274b88120a6b1e5d01b5ec00ab5d01098346e90e7c7a3c9b8f0181c80", mainGasPrice);
    var message2 = helpers.createMessage(recipientAccount, value, "0x1045bfe274b88120a6b1e5d01b5ec00ab5d01098346e90e7c7a3c9b8f0181c80", mainGasPrice);

    return newMainBridge({
      requiredSignatures: 1,
      authorities: authorities,
      estimatedGasCostOfWithdraw: 0,
    }).then(function(instance) {
      mainBridge = instance;
      // "charge" MainBridge so we can withdraw later
      return mainBridge.sendTransaction({
        value: value.times(2),
        from: userAccount
      })
    }).then(function(result) {
      return helpers.sign(authorities[0], message1);
    }).then(function(signature) {
      var vrs = helpers.signatureToVRS(signature);
      return mainBridge.withdraw(
        [vrs.v],
        [vrs.r],
        [vrs.s],
        message1,
        {from: authorities[0], gasPrice: mainGasPrice}
      );
    }).then(function(result) {
      assert.equal(1, result.logs.length, "Exactly one event should be created");
      assert.equal("Withdraw", result.logs[0].event, "Event name should be Withdraw");
      assert.equal(recipientAccount, result.logs[0].args.recipient, "Event recipient should match recipient in message");
      assert(value.equals(result.logs[0].args.value), "Event value should match value in message");

      return helpers.sign(authorities[0], message2);
    }).then(function(signature) {
      var vrs = helpers.signatureToVRS(signature);
      return mainBridge.withdraw(
        [vrs.v],
        [vrs.r],
        [vrs.s],
        message2,
        {from: authorities[0], gasPrice: mainGasPrice}
      ).then(function() {
        assert(false, "should fail");
      }, helpers.ignoreExpectedError)
    })
  })

  it("withdraw without funds on MainBridge should fail", function() {
    var mainBridge;
    var signature;
    var authorities = [accounts[0], accounts[1]];
    var userAccount = accounts[2];
    var recipientAccount = accounts[3];
    var value = web3.toBigNumber(web3.toWei(1, "ether"));
    var mainGasPrice = web3.toBigNumber(10000);
    var message = helpers.createMessage(recipientAccount, value, "0x1045bfe274b88120a6b1e5d01b5ec00ab5d01098346e90e7c7a3c9b8f0181c80", mainGasPrice);

    return newMainBridge({
      requiredSignatures: 1,
      authorities: authorities,
      estimatedGasCostOfWithdraw: 0,
    }).then(function(instance) {
      mainBridge = instance;
      return helpers.sign(authorities[0], message);
    }).then(function(result) {
      signature = result;
      var vrs = helpers.signatureToVRS(signature);
      return mainBridge.withdraw(
        [vrs.v],
        [vrs.r],
        [vrs.s],
        message.substr(0, 83),
        {from: authorities[0], gasPrice: mainGasPrice}
      ).then(function() {
        assert(false, "should fail");
      }, helpers.ignoreExpectedError)
    })
  })

  it("should not allow withdraw with message.length too short", function() {
    var mainBridge;
    var signature;
    var authorities = [accounts[0], accounts[1]];
    var userAccount = accounts[2];
    var recipientAccount = accounts[3];
    var value = web3.toBigNumber(web3.toWei(1, "ether"));
    var mainGasPrice = web3.toBigNumber(10000);
    var message = helpers.createMessage(recipientAccount, value, "0x1045bfe274b88120a6b1e5d01b5ec00ab5d01098346e90e7c7a3c9b8f0181c80", mainGasPrice);
    // make message too short
    message = message.substr(0, 115);

    return newMainBridge({
      requiredSignatures: 1,
      authorities: authorities,
      estimatedGasCostOfWithdraw: 0,
    }).then(function(instance) {
      mainBridge = instance;

      // "charge" MainBridge so we can withdraw later
      return mainBridge.sendTransaction({
        value: value,
        from: userAccount
      })
    }).then(function(result) {
      return helpers.sign(authorities[0], message);
    }).then(function(result) {
      signature = result;
      var vrs = helpers.signatureToVRS(signature);

      return mainBridge.withdraw(
        [vrs.v],
        [vrs.r],
        [vrs.s],
        message,
        // anyone can call withdraw (provided they have the message and required signatures)
        {from: userAccount, gasPrice: mainGasPrice}
      ).then(function() {
        assert(false, "withdraw should fail");
      }, helpers.ignoreExpectedError)
    })
  })

  it("withdraw should fail if not enough signatures are provided", function() {
    var mainBridge;
    var signature;
    var authorities = [accounts[0], accounts[1]];
    var userAccount = accounts[2];
    var recipientAccount = accounts[3];
    var value = web3.toBigNumber(web3.toWei(1, "ether"));
    var mainGasPrice = web3.toBigNumber(10000);
    var message = helpers.createMessage(recipientAccount, value, "0x1045bfe274b88120a6b1e5d01b5ec00ab5d01098346e90e7c7a3c9b8f0181c80", mainGasPrice);

    return newMainBridge({
      requiredSignatures: 2,
      authorities: authorities,
      estimatedGasCostOfWithdraw: 0,
    }).then(function(instance) {
      mainBridge = instance;

      // "charge" MainBridge so we can withdraw later
      return mainBridge.sendTransaction({
        value: value,
        from: userAccount
      })
    }).then(function(result) {
      return helpers.sign(authorities[0], message);
    }).then(function(result) {
      signature = result;
      var vrs = helpers.signatureToVRS(signature);

      return mainBridge.withdraw(
        [vrs.v],
        [vrs.r],
        [vrs.s],
        message,
        // anyone can call withdraw (provided they have the message and required signatures)
        {from: userAccount, gasPrice: mainGasPrice}
      ).then(function() {
        assert(false, "should fail");
      }, helpers.ignoreExpectedError)
    })
  })

  it("withdraw should fail if duplicate signature is provided", function() {
    var mainBridge;
    var signature;
    var authorities = [accounts[0], accounts[1]];
    var userAccount = accounts[2];
    var recipientAccount = accounts[3];
    var value = web3.toBigNumber(web3.toWei(1, "ether"));
    var mainGasPrice = web3.toBigNumber(10000);
    var message = helpers.createMessage(recipientAccount, value, "0x1045bfe274b88120a6b1e5d01b5ec00ab5d01098346e90e7c7a3c9b8f0181c80", mainGasPrice);

    return newMainBridge({
      requiredSignatures: 2,
      authorities: authorities,
      estimatedGasCostOfWithdraw: 0,
    }).then(function(instance) {
      mainBridge = instance;

      // "charge" MainBridge so we can withdraw later
      return mainBridge.sendTransaction({
        value: value,
        from: userAccount
      })
    }).then(function(result) {
      return helpers.sign(authorities[0], message);
    }).then(function(result) {
      signature = result;
      var vrs = helpers.signatureToVRS(signature);

      return mainBridge.withdraw(
        [vrs.v, vrs.v],
        [vrs.r, vrs.r],
        [vrs.s, vrs.s],
        message,
        // anyone can call withdraw (provided they have the message and required signatures)
        {from: userAccount, gasPrice: mainGasPrice}
      ).then(function() {
        assert(false, "should fail");
      }, helpers.ignoreExpectedError)
    })
  })
})
