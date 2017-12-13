// returns a Promise that resolves with a hex string that is the signature of
// `data` signed with the key of `address`
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

// strips leading "0x" if present
function strip0x(input) {
  return input.replace(/^0x/, "");
}
module.exports.strip0x = strip0x;

// extracts and returns the `v`, `r` and `s` values from a `signature`.
// all inputs and outputs are hex strings with leading '0x'.
function signatureToVRS(signature) {
  assert.equal(signature.length, 2 + 32 * 2 + 32 * 2 + 2);
  signature = strip0x(signature);
  var v = parseInt(signature.substr(64 * 2), 16);
  var r = "0x" + signature.substr(0, 32 * 2);
  var s = "0x" + signature.substr(32 * 2, 32 * 2);
  return {v: v, r: r, s: s};
}
module.exports.signatureToVRS = signatureToVRS;

// returns BigNumber `num` converted to a little endian hex string.
// `num` must represent an unsigned integer
function bigNumberToHexString(num) {
  assert(web3._extend.utils.isBigNumber(num));
  assert(num.isInteger());
  assert(!num.isNegative());
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
