var MainBridge = artifacts.require("MainBridge");
var MainExample = artifacts.require("MainExample");
var SideBridge = artifacts.require("SideBridge");
var SideExample = artifacts.require("SideExample");
var helpers = require("./helpers/helpers");

const Promisify = (inner) =>
  new Promise((resolve, reject) =>
	  inner((err, res) => {
		  if (err) {
			  reject(err);
		  } else {
			  resolve(res);
		  }
	  })
  );

contract('MainBridge', function(accounts) {
  it("should deploy contract", function() {
	var user = accounts[0];

    var mainBridge;
	var mainExample;
	var sideBridge;
	var sideExample;

    return MainBridge.new().then(function(instance) {
		mainBridge = instance;
		console.log("MainBridge deployed");

		return MainExample.new(mainBridge.address);
	}).then(function(instance) {
		mainExample = instance;
		console.log("MainExample deployed");

		return SideBridge.new();
	}).then(function(instance) {
		sideBridge = instance;
		console.log("SideBridge deployed");

		return SideExample.new();
	}).then(function(instance) {
		sideExample = instance;
		console.log("SideExample deployed");

		return mainExample.something.estimateGas(sideExample.address, 2, 32);
	}).then(function(gas) {
		console.log("estimated gas for MainExample.something =", gas);

		return mainExample.something(sideExample.address, 2, 32, {
			from: user,
		})
	}).then(function(result) {
		console.log("something called. result =", result);

		return Promisify(cb => mainBridge.Send().get(cb));
	}).then(function(events) {
		console.log("events =", events);
		assert.equal(1, events.length);
		assert.equal("Send", events[0].event)
		assert.equal(mainExample.address, events[0].args.sender);
		assert.equal(sideExample.address, events[0].args.receiver);

		return sideBridge.receive(
		  events[0].args.sender,
		  events[0].args.receiver,
		  events[0].args.data
		);
	}).then(function(result) {
		console.log("receive called. logs =", result.logs);
		assert.equal(1, result.logs.length)
		assert.equal("Receive", result.logs[0].event)
		assert.equal(mainExample.address, result.logs[0].args.sender);
		assert.equal(sideExample.address, result.logs[0].args.receiver);
		assert.equal(true, result.logs[0].args.success);

		return Promisify(cb => sideExample.Times().get(cb));
	}).then(function(events) {
		console.log("events =", events);
		assert.equal(1, events.length);
		assert.equal("Times", events[0].event)
		assert.equal(2, events[0].args.a);
		assert.equal(32, events[0].args.b);
		assert.equal(64, events[0].args.result);
	})
  })
})
