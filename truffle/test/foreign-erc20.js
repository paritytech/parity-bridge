var ForeignBridge = artifacts.require("ForeignBridge");
var helpers = require("./helpers/helpers");

contract('ForeignBridge', function(accounts) {
  it("totalSupply", function() {
	  var contract;
    var requiredSignatures = 1;
    var estimatedGasCostOfWithdraw = 0;
    var authorities = [accounts[0], accounts[1]];
	  var owner = accounts[2];
    var hash = "0xe55bb43c36cdf79e23b4adc149cdded921f0d482e613c50c6540977c213bc408";
    var value = web3.toWei(3, "ether");

    return ForeignBridge.new(requiredSignatures, authorities, estimatedGasCostOfWithdraw).then(function(instance) {
      contract = instance;

      return contract.totalSupply();
    }).then(function(result) {
      assert.equal(0, result, "initial supply should be 0");

      return contract.deposit(owner, value, hash, {from: authorities[0]});
    }).then(function(result) {

      return contract.totalSupply();
    }).then(function(result) {
      console.log(result);
      assert(result.equals(value), "deposit should increase supply");

      var homeGasPrice = 1000;
      return contract.transferHomeViaRelay(owner, value, homeGasPrice, {from: owner});
    }).then(function() {

      return contract.totalSupply();
    }).then(function(result) {
      assert.equal(0, result, "home transfer should decrease supply");
    })
  })

  it("should be able to approve others to spend tokens in their name", function() {
    var contract;
    var requiredSignatures = 1;
    var estimatedGasCostOfWithdraw = 0;
    var authorities = [accounts[0], accounts[1]];
    var owner = accounts[2];
    var spender = accounts[3];
    var receiver = accounts[4];
    var hash = "0xe55bb43c36cdf79e23b4adc149cdded921f0d482e613c50c6540977c213bc408";

    return ForeignBridge.new(requiredSignatures, authorities, estimatedGasCostOfWithdraw).then(function(instance) {
      contract = instance;

	  // deposit something so we can transfer it
      return contract.deposit(owner, web3.toWei(3, "ether"), hash, {from: authorities[0]});
    }).then(function(result) {

      return contract.allowance(owner, spender);
    }).then(function(result) {
      assert.equal(0, result, "initial allowance should be 0");

      return contract.transferFrom(owner, receiver, web3.toWei(1, "ether"), {from: spender});
    }).then(function(result) {
      assert(false, "transfer without allowance should fail");

	  // transfer 0 without allowance should work
      return contract.transferFrom(owner, receiver, 0, {from: spender});
    }).then(function(result) {
      assert.equal(1, result.logs.length, "Exactly one event should be created");
      assert.equal("Transfer", result.logs[0].event, "Event name should be Transfer");
      assert.equal(owner, result.logs[0].args.from);
      assert.equal(receiver, result.logs[0].args.to);
      assert.equal(0, result.logs[0].args.tokens);

    }, function(err) {
      return contract.approve(spender, web3.toWei(4, "ether"), {from: owner});
    }).then(function(result) {
      assert.equal(1, result.logs.length, "Exactly one event should be created");
      assert.equal("Approval", result.logs[0].event, "Event name should be Approval");
      assert.equal(owner, result.logs[0].args.tokenOwner);
      assert.equal(spender, result.logs[0].args.spender);
      assert.equal(web3.toWei(4, "ether"), result.logs[0].args.tokens);

      return contract.allowance(owner, spender);
    }).then(function(result) {
      assert.equal(web3.toWei(4, "ether"), result, "approval should set allowance");

      return contract.transferFrom(owner, receiver, web3.toWei(4, "ether"), {from: spender});
    }).then(function(result) {
      assert(false, "transferring more than balance should fail");
    }, function(err) {
      return contract.approve(spender, web3.toWei(2, "ether"), {from: owner});
    }).then(function(result) {
      assert.equal(1, result.logs.length, "Exactly one event should be created");
      assert.equal("Approval", result.logs[0].event, "Event name should be Approval");
      assert.equal(owner, result.logs[0].args.tokenOwner);
      assert.equal(spender, result.logs[0].args.spender);
      assert.equal(web3.toWei(2, "ether"), result.logs[0].args.tokens);

      return contract.allowance(owner, spender);
    }).then(function(result) {
      assert.equal(web3.toWei(2, "ether"), result, "approval should update allowance");

      return contract.transferFrom(owner, receiver, web3.toWei(2, "ether") + 2, {from: spender});
    }).then(function(result) {
      assert(false, "transferring more than allowance should fail");
    }, function(err) {
      return contract.transferFrom(owner, receiver, web3.toWei(2, "ether"), {from: spender});
    }).then(function(result) {
      assert.equal(1, result.logs.length, "Exactly one event should be created");
      assert.equal("Transfer", result.logs[0].event, "Event name should be Transfer");
      assert.equal(owner, result.logs[0].args.from);
      assert.equal(receiver, result.logs[0].args.to);
      assert.equal(web3.toWei(2, "ether"), result.logs[0].args.tokens);

      return contract.balanceOf(owner);
    }).then(function(result) {
      assert.equal(web3.toWei(1, "ether"), result, "transferring should reduce owners balance");

      return contract.balanceOf(receiver);
    }).then(function(result) {
      assert.equal(web3.toWei(2, "ether"), result, "transferring should increase receivers balance");

      return contract.balanceOf(spender);
    }).then(function(result) {
      assert.equal(0, result, "transferring should not modify spenders balance");

      return contract.allowance(owner, spender);
    }).then(function(result) {
      assert.equal(0, result, "transferring whole allowance should set allowance to 0");
    })
  })

  it("should allow user to transfer value locally", function() {
    var meta;
    var requiredSignatures = 1;
    var estimatedGasCostOfWithdraw = 0;
    var authorities = [accounts[0], accounts[1]];
    var userAccount = accounts[2];
    var userAccount2 = accounts[3];
    var user1InitialValue = web3.toWei(3, "ether");
    var transferedValue = web3.toWei(1, "ether");
    var hash = "0xe55bb43c36cdf79e23b4adc149cdded921f0d482e613c50c6540977c213bc408";
    return ForeignBridge.new(requiredSignatures, authorities, estimatedGasCostOfWithdraw).then(function(instance) {
      meta = instance;
      // top up balance so we can transfer
      return meta.deposit(userAccount, user1InitialValue, hash, { from: authorities[0] });
    }).then(function(result) {
      return meta.transfer(userAccount2, transferedValue, { from: userAccount });
    }).then(function(result) {
      assert.equal(1, result.logs.length, "Exactly one event should be created");
      assert.equal("Transfer", result.logs[0].event, "Event name should be Transfer");
      assert.equal(userAccount, result.logs[0].args.from, "Event from should be transaction sender");
      assert.equal(userAccount2, result.logs[0].args.to, "Event from should be transaction recipient");
      assert.equal(transferedValue, result.logs[0].args.tokens, "Event tokens should match transaction value");
      return Promise.all([
        meta.balances.call(userAccount),
        meta.balances.call(userAccount2)
      ])
    }).then(function(result) {
      assert.equal(web3.toWei(2, "ether"), result[0]);
      assert.equal(transferedValue, result[1]);
    })
  })

  it("should not allow user to transfer value they don't have", function() {
    var meta;
    var requiredSignatures = 1;
    var estimatedGasCostOfWithdraw = 0;
    var authorities = [accounts[0], accounts[1]];
    var userAccount = accounts[2];
    var recipientAccount = accounts[3];
    var userValue = web3.toWei(3, "ether");
    var transferedValue = web3.toWei(4, "ether");
    var hash = "0xe55bb43c36cdf79e23b4adc149cdded921f0d482e613c50c6540977c213bc408";
    return ForeignBridge.new(requiredSignatures, authorities, estimatedGasCostOfWithdraw).then(function(instance) {
      meta = instance;
      return meta.deposit(userAccount, userValue, hash, { from: authorities[0] });
    }).then(function(result) {
      return meta.transfer(recipientAccount, transferedValue, { from: userAccount });
    }).then(function(result) {
      assert(false, "transfer should fail");
    }, function(err) {
	})
  })

  it("should allow transfer of 0 value according to ERC20", function() {
    var meta;
    var requiredSignatures = 1;
    var estimatedGasCostOfWithdraw = 0;
    var authorities = [accounts[0], accounts[1]];
    var userAccount = accounts[2];
    var recipientAccount = accounts[3];
    var userValue = web3.toWei(3, "ether");
    var hash = "0xe55bb43c36cdf79e23b4adc149cdded921f0d482e613c50c6540977c213bc408";
    return ForeignBridge.new(requiredSignatures, authorities, estimatedGasCostOfWithdraw).then(function(instance) {
      meta = instance;
      return meta.deposit(userAccount, userValue, hash, { from: authorities[0] });
    }).then(function(result) {
      return meta.transfer(recipientAccount, 0, { from: userAccount });
    }).then(function(result) {
      assert.equal(1, result.logs.length, "Exactly one event should be created");
      assert.equal("Transfer", result.logs[0].event, "Event name should be Transfer");
      assert.equal(userAccount, result.logs[0].args.from, "Event from should be transaction sender");
      assert.equal(recipientAccount, result.logs[0].args.to, "Event from should be transaction recipient");
      assert.equal(0, result.logs[0].args.tokens, "Event tokens should match transaction value");
      return Promise.all([
        meta.balances.call(userAccount),
        meta.balances.call(recipientAccount)
      ])
    }).then(function(result) {
      assert.equal(userValue, result[0]);
      assert.equal(0, result[1]);
    })
  })

  it("transfer that results in overflow should fail", function() {
    var meta;
    var requiredSignatures = 1;
    var estimatedGasCostOfWithdraw = 0;
    var authorities = [accounts[0], accounts[1]];
    var userAccount = accounts[2];
    var recipientAccount = accounts[3];
    var maxValue = web3.toWei("0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff", "wei");
    var hash = "0xe55bb43c36cdf79e23b4adc149cdded921f0d482e613c50c6540977c213bc408";
    return ForeignBridge.new(requiredSignatures, authorities, estimatedGasCostOfWithdraw).then(function(instance) {
      meta = instance;
      return meta.deposit(recipientAccount, maxValue, hash, { from: authorities[0] });
    }).then(function(result) {
      return meta.deposit(userAccount, 1, hash, { from: authorities[0] });
    }).then(function(result) {
      return meta.transfer(recipientAccount, 1, { from: userAccount });
    }).then(function(result) {
      assert(false, "transfer should fail");
    }, function(err) {
    })
  })

  it("transferFrom that results in overflow should fail", function() {
    var meta;
    var requiredSignatures = 1;
    var estimatedGasCostOfWithdraw = 0;
    var authorities = [accounts[0], accounts[1]];
    var userAccount = accounts[2];
    var spenderAccount = accounts[3];
    var recipientAccount = accounts[4];
    var maxValue = web3.toWei("0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff", "wei");
    var hash = "0xe55bb43c36cdf79e23b4adc149cdded921f0d482e613c50c6540977c213bc408";
    return ForeignBridge.new(requiredSignatures, authorities, estimatedGasCostOfWithdraw).then(function(instance) {
      meta = instance;
      return meta.deposit(recipientAccount, maxValue, hash, { from: authorities[0] });
    }).then(function(result) {
      return meta.deposit(userAccount, 1, hash, { from: authorities[0] });
    }).then(function(result) {
	  return meta.approve(spenderAccount, 1, {from: userAccount});
    }).then(function(result) {
      return meta.transferFrom(userAccount, recipientAccount, 1, { from: spenderAccount });
    }).then(function(result) {
      assert(false, "transfer should fail");
    }, function(err) {
    })
  })
})
