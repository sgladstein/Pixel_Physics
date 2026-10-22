# SCRATCH (teacher lane, never for main). Portable copy of the lane's rounds2.sh, run from the repo root.
# usage: rounds.sh OUT SEED ROUNDS FRAMES SEL GENOME [extra inrun args...]
# Round 0 founds on 30 mutants of GENOME; each later round founds a fresh world (seed*1000+r) on the last round's living
# ants (top 30 by score; random arm: 30 at random), topped up to 30 requests by repeating them. Scores and ages restart.
O=$1; S=$2; R=$3; F=$4; SEL=$5; G=$6; shift 6
B=${BIN:-target/release/examples/inrun}
C="frames=$F dist=25 spread=10 grants=3 window=4000 sel=$SEL $*"
mkdir -p $O
$B founders=$G:1 mutants=30:0.03:1.0:7 seed=$S $C out=$O/r0 > /dev/null 2>&1
for r in $(seq 1 $((R-1))); do
  p=$O/r$((r-1))
  list=$(python3 -I -c "
import csv, random
r=list(csv.DictReader(open('$p/final/index.csv')))
r.sort(key=lambda x:int(x['id']))  # index.csv is written in HashMap order: sort first, or ties (newborns at 0) reorder
if '$SEL' == 'random': random.Random($S*7+$r).shuffle(r)
else: r.sort(key=lambda x:(-float(x['score']), int(x['id'])))
r=r[:30]; n=len(r)
print(','.join(f\"$p/final/{x['id']}.raw.txt@{x['family']}.{x['id']}:{30//n + (1 if i < 30%n else 0)}\" for i,x in enumerate(r)) if n else '')")
  [ -n "$list" ] || { echo "extinct after round $((r-1))" > $O/EXTINCT; exit 0; }
  $B founders=$list seed=$((S*1000+r)) $C out=$O/r$r > /dev/null 2>&1
  # long runs (more than 10 rounds): drop the every-4k maps and ant lists of the round before (the small pull never
  # fetched them; the round-end ones stay) so 60+ rounds fit the VM's disk
  # ...and gzip its event log (about 5x): 64 jobs x 60 rounds of plain events.csv filled a 60 GB disk on 2026-10-22,
  # and every job started after that crashed at once and read as extinct. evreduce.py reads events.csv.gz too.
  [ $R -gt 10 ] && rm -f $p/map_f0[01]* $p/ants_f0[01]* && gzip -f $p/events.csv
done
