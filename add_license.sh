#!/bin/sh

for f in $(find . -name '*.rs'); do
 	cat license_header.txt $f > $f.new
	mv $f.new $f
done
