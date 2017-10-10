var HomeBridge = artifacts.require("HomeBridge");

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

  it("should fail to deploy contract with to many signatures", function() {
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
})
