# usage: fresh_for.sh PILOTDIR ARM OUTDIR  -- fresh-colony jobs (sel off, 200k, seeds 11-14) for each run's final population
P=$1; A=$2; O=$3; B=/home/user/tp/target/release/examples/inrun
C="frames=200000 dist=25 spread=10 cap=200 grants=3 window=2000 maps=0"
mkdir -p $O
for s in 1 2 3 4 5 6 7 8; do
  [ -s $P/${A}_$s/final/index.csv ] || continue
  list=$(python3 -I -c "
import csv
r=list(csv.DictReader(open('$P/${A}_$s/final/index.csv')))
r.sort(key=lambda x:-float(x['score'])); r=r[:30]; n=len(r)
print(','.join(f\"$P/${A}_$s/final/{x['id']}.raw.txt:{30//n + (1 if i < 30%n else 0)}\" for i,x in enumerate(r)) if n else '')")
  [ -n "$list" ] || continue
  for t in 11 12 13 14; do echo "$B founders=$list seed=$t $C out=$O/${A}${s}_$t"; done
done
