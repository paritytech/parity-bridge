#!/usr/bin/env bash

solc \
  --abi \
  --bin \
  --optimize \
  --output-dir compiled_contracts \
  --overwrite \
  contracts/bridge.sol
