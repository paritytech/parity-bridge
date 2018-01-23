var ForeignBridge = artifacts.require("ForeignBridge");
var helpers = require("./helpers/helpers");

contract('ForeignBridge', function(accounts) {
  it("totalSupply", function() {
	var contract;
    var requiredSignatures = 1;
    var authorities = [accounts[0], accounts[1]];
	var owner = accounts[2];
    var hash = "0xe55bb43c36cdf79e23b4adc149cdded921f0d482e613c50c6540977c213bc408";
    var value = web3.toWei(3, "ether");

    return ForeignBridge.new(requiredSignatures, authorities).then(function(instance) {
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

	  return contract.transferHomeViaRelay(owner, value, {from: owner});
	}).then(function() {

      return contract.totalSupply();
    }).then(function(result) {
	  assert.equal(0, result, "home transfer should decrease supply");
	})
  })

  it("should be able to approve others to spend tokens in their name", function() {
	var contract;
    var requiredSignatures = 1;
    var authorities = [accounts[0], accounts[1]];
	var owner = accounts[2];
	var approved = accounts[3];
	var receiver = accounts[4];
    var hash = "0xe55bb43c36cdf79e23b4adc149cdded921f0d482e613c50c6540977c213bc408";

    return ForeignBridge.new(requiredSignatures, authorities).then(function(instance) {
	  contract = instance;

	  // deposit something so we can use it
	  return contract.deposit(owner, web3.toWei(3, "ether"), hash, {from: authorities[0]});
	}).then(function(result) {

      return contract.allowance(owner, approved);
    }).then(function(result) {
	  assert.equal(0, result, "user should have 0 allowance initially");

      return contract.transferFrom(owner, receiver, web3.toWei(1, "ether"));
    }).then(function(result) {
	  assert(false, "transfer without allowance should fail");

	}, function(err) {
	  return contract.approve(approved, web3.toWei(2, "ether"), {from: owner});
	}).then(function(result) {
	  console.log(result);
      assert.equal(1, result.logs.length, "Exactly one event should be created");
      assert.equal("Approval", result.logs[0].event, "Event name should be Approval");
      assert.equal(owner, result.logs[0].args.tokenOwner);
      assert.equal(approved, result.logs[0].args.spender);
      assert.equal(web3.toWei(2, "ether"), result.logs[0].args.tokens);
	})
  })
})
