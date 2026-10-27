#!/bin/bash
# usage: garden.sh NAME COLONY [extra args]; SETX extra env; BINDIR default brood-wt
cd ${BINDIR:-/home/user/brood-wt}
N=$1; C=$2; shift 2
D=/mnt/project-files/brood/carry/cs20light_c$C/pool
O=/tmp/claude-0/-home-user-Pixel-Physics/c7f51f81-78a1-5e63-8d5d-37e2280ac4f0/scratchpad/pilot1/g/$N
mkdir -p $O
F=$(python3 -c "
import csv,sys
r=list(csv.DictReader(open('$D/index.csv'))); r.sort(key=lambda x:int(x['id'])); r.sort(key=lambda x:(-float(x['score']),int(x['id']))); r=r[:30]; n=len(r)
print(','.join(f\"$D/{x['id']}.raw.txt@{x['family']}.{x['id']}:{30//n+(1 if i<30%n else 0)}\" for i,x in enumerate(r)))")
SE="setenv=PP_HOMEVEC=40:30,PP_DIGHEAD=scratch/inrun/genomes/dh7.head,PP_DIGHEAD_GATE=1,PP_DIGHEAD_EVOLVE=1,PP_DIGLIGHT=1${SETX:+,$SETX}"
SECONDS=0
target/release/examples/inrun founders=$F seed=${SEED:-$((C*1000+99))} frames=200000 dist=${DIST:-25} spread=10 grants=3 window=4000 sel=off min_age=8000 cooldown=3000 w_deliver=7 w_field=0.05 w_depth=2 rim=15 cap=60 fresh_kid=1 drip=20 mutation=neutral kevents=10 w_home_eat=2 kid_cap=4 retire=1 hall=30000 fprice=tank w_hungry=10 progress=1 turn=30000 kid_grants=3 lesson=scratch/inrun/genomes/c1.raw.txt:0.1 "$SE" cut_join=32 "$@" out=$O > $O.log 2>&1
echo "wall $SECONDS s" >> $O.log
