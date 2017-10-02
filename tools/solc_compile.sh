#!/bin/bash

cd contracts
solc --abi --bin -o . --overwrite bridge.sol

for abi in *.abi; do
	python -m json.tool "$abi" > tmp
	cat tmp > "$abi"
done

rm tmp
