#!/bin/bash
# world.sh NAME POOLDIR SEED [env...] : teacher OFF, far moving food, 200k frames; prints starved / food home / alive
# Run from a scratch work dir holding c13/ (split.py of assets/minds/std_pool.txt, feature 18 zeroed), runs/, fits/.
n=$1; d=$2; s=$3; shift 3
F=$(ls $d/*.raw.txt | sed 's/$/:1/' | paste -sd,); D=wruns/$n-$s; rm -rf $D; mkdir -p $D
env PP_LASTFOOD=off PP_BED_W=448 RAYON_NUM_THREADS=1 "$@" $(git rev-parse --show-toplevel)/target/release/examples/inrun founders=$F seed=$s frames=200000 patches=20000:40:120 sel=off out=$D/run > $D/run.log 2>&1
python3 -I - $D <<'P'
import sys,csv; d=sys.argv[1]; st=fh=0
for e in csv.DictReader(open(d+'/run/events.csv')):
    if e['kind']=='death' and e['v']=='1': st+=1
    elif e['kind']=='deliver' and abs(int(e['x'])-128)<=8: fh+=1
al=[l for l in open(d+'/run.log') if l.startswith('frame')][-1].split()[3]
print(f"{d} starved {st} foodhome {fh} alive {al}")
P
rm -f $D/run/events.csv $D/run/map_f*
