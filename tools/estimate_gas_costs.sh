#!/usr/bin/env bash

cd truffle
truffle test | grep "estimated gas cost" > ../res/gas_cost_estimates.txt
