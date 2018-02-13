var Message = artifacts.require("MessageTest");
var helpers = require("./helpers/helpers");

contract("Message", function(accounts) {
  var recipientAccount = accounts[0];
  var value = web3.toBigNumber(web3.toWei(1, "ether"));
  var transactionHash = "0x1045bfe274b88120a6b1e5d01b5ec00ab5d01098346e90e7c7a3c9b8f0181c80";
  var homeGasPrice = web3.toBigNumber(web3.toWei(3, "gwei"));
  var message = helpers.createMessage(recipientAccount, value, transactionHash, homeGasPrice);

  it("should extract value", function() {
    return Message.new().then(function(instance) {
      return instance.getValue.call(message)
    }).then(function(result) {
      assert(result.equals(value));
    })
  })

  it("should extract recipient", function() {
    return Message.new().then(function(instance) {
      return instance.getRecipient.call(message)
    }).then(function(result) {
      assert.equal(result, recipientAccount);
    })
  })

  it("should extract transactionHash", function() {
    return Message.new().then(function(instance) {
      return instance.getTransactionHash.call(message)
    }).then(function(result) {
      assert.equal(result, transactionHash);
    })
  })

  it("should extract homeGasPrice", function() {
    return Message.new().then(function(instance) {
      return instance.getHomeGasPrice.call(message)
    }).then(function(result) {
      assert(result.equals(homeGasPrice));
    })
  })
})
