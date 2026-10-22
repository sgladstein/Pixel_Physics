# usage: runsel.sh K  -- round-6 ants of shortr-1022 colony hA1r200_K (top 30 by score, as rounds.sh), fresh world seed K, sel off
cd /home/user/cloud
S=/tmp/claude-0/-home-user-Pixel-Physics/27939a3f-efe9-5f8e-aa38-c8f353d2e900/scratchpad/st
K=$1; p=$S/sel/hA1r200_$K/r5
list=$(python3 -I -c "
import csv
r=list(csv.DictReader(open('$p/final/index.csv')))
r.sort(key=lambda x:int(x['id'])); r.sort(key=lambda x:(-float(x['score']), int(x['id'])))
r=r[:30]; n=len(r)
print(','.join(f\"$p/final/{x['id']}.raw.txt@{x['family']}.{x['id']}:{30//n + (1 if i < 30%n else 0)}\" for i,x in enumerate(r)))")
CTL="min_age=8000 cooldown=3000 w_deliver=7 w_field=0.05 w_depth=2 rim=15 cap=60 fresh_kid=1 drip=20 mutation=neutral kevents=10 w_home_eat=2 kid_cap=4 retire=1 hall=30000 fprice=tank w_hungry=10 progress=1 turn=30000"
O=$S/l/sel$K; rm -rf $O; mkdir -p $O
env RAYON_NUM_THREADS=1 PP_DLOG=$O/dlog.csv target/release/examples/inrun founders=$list seed=$K frames=200000 dist=25 spread=10 grants=3 window=4000 sel=off $CTL trace=50 out=$O > $O.log 2>&1
rm -f $O/map_* $O/ants_f*
