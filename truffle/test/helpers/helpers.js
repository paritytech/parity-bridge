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
