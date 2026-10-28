#!/bin/bash
# SCRATCH (nest lane, 2026-10-28): scratch/inrun/rounds.sh started from a whole pool instead of 30 mutants of one
# genome. Round 0 founds on POOLDIR/*.raw.txt (baseline.py's pool, one ant each); each later round founds a fresh world
# (seed*1000+r) on the last round's 30 best-scored living ants, as rounds.sh does. Exam-bed settings (exam.py).
# usage: poolrounds.sh OUT SEED ROUNDS FRAMES POOLDIR [extra inrun args...]
O=$1; S=$2; R=$3; F=$4; P=$5; shift 5
B=${BIN:-target/release/examples/inrun}
C="frames=$F dist=25 spread=10 grants=3 window=4000 sel=score food=mix lesson=scratch/inrun/genomes/c1.raw.txt:0.1 patches=20000:40:120 setenv=PP_BED_W=448,PP_MIND_DOOR_OPEN=drop $*"
mkdir -p $O
G=$(for p in $P/*.raw.txt; do n=$(basename $p .raw.txt); printf '%s@%s:1,' $p $n; done); G=${G%,}
[ -f $O/r0/.done ] || { RAYON_NUM_THREADS=1 $B founders=$G seed=$S $C out=$O/r0 > $O/r0.log 2>&1 && touch $O/r0/.done; }
for r in $(seq 1 $((R-1))); do
  p=$O/r$((r-1))
  [ -f $O/r$r/.done ] && continue
  list=$(python3 -I -c "
import csv
r=list(csv.DictReader(open('$p/final/index.csv')))
r.sort(key=lambda x:int(x['id']))
r.sort(key=lambda x:(-float(x['score']), int(x['id'])))
r=r[:30]; n=len(r)
print(','.join(f\"$p/final/{x['id']}.raw.txt@{x['family']}.{x['id']}:{30//n + (1 if i < 30%n else 0)}\" for i,x in enumerate(r)) if n else '')")
  [ -n "$list" ] || { echo "extinct after round $((r-1))" > $O/EXTINCT; exit 0; }
  RAYON_NUM_THREADS=1 $B founders=$list seed=$((S*1000+r)) $C out=$O/r$r > $O/r$r.log 2>&1 && touch $O/r$r/.done
  rm -f $p/map_f0* $p/ants_f0*
done
