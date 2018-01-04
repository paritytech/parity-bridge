#!/usr/bin/env bash

# prints out estimated gas costs of contract functions.
# runs the tests which estimate and print out gas costs and greps by gas costs.

cd truffle
yarn test | grep "estimated gas cost"
