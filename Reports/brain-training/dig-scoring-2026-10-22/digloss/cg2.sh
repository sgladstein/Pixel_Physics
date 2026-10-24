# common garden for a rounds.sh round dir: its final genomes, selection off, world $2
S=/tmp/claude-0/-home-user-Pixel-Physics/d8079a03-3861-5d88-a8a6-9ab879927dc0/scratchpad
rd=$1; ws=$2; out=$rd/cg_w$ws
list=$(python3 -I -c "
import csv
r=list(csv.DictReader(open('$rd/final/index.csv'))); r.sort(key=lambda x:int(x['id'])); r=r[:30]; n=len(r)
print(','.join(f\"$rd/final/{x['id']}.raw.txt@{x['family']}.{x['id']}:{30//n + (1 if i < 30%n else 0)}\" for i,x in enumerate(r)) if n else '')")
[ -n "$list" ] || { echo "$rd w$ws EXTINCT"; exit 0; }
cd /home/user/nestzone
target2/release/examples/inrun founders=$list seed=$ws frames=60000 dist=25 spread=10 grants=3 window=4000 sel=off min_age=8000 cooldown=3000 w_deliver=7 w_field=0.05 w_depth=2 rim=15 cap=60 drip=20 kevents=10 w_home_eat=2 fprice=tank progress=1 out=$out > $out.log 2>&1
echo "$rd w$ws $(awk -F, '$3=="dig"&&($4-128)^2<=400' $out/events.csv | wc -l) $(awk -F, '$3=="deliver"' $out/events.csv | wc -l)"
