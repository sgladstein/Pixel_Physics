#!/bin/bash
name=$1; seed=$2; shift 2
d=$(echo "$@" | grep -o 'out=[^ ]*' | cut -d= -f2)
[ -f $d/done ] && exit 0
mkdir -p $d
cd /home/user/Pixel_Physics; export RAYON_NUM_THREADS=1
t0=$(date +%s)
target/release/examples/inrun "$@" > $d/log.txt 2>&1 && echo $(( $(date +%s) - t0 )) > $d/time.txt && touch $d/done
