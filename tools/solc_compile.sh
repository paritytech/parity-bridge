#!/bin/bash

cd contracts
solc --abi --bin -o . --overwrite bridge.sol
