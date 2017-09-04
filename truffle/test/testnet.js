var KovanBridge = artifacts.require("KovanBridge");

contract('KovanBridge', function(accounts) {
  it("should deploy contract", function() {
    var meta;
    var requiredSignatures = 1;
    var authorities = [accounts[0], accounts[1]];

    return KovanBridge.new(requiredSignatures, authorities).then(function(instance) {
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
    return KovanBridge.new(0, authorities).then(function(_) {
      assert(false, "Contract should fail to deploy");
    }, function(err) {
      // do nothing
    })
  })

  it("should fail to deploy contract with to many signatures", function() {
    var authorities = [accounts[0], accounts[1]];
    return KovanBridge.new(3, authorities).then(function(_) {
      assert(false, "Contract should fail to deploy");
    }, function(err) {
      // do nothing
    })
  })

  it("should allow a single authority to confirm a deposit", function() {
    var meta;
    var requiredSignatures = 1;
    var authorities = [accounts[0], accounts[1]];
    var user_account = accounts[2];
    var value = web3.toWei(1, "ether");
    var hash = "0xe55bb43c36cdf79e23b4adc149cdded921f0d482e613c50c6540977c213bc408";

    return KovanBridge.new(requiredSignatures, authorities).then(function(instance) {
      meta = instance;
      return meta.deposit(user_account, value, hash, { from: authorities[0] });
    }).then(function(result) {
      assert.equal(1, result.logs.length, "Exactly one event should be created");
      assert.equal("Deposit", result.logs[0].event, "Event name should be Deposit");
      assert.equal(user_account, result.logs[0].args.recipient, "Event recipient should be transaction sender");
      assert.equal(value, result.logs[0].args.value, "Event value should match deposited ether");
      return meta.balances.call(user_account);
    }).then(function(result) {
      assert.equal(value, result, "Contract balance should change");
    })
  })

  it("should require 2 authorities to confirm deposit", function() {
    var meta;
    var requiredSignatures = 2;
    var authorities = [accounts[0], accounts[1]];
    var user_account = accounts[2];
    var value = web3.toWei(1, "ether");
    var hash = "0xe55bb43c36cdf79e23b4adc149cdded921f0d482e613c50c6540977c213bc408";

    return KovanBridge.new(requiredSignatures, authorities).then(function(instance) {
      meta = instance;
      return meta.deposit(user_account, value, hash, { from: authorities[0] });
    }).then(function(result) {
      assert.equal(0, result.logs.length, "No event should be created");
      return meta.balances.call(user_account);
    }).then(function(result) {
      assert.equal(web3.toWei(0, "ether"), result, "Contract balance should not change yet");
      return meta.deposit(user_account, value, hash, { from: authorities[1] });
    }).then(function(result) {
      assert.equal(1, result.logs.length, "Exactly one event should be created");
      assert.equal("Deposit", result.logs[0].event, "Event name should be Deposit");
      assert.equal(user_account, result.logs[0].args.recipient, "Event recipient should be transaction sender");
      assert.equal(value, result.logs[0].args.value, "Event value should match deposited ether");
      return meta.balances.call(user_account);
    }).then(function(result) {
      assert.equal(value, result, "Contract balance should change");
    })
  })

  it("should ignore misbehaving authority when confirming deposit", function() {
    var meta;
    var requiredSignatures = 2;
    var authorities = [accounts[0], accounts[1], accounts[2]];
    var user_account = accounts[3];
    var invalid_value = web3.toWei(2, "ether");
    var value = web3.toWei(1, "ether");
    var hash = "0xe55bb43c36cdf79e23b4adc149cdded921f0d482e613c50c6540977c213bc408";

    return KovanBridge.new(requiredSignatures, authorities).then(function(instance) {
      meta = instance;
      return meta.deposit(user_account, value, hash, { from: authorities[0] });
    }).then(function(result) {
      assert.equal(0, result.logs.length, "No event should be created yet");
      return meta.deposit(user_account, invalid_value, hash, { from: authorities[1] });
    }).then(function(result) {
      assert.equal(0, result.logs.length, "Misbehaving authority should be ignored");
      return meta.deposit(user_account, value, hash, { from: authorities[2] })
    }).then(function(result) {
      assert.equal(1, result.logs.length, "Exactly one event should be created");
      assert.equal("Deposit", result.logs[0].event, "Event name should be Deposit");
      assert.equal(user_account, result.logs[0].args.recipient, "Event recipient should be transaction sender");
      assert.equal(value, result.logs[0].args.value, "Event value should match deposited ether");
      return meta.balances.call(user_account);
    }).then(function(result) {
      assert.equal(value, result, "Contract balance should change");
    })
  })

  it("should allow user to transfer value internally", function() {
  })

  it("should not allow user to transfer value", function() {
  })

  it("should allow user to trigger withdraw", function() {
  })

})
