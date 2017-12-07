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
      // estimate gas of fallback function
      return web3.eth.estimateGas({
        to: meta.address,
        value: value,
        from: user_account
      });
    }).then(function(result) {
      console.log("estimated gas cost of HomeBridge fallback function =", result, "wei");

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

  function signatureToVRS(signature) {
    signature = strip0x(signature);
    var v = parseInt(signature.substr(64 * 2), 16);
    var r = "0x" + signature.substr(0, 32 * 2);
    var s = "0x" + signature.substr(32 * 2, 32 * 2);
    return {v: v, r: r, s: s};
  }

  // geth && testrpc has different output of eth_sign than parity
  // https://github.com/ethereumjs/testrpc/issues/243#issuecomment-326750236
  function normalizeSignature(signature) {
    signature = strip0x(signature);

    // increase v by 27...
    return "0x" + signature.substr(0, 128) + (parseInt(signature.substr(128), 16) + 27).toString(16);
  }

  // strips leading "0x"
  function strip0x(input) {
    return input.substr(2);
  }

  function bigNumberToHexString(num) {
    web3._extend.utils.isBigNumber(num);
    var quotient = num;
    var result = "";
    while (quotient > 0) {
      var remainderDec = quotient.mod(16).toNumber();
      assert(remainderDec < 16);
      var remainderHexDigit = remainderDec.toString(16);
      assert.equal(remainderHexDigit.length, 1)
      result = remainderHexDigit + result;
      quotient = quotient.dividedToIntegerBy(16);
    }
    return "0x" + result;
  }

  function bigNumberToPaddedBytes32(num) {
    web3._extend.utils.isBigNumber(num);
    var result = strip0x(bigNumberToHexString(num));
    while (result.length < 64) {
      result = "0" + result;
    }
    return "0x" + result;
  }

  function createMessage(recipient, value, transactionHash) {
    web3._extend.utils.isBigNumber(value);
    recipient = strip0x(recipient);
    assert.equal(recipient.length, 20 * 2);

    transactionHash = strip0x(transactionHash);
    assert.equal(transactionHash.length, 32 * 2);

    var value = strip0x(bigNumberToPaddedBytes32(value));
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
      return homeBridge.sendTransaction({
        value: value,
        from: user_account
      })
    }).then(function(result) {
      message = createMessage(recipient_account, value, result.tx);
      return sign(authorities[0], message);
    }).then(function(result) {
      signature = result;
      var vrs = signatureToVRS(signature);

      return homeBridge.withdraw.estimateGas(
        [vrs.v],
        [vrs.r],
        [vrs.s],
        message,
        {from: authorities[0]}
      );
    }).then(function(result) {
      console.log("estimated gas cost of HomeBridge.withdraw =", result, "wei");

      var vrs = signatureToVRS(signature);
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

  it("should not allow withdraw from misbehaving authority", function() {
    // TODO change a byte in signature
  })

  it("should not allow withdraw without funds", function() {
  })

  it("should not allow reentry (DAO-bug) in withdraw", function() {
  })

  it("should not allow duplicated withdraws", function() {
  })

  it("should estimate gas cost of withdraw", function() {
  })
})
