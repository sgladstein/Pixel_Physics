#!/bin/bash
# usage: run.sh GROUP SEEDIDX FRAMES OUTDIR [extra...]   GROUP = old | hcs | cs10:<colony>
S=/tmp/claude-0/-home-user-Pixel-Physics/8a2e7e11-3afa-57e4-8c63-69bb9c615c96/scratchpad
g=$1; k=$2; F=$3; out=$4; shift 4
A="min_age=8000 cooldown=3000 w_deliver=7 w_field=0.05 w_depth=2 rim=15 cap=60 fresh_kid=1 drip=20 mutation=neutral kevents=10 w_home_eat=2 kid_cap=4 retire=1 hall=30000 fprice=tank w_hungry=10 progress=1 turn=30000 kid_grants=3"
C="frames=$F dist=25 spread=10 grants=3 window=500 sel=off $A"
B=${BIN_INRUN:-/home/user/cloud/target/release/examples/inrun}
G=/home/user/cloud/scratch/inrun/genomes
case $g in
 old) list="$G/hA1.raw.txt:1"; extra="mutants=30:0.03:1.0:7 nomind=1";;
 hcs) list="$G/hcs.raw.txt:1"; extra="mutants=30:0.03:1.0:7 lesson=$G/c1.raw.txt:0.1 setenv=PP_HOMEVEC=40:30 cut_join=32";;
 cs10:*|cs11:*) c=${g#cs1?:}; arm=cs10std; [ ${g:0:4} = cs11 ] && arm=../colsel11/cs11r25; d=$S/colsel10/$arm/c$c/r47/final
   list=$(python3 -I -c "
import csv
r=list(csv.DictReader(open('$d/index.csv'))); r.sort(key=lambda x:int(x['id'])); r.sort(key=lambda x:(-float(x['score']), int(x['id']))); r=r[:30]; n=len(r)
print(','.join(f\"$d/{x['id']}.raw.txt@{x['family']}.{x['id']}:{30//n + (1 if i < 30%n else 0)}\" for i,x in enumerate(r)))")
   extra="lesson=$G/c1.raw.txt:0.1 setenv=PP_HOMEVEC=40:30 cut_join=32";;
esac
cd /home/user/cloud
$B founders=$list $extra seed=$((k*1000+99)) $C out=$out "$@" > $out.log 2>&1
