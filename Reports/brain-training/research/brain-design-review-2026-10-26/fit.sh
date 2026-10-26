#!/bin/bash
# fit.sh NAME [homefit args...] : one offline copy of the come-home teacher onto colony 13 member 0
# Run from a scratch work dir holding c13/ (split.py of assets/minds/std_pool.txt, feature 18 zeroed), runs/, fits/.
n=$1; shift
L=runs/w32007/copy.log,runs/w33007/copy.log
$(git rev-parse --show-toplevel)/target/release/examples/homefit logs=$L start=c13/m00.raw.txt out=fits/$n.raw.txt "$@" > fits/$n.log 2>&1; echo "elapsed $SECONDS s" >> fits/$n.log
