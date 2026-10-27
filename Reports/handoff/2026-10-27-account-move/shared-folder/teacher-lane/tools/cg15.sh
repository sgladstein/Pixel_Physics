# garden of a colsel15 colony's round-47 pool, fresh world c*1000+99, keeping the end map: ARM C [ENV]
S=/tmp/claude-0/-home-user-Pixel-Physics/8a2e7e11-3afa-57e4-8c63-69bb9c615c96/scratchpad
a=$1; c=$2; env=$3; d=$S/colsel${SET:-15}/$a/c$c/r47/final; out=$S/cg15/${a}_$c
A="min_age=8000 cooldown=3000 w_deliver=7 w_field=0.05 w_depth=2 rim=15 cap=60 fresh_kid=1 drip=20 mutation=neutral kevents=10 w_home_eat=2 kid_cap=4 retire=1 hall=30000 fprice=tank w_hungry=10 progress=1 turn=30000 kid_grants=3 cut_join=32"
B=/home/user/cloud/target/release/examples/inrun
list=$(python3 -I -c "
import csv
r=list(csv.DictReader(open('$d/index.csv'))); r.sort(key=lambda x:int(x['id'])); r.sort(key=lambda x:(-float(x['score']), int(x['id']))); r=r[:30]; n=len(r)
print(','.join(f\"$d/{x['id']}.raw.txt@{x['family']}.{x['id']}:{30//n + (1 if i < 30%n else 0)}\" for i,x in enumerate(r)) if n else '')")
[ -n "$list" ] || { echo "$a $c EXTINCT"; exit 0; }
mkdir -p $S/cg15; $B founders=$list seed=$((c*1000+99)) frames=200000 dist=25 spread=10 grants=3 window=4000 sel=off $A setenv=$env out=$out > /dev/null 2>&1
rm -f $out/map_f* $out/ants_f*; gzip -f $out/events.csv
echo "$a $c done"
