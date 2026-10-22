#!/bin/bash
# Common garden for the fix test: each job's LAST round's final ants in a fresh fixed world (seed*1000+99), 200k frames,
# selection off, plus base (unselected hA1 mutants) and the old ant. Line: NAME home8 starved cuts20 field
set -u
W=/home/user/review-fix
S=/tmp/claude-0/-home-user-Pixel-Physics/44bcf220-1a62-5c29-9989-f15bd0e562ae/scratchpad/fixes
BIN=$W/target/release/examples/inrun
G=$W/scratch/inrun/genomes/hA1.raw.txt
C="frames=200000 dist=25 spread=10 grants=3 window=4000 sel=off min_age=8000 cooldown=3000 w_deliver=7 w_field=0.05 w_depth=2 rim=15 cap=60 drip=20 kevents=10 w_home_eat=2 fprice=tank progress=1"
one() {
  name=$1; s=$2; shift 2
  out=$S/cg/$name; mkdir -p $out
  $BIN "$@" seed=$((s*1000+99)) $C out=$out > $out.log 2>&1
  ev=$out/events.csv
  echo "$name $(awk -F, '$3=="deliver"&&($4-128)^2<=64' $ev | wc -l) $(awk -F, '$3=="death"&&$6=="1"' $ev | wc -l) $(awk -F, '$3=="dig"&&($4-128)^2<=400' $ev | wc -l) $(awk -F, '$3=="field"' $ev | wc -l)" >> $S/cg/results.txt
  rm -f $out/events.csv $out/map_f* $out/ants_f*
}
export -f one; export BIN S C G
: > $S/cg/list.txt; : > $S/cg/results.txt
for s in $(seq 1 12); do
  for arm in ctl band cross lex prot; do
    j=$S/jobs/${arm}_$s; [ $arm = ctl ] && j=$S/../mutload/jobs/scoreM_$s
    last=$(ls -d $j/r* 2>/dev/null | sed 's/.*r//' | sort -n | tail -1)
    if [ -f $j/EXTINCT ] || [ -z "$last" ] || [ ! -f $j/r$last/final/index.csv ]; then echo "${arm}_$s EXTINCT" >> $S/cg/results.txt; continue; fi
    list=$(python3 -I -c "
import csv
r=list(csv.DictReader(open('$j/r$last/final/index.csv'))); r.sort(key=lambda x:int(x['id'])); r=r[:30]; n=len(r)
print(','.join(f\"$j/r$last/final/{x['id']}.raw.txt@{x['family']}.{x['id']}:{30//n + (1 if i < 30%n else 0)}\" for i,x in enumerate(r)) if n else '')")
    [ -n "$list" ] || { echo "${arm}_$s EXTINCT" >> $S/cg/results.txt; continue; }
    echo "${arm}_$s $s founders=$list" >> $S/cg/list.txt
  done
  echo "base_$s $s founders=$G:1 mutants=30:0.03:1.0:7" >> $S/cg/list.txt
  echo "old_$s $s founders=$G:30 nomind=1" >> $S/cg/list.txt
done
cd $W
xargs -P 4 -L 1 bash -c 'one "$@"' _ < $S/cg/list.txt
echo "CG DONE $(date -u +%T)" >> $S/progress.txt
