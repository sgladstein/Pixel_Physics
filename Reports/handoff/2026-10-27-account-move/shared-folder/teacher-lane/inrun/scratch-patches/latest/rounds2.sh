# usage: rounds2.sh OUT SEED ROUNDS FRAMES SEL GENOME [extra inrun args...]
# Round 0 founds on the 30 mutants; each later round founds a fresh world (seed*1000+r) on the last round's living
# ants (top 30 by score; random arm: 30 at random), topped up to 30 requests by repeating them. Scores and ages restart.
cd /tmp/claude-0/-home-user-Pixel-Physics/8a2e7e11-3afa-57e4-8c63-69bb9c615c96/scratchpad/inrun
O=$1; S=$2; R=$3; F=$4; SEL=$5; G=$6; shift 6
B=/home/user/tp/target/release/examples/inrun
C="frames=$F dist=25 spread=10 grants=3 window=4000 sel=$SEL $*"
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
done
