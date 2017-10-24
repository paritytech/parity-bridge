var ForeignBridge = artifacts.require("ForeignBridge");

contract('ForeignBridge', function(accounts) {
  it("should deploy contract", function() {
    var meta;
    var requiredSignatures = 1;
    var authorities = [accounts[0], accounts[1]];

    return ForeignBridge.new(requiredSignatures, authorities).then(function(instance) {
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
    return ForeignBridge.new(0, authorities).then(function(_) {
      assert(false, "Contract should fail to deploy");
    }, function(err) {
      // do nothing
    })
  })

  it("should fail to deploy contract with to many signatures", function() {
    var authorities = [accounts[0], accounts[1]];
    return ForeignBridge.new(3, authorities).then(function(_) {
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

    return ForeignBridge.new(requiredSignatures, authorities).then(function(instance) {
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

    return ForeignBridge.new(requiredSignatures, authorities).then(function(instance) {
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

    return ForeignBridge.new(requiredSignatures, authorities).then(function(instance) {
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
      assert.equal(value, result.logs[0].args.value, "Event value should match transaction value");
      return meta.balances.call(user_account);
    }).then(function(result) {
      assert.equal(value, result, "Contract balance should change");
    })
  })

  it("should allow user to transfer value internally", function() {
    var meta;
    var requiredSignatures = 1;
    var authorities = [accounts[0], accounts[1]];
    var user_account = accounts[2];
    var user_account2 = accounts[3];
    var value = web3.toWei(3, "ether");
    var value2 = web3.toWei(1, "ether");
    var hash = "0xe55bb43c36cdf79e23b4adc149cdded921f0d482e613c50c6540977c213bc408";
    return ForeignBridge.new(requiredSignatures, authorities).then(function(instance) {
      meta = instance;
      return meta.deposit(user_account, value, hash, { from: authorities[0] });
    }).then(function(result) {
      return meta.transfer(user_account2, value2, false, { from: user_account });
    }).then(function(result) {
      assert.equal(1, result.logs.length, "Exactly one event should be created");
      assert.equal("Transfer", result.logs[0].event, "Event name should be Transfer");
      assert.equal(user_account, result.logs[0].args.from, "Event from should be transaction sender");
      assert.equal(user_account2, result.logs[0].args.to, "Event from should be transaction recipient");
      assert.equal(value2, result.logs[0].args.value, "Event value should match transaction value");
      return Promise.all([
        meta.balances.call(user_account),
        meta.balances.call(user_account2)
      ])
    }).then(function(result) {
      assert.equal(web3.toWei(2, "ether"), result[0]);
      assert.equal(web3.toWei(1, "ether"), result[1]);
    })
  })

  it("should not allow user to transfer value", function() {
    var meta;
    var requiredSignatures = 1;
    var authorities = [accounts[0], accounts[1]];
    var user_account = accounts[2];
    var user_account2 = accounts[3];
    var value = web3.toWei(3, "ether");
    var value2 = web3.toWei(4, "ether");
    var hash = "0xe55bb43c36cdf79e23b4adc149cdded921f0d482e613c50c6540977c213bc408";
    return ForeignBridge.new(requiredSignatures, authorities).then(function(instance) {
      meta = instance;
      return meta.deposit(user_account, value, hash, { from: authorities[0] });
    }).then(function(result) {
      return meta.transfer(user_account2, value2, false, { from: user_account });
    }).then(function(result) {
      assert(false, "Transfer should fail");
    }, function(err) {
    })
  })

  it("should fail to transfer 0 value", function() {
    var meta;
    var requiredSignatures = 1;
    var authorities = [accounts[0], accounts[1]];
    var user_account = accounts[2];
    var user_account2 = accounts[3];
    var value = web3.toWei(3, "ether");
    var value2 = web3.toWei(0, "ether");
    var hash = "0xe55bb43c36cdf79e23b4adc149cdded921f0d482e613c50c6540977c213bc408";
    return ForeignBridge.new(requiredSignatures, authorities).then(function(instance) {
      meta = instance;
      return meta.deposit(user_account, value, hash, { from: authorities[0] });
    }).then(function(result) {
      return meta.transfer(user_account2, value2, false, { from: user_account });
    }).then(function(result) {
      assert(false, "Transfer of value 0 should fail");
    }, function (err) {
    })
  })

  it("should fail to transfer with value overflow", function() {
    var meta;
    var requiredSignatures = 1;
    var authorities = [accounts[0], accounts[1]];
    var user_account = accounts[2];
    var user_account2 = accounts[3];
    var value = web3.toWei("0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff", "wei");
    var value2 = web3.toWei(1, "wei");
    var hash = "0xe55bb43c36cdf79e23b4adc149cdded921f0d482e613c50c6540977c213bc408";
    return ForeignBridge.new(requiredSignatures, authorities).then(function(instance) {
      meta = instance;
      return Promise.all([
        meta.deposit(user_account, value, hash, { from: authorities[0] }),
        meta.deposit(user_account2, value2, hash, { from: authorities[0] }),
      ])
    }).then(function(result) {
      return meta.transfer(user_account2, value, false, { from: user_account });
    }).then(function(result) {
      assert(false, "Transfer with overflow should fail");
    }, function (err) {
    })
  })

  it("should allow user to trigger withdraw", function() {
    var meta;
    var requiredSignatures = 1;
    var authorities = [accounts[0], accounts[1]];
    var user_account = accounts[2];
    var user_account2 = accounts[3];
    var value = web3.toWei(3, "ether");
    var value2 = web3.toWei(1, "ether");
    var hash = "0xe55bb43c36cdf79e23b4adc149cdded921f0d482e613c50c6540977c213bc408";
    return ForeignBridge.new(requiredSignatures, authorities).then(function(instance) {
      meta = instance;
      return meta.deposit(user_account, value, hash, { from: authorities[0] });
    }).then(function(result) {
      return meta.transfer(user_account2, value2, true, { from: user_account });
    }).then(function(result) {
      assert.equal(1, result.logs.length, "Exactly one event should be created");
      assert.equal("Withdraw", result.logs[0].event, "Event name should be Withdraw");
      assert.equal(user_account2, result.logs[0].args.recipient, "Event recipient should be equal to transaction recipient");
      assert.equal(value2, result.logs[0].args.value, "Event value should match transaction value");
      return Promise.all([
        meta.balances.call(user_account),
        meta.balances.call(user_account2)
      ])
    }).then(function(result) {
      assert.equal(web3.toWei(2, "ether"), result[0]);
      assert.equal(web3.toWei(0, "ether"), result[1]);
    })
  })

  function sign(address, data) {
    return new Promise(function(resolve, reject) {
      web3.eth.sign(address, data, function(err, result) {
        if (err !== null) {
          return reject(err);
        } else {
          return resolve(normalizeSignature(result));
          //return resolve(result);
        }
      })
    })
  }

  // geth && testrpc has different output of eth_sign than parity
  // https://github.com/ethereumjs/testrpc/issues/243#issuecomment-326750236
  function normalizeSignature(signature) {
    // strip 0x
    signature = signature.substr(2);

    // increase v by 27...
    return "0x" + signature.substr(0, 128) + (parseInt(signature.substr(128), 16) + 27).toString(16);
  }

  it("should successfully submit signature and trigger CollectedSignatures event", function() {
    var meta;
    var signature;
    var requiredSignatures = 1;
    var authorities = [accounts[0], accounts[1]];
    var message = "0x111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111";
    return ForeignBridge.new(requiredSignatures, authorities).then(function(instance) {
      meta = instance;
      return sign(authorities[0], message);
    }).then(function(result) {
      signature = result;
      return meta.submitSignature(result, message, { from: authorities[0] });
    }).then(function(result) {
      assert.equal(1, result.logs.length, "Exactly one event should be created");
      assert.equal("CollectedSignatures", result.logs[0].event, "Event name should be CollectedSignatures");
      assert.equal(authorities[0], result.logs[0].args.authority, "Event authority should be equal to transaction sender");
      return Promise.all([
        meta.signature.call(result.logs[0].args.messageHash, 0),
        meta.message(result.logs[0].args.messageHash),
      ])
    }).then(function(result) {
      assert.equal(signature, result[0]);
      assert.equal(message, result[1]);
    })
  })

  it("should successfully submit signature but not trigger CollectedSignatures event", function() {
    var meta;
    var requiredSignatures = 2;
    var authorities = [accounts[0], accounts[1]];
    var message = "0x111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111";
    return ForeignBridge.new(requiredSignatures, authorities).then(function(instance) {
      meta = instance;
      return sign(authorities[0], message);
    }).then(function(result) {
      return meta.submitSignature(result, message, { from: authorities[0] });
    }).then(function(result) {
      assert.equal(0, result.logs.length, "No events should be created");
    })
  })

  it("should be able to collect signatures for multiple events in parallel", function() {
    var meta;
    var signatures_for_message = [];
    var signatures_for_message2 = [];
    var requiredSignatures = 2;
    var authorities = [accounts[0], accounts[1]];
    var message = "0x111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111";
    var message2 = "0x111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111112";
    return ForeignBridge.new(requiredSignatures, authorities).then(function(instance) {
      meta = instance;
      return Promise.all([
        sign(authorities[0], message),
        sign(authorities[1], message),
        sign(authorities[0], message2),
        sign(authorities[1], message2),
      ]);
    }).then(function(result) {
      signatures_for_message.push(result[0]);
      signatures_for_message.push(result[1]);
      signatures_for_message2.push(result[2]);
      signatures_for_message2.push(result[3]);
      return meta.submitSignature(signatures_for_message[0], message, { from: authorities[0] });
    }).then(function(result) {
      assert.equal(0, result.logs.length, "No events should be created");
      return meta.submitSignature(signatures_for_message2[1], message2, { from: authorities[1] });
    }).then(function(result) {
      assert.equal(0, result.logs.length, "No events should be created");
      return meta.submitSignature(signatures_for_message2[0], message2, { from: authorities[0] });
    }).then(function(result) {
      assert.equal(1, result.logs.length, "Exactly one event should be created");
      assert.equal("CollectedSignatures", result.logs[0].event, "Event name should be CollectedSignatures");
      assert.equal(authorities[0], result.logs[0].args.authority, "Event authority should be equal to transaction sender");
      return Promise.all([
        meta.signature.call(result.logs[0].args.messageHash, 0),
        meta.signature.call(result.logs[0].args.messageHash, 1),
        meta.message(result.logs[0].args.messageHash),
      ])
    }).then(function(result) {
      assert.equal(signatures_for_message2[1], result[0]);
      assert.equal(signatures_for_message2[0], result[1]);
      assert.equal(message2, result[2]);
      return meta.submitSignature(signatures_for_message[1], message, { from: authorities[1] });
    }).then(function(result) {
      assert.equal(1, result.logs.length, "Exactly one event should be created");
      assert.equal("CollectedSignatures", result.logs[0].event, "Event name should be CollectedSignatures");
      assert.equal(authorities[1], result.logs[0].args.authority, "Event authority should be equal to transaction sender");
      return Promise.all([
        meta.signature.call(result.logs[0].args.messageHash, 0),
        meta.signature.call(result.logs[0].args.messageHash, 1),
        meta.message(result.logs[0].args.messageHash),
      ])
    }).then(function(result) {
      assert.equal(signatures_for_message[0], result[0]);
      assert.equal(signatures_for_message[1], result[1]);
      assert.equal(message, result[2]);
    })
  })

  it("should not be possible to submit to short message", function() {
    var meta;
    var requiredSignatures = 1;
    var authorities = [accounts[0], accounts[1]];
    var message = "0x1111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111";
    return ForeignBridge.new(requiredSignatures, authorities).then(function(instance) {
      meta = instance;
      return sign(authorities[0], message);
    }).then(function(result) {
      return meta.submitSignature(result, message, { from: authorities[0] });
    }).then(function(result) {
      assert(false, "submitSignature should fail");
    }, function (err) {
      // nothing
    })
  })

  it("should not be possible to submit different message then the signed one", function() {
    var meta;
    var requiredSignatures = 1;
    var authorities = [accounts[0], accounts[1]];
    var message = "0x111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111";
    var message2 = "0x111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111112";
    return ForeignBridge.new(requiredSignatures, authorities).then(function(instance) {
      meta = instance;
      return sign(authorities[0], message);
    }).then(function(result) {
      return meta.submitSignature(result, message2, { from: authorities[0] });
    }).then(function(result) {
      assert(false, "submitSignature should fail");
    }, function (err) {
      // nothing
    })
  })

  it("should not be possible to submit signature signed by different authority", function() {
    var meta;
    var requiredSignatures = 1;
    var authorities = [accounts[0], accounts[1]];
    var message = "0x111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111";
    return ForeignBridge.new(requiredSignatures, authorities).then(function(instance) {
      meta = instance;
      return sign(authorities[0], message);
    }).then(function(result) {
      return meta.submitSignature(result, message, { from: authorities[1] });
    }).then(function(result) {
      assert(false, "submitSignature should fail");
    }, function (err) {
      // nothing
    })
  })

  it("should not be possible to submit signature twice", function() {
    var meta;
    var requiredSignatures = 0;
    var authorities = [accounts[0], accounts[1]];
    var message = "0x111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111";
    return ForeignBridge.new(requiredSignatures, authorities).then(function(instance) {
      meta = instance;
      return sign(authorities[0], message);
    }).then(function(result) {
      return meta.submitSignature(result, message, { from: authorities[0] });
    }).then(function(result) {
      return meta.submitSignature(result, message, { from: authorities[0] });
    }).then(function(result) {
      assert(false, "submitSignature should fail");
    }, function (err) {
      // nothing
    })
  })
})
