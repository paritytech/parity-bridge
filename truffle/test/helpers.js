// solidity Helpers library
var Helpers = artifacts.require("HelpersTest");
// testing helpers
var helpers = require("./helpers/helpers");

contract("Helpers", function() {
  it("`addressArrayContains` should function correctly", function() {
    var addresses = [
      "0xd4f04f18d253f831e5b9bcfde7f20450562e03da",
      "0x46ee1abbcd7215364174f84c3cbc4770d45966e9",
      "0x5ef98710ff315ded660fe757bf7a861114287c1e",
    ];
    var otherAddress = "0x006e27b6a72e1f34c626762f3c4761547aff1421";
    var library;
    return Helpers.new().then(function(instance) {
      library = instance;

      return library.addressArrayContains.call([], otherAddress);
    }).then(function(result) {
      assert.equal(result, false, "should return false for empty array");

      return library.addressArrayContains.call([otherAddress], otherAddress);
    }).then(function(result) {
      assert.equal(result, true, "should return true for singleton array containing value");

      return library.addressArrayContains.call([addresses[0]], addresses[1]);
    }).then(function(result) {
      assert.equal(result, false, "should return false for singleton array not containing value");

      return library.addressArrayContains.call(addresses, addresses[0]);
    }).then(function(result) {
      assert.equal(result, true);

      return library.addressArrayContains.call(addresses, addresses[1]);
    }).then(function(result) {
      assert.equal(result, true);

      return library.addressArrayContains.call(addresses, addresses[2]);
    }).then(function(result) {
      assert.equal(result, true);

      return library.addressArrayContains.call(addresses, otherAddress);
    }).then(function(result) {
      assert.equal(result, false);
    })
  })

  it("`intToString` should convert int to string", function() {
    var numbersFrom1To100 = helpers.range(1, 101);
    var library;
    return Helpers.new().then(function(instance) {
      library = instance;

      return library.intToString.call(0)
    }).then(function(result) {
      assert.equal(result, "0");

      return Promise.all(numbersFrom1To100.map(function(number) {
        return library.intToString.call(number);
      }));
    }).then(function(result) {
      assert.deepEqual(result, numbersFrom1To100.map(function(number) {
        return number.toString();
      }), "should convert numbers from 1 to 100 correctly");

      return library.intToString.estimateGas(1);
    }).then(function(result) {
      console.log("estimated gas cost of Helpers.intToString(1)", result);

      return library.intToString.call(1234)
    }).then(function(result) {
      assert.equal(result, "1234");

      return library.intToString.call(12345678)
    }).then(function(result) {
      assert.equal(result, "12345678");

      return library.intToString.estimateGas(12345678)
    }).then(function(result) {
      console.log("estimated gas cost of Helpers.intToString(12345678)", result);

      return library.intToString.call(web3.toBigNumber("131242344353464564564574574567456"));
    }).then(function(result) {
      assert.equal(result, "131242344353464564564574574567456");
    })
  })
})
