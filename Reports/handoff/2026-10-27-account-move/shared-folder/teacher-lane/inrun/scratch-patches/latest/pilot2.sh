set -e
cd /tmp/claude-0/-home-user-Pixel-Physics/8a2e7e11-3afa-57e4-8c63-69bb9c615c96/scratchpad/inrun
B=/home/user/tp/target/release/examples/inrun
mkdir -p pilot2 fresh2
for s in 1 2 3 4 5 6 7 8; do echo "$B founders=v2.txt:1 mutants=30:0.03:1.0:7 frames=200000 seed=$s dist=25 spread=10 cap=200 grants=3 window=2000 sel=score w_deliver=7 w_field=0.05 out=pilot2/score_$s"; done > pilot2/jobs.txt
cat pilot2/jobs.txt | xargs -P 4 -I{} sh -c '{} > /dev/null 2>&1'
C="frames=200000 dist=25 spread=10 cap=200 grants=3 window=2000 maps=0"
: > fresh2/jobs.txt
for s in 1 2 3 4 5 6 7 8; do
  list=$(python3 -I -c "
import csv
r=list(csv.DictReader(open('pilot2/score_$s/final/index.csv')))
r.sort(key=lambda x:-float(x['score'])); r=r[:30]; n=len(r)
print(','.join(f\"pilot2/score_$s/final/{x['id']}.raw.txt:{30//n + (1 if i < 30%n else 0)}\" for i,x in enumerate(r)))")
  for t in 11 12 13 14; do echo "$B founders=$list seed=$t $C out=fresh2/score$s"_"$t" >> fresh2/jobs.txt; done
done
cat fresh2/jobs.txt | xargs -P 4 -I{} sh -c '{} > /dev/null 2>&1'
echo done > fresh2/ALLDONE
