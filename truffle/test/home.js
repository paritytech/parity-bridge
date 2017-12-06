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
    // strip 0x
    signature = signature.substr(2);
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

  function padLeft(input, padding, requestedLength) {
      var output = input;
      while (output.length < requestedLength) {
          output = padding + output;
      }
      return output;
  }

  function padRight(input, padding, requestedLength) {
      var output = input;
      while (output.length < requestedLength) {
          output = output + padding;
      }
      return output;
  }

  function hexToBytes(hex) {
    for (var bytes = [], c = 0; c < hex.length; c+=2)
      bytes.push(parseInt(hex.substr(c, 2), 16));
    return bytes;
  }

  function bigNumberToPaddedBytes32(num) {
      var n = num.toString(16).replace(/^0x/, '');
      while (n.length < 64) {
          n = "0" + n;
      }
      return "0x" + n;
  }

  function createMessage(recipient, value, transactionHash) {
    recipient = strip0x(recipient);
    // console.log("recipient =", recipient);
    assert.equal(recipient.length, 20 * 2);

    transactionHash = strip0x(transactionHash);
    // console.log("transactionHash =", transactionHash);
    assert.equal(transactionHash.length, 32 * 2);

    var value = strip0x(bigNumberToPaddedBytes32(value));
    var value = padLeft("1", 0, 64);
    console.log("value =", value);
    assert.equal(value.length, 64);
    var message = "0x" + recipient + value + transactionHash;
    var expectedMessageLength = (20 + 32 + 32) * 2 + 2;
    console.log("expectedMessageLength =", expectedMessageLength);
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
    var value = web3.toWei(1, "ether");
    console.log("value.toString() =", value.toString());
    var m1 = "0x111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111";
    console.log("m1.length =", m1.length);

    return HomeBridge.new(requiredSignatures, authorities).then(function(instance) {
      homeBridge = instance;
      return Promise.all([
        web3.eth.getBalance(authorities[0]),
        web3.eth.getBalance(authorities[1]),
        web3.eth.getBalance(user_account)
      ]);
    }).then(function(result) {
      console.log(result);
      return homeBridge.sendTransaction({
        value: value,
        from: user_account
      })
    }).then(function(result) {
      console.log("recipient =", user_account);
      console.log("hash =", result.tx);
      message = createMessage(user_account, 100, result.tx);
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
      console.log("estimated gas cost of HomeBridge.withdraw =", result);

      var vrs = signatureToVRS(signature);
      return homeBridge.withdraw(
        [vrs.v],
        [vrs.r],
        [vrs.s],
        message,
        {from: authorities[0]}
      );
    }).then(function(result) {
      result.logs.forEach(function(log) {
        console.log(log);
      });
      console.log(result.logs[0].args.value.toString());
      console.log("withdraw success");
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
