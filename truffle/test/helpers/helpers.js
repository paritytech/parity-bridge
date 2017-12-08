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
module.exports.sign = sign;

// geth && testrpc has different output of eth_sign than parity
// https://github.com/ethereumjs/testrpc/issues/243#issuecomment-326750236
function normalizeSignature(signature) {
  signature = strip0x(signature);

  // increase v by 27...
  return "0x" + signature.substr(0, 128) + (parseInt(signature.substr(128), 16) + 27).toString(16);
}
module.exports.normalizeSignature = normalizeSignature;

function signatureToVRS(signature) {
  signature = strip0x(signature);
  var v = parseInt(signature.substr(64 * 2), 16);
  var r = "0x" + signature.substr(0, 32 * 2);
  var s = "0x" + signature.substr(32 * 2, 32 * 2);
  return {v: v, r: r, s: s};
}
module.exports.signatureToVRS = signatureToVRS;

// strips leading "0x"
function strip0x(input) {
  return input.substr(2);
}
module.exports.strip0x = strip0x;

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
module.exports.bigNumberToHexString = bigNumberToHexString;

function bigNumberToPaddedBytes32(num) {
  web3._extend.utils.isBigNumber(num);
  var result = strip0x(bigNumberToHexString(num));
  while (result.length < 64) {
    result = "0" + result;
  }
  return "0x" + result;
}
module.exports.bigNumberToPaddedBytes32 = bigNumberToPaddedBytes32;
