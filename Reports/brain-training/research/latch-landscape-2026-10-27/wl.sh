#!/bin/bash
# wl.sh ARM SEED : the Teacher lane's 16-world setting (colony-selection log 2026-10-26/27): teacher off, sel off, far
# moving food, founding grant; prints starved / food home / alive / lost
tag=$1; s=$2; a=${tag%@*}; X=""; [ "$tag" != "$a" ] && X="PIXEL_PHYSICS_LINK_PRICE=${tag#*@p}"; R=${R:-$PWD}  # a work dir holding arms/ (mkarms.py) and runs/
F=$(ls $R/arms/$a/*.raw.txt | sed 's/$/:1/' | paste -sd,); D=$R/runs/$tag-$s; rm -rf $D; mkdir -p $D
env $X PP_LASTFOOD=off PP_BED_W=448 PP_MIND_DOOR_OPEN=drop RAYON_NUM_THREADS=1 $(git rev-parse --show-toplevel)/target/release/examples/inrun \
  founders=$F seed=$s frames=200000 dist=25 grants=3 food=mix patches=20000:40:120 sel=off out=$D/run > $D/run.log 2>&1
python3 -I - $D $tag $s <<'P'
import sys,csv; d,a,s=sys.argv[1:4]; st=fh=0
for e in csv.DictReader(open(d+'/run/events.csv')):
    if e['kind']=='death' and e['v']=='1': st+=1
    elif e['kind']=='deliver' and abs(int(e['x'])-128)<=8: fh+=1
fr=[l for l in open(d+'/run.log') if l.startswith('frame')]
al=fr[-1].split()[3] if fr else 'NA'
print(f"{a} {s} starved {st} foodhome {fh} alive {al}", flush=True)
P
rm -f $D/run/events.csv $D/run/map_f* $D/run/ants_f*
