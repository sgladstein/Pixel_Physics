# replay.sh POOLDIR SEED NAME TITLE [extra inrun args] -> $S/rp/NAME/{film run, NAME.mp4, NAME-data.json, last.png}
# POOLDIR = a colony's final/ (top 30 by score), or "old" for the old ant. Common-garden settings, film=50.
S=/tmp/claude-0/-home-user-Pixel-Physics/8a2e7e11-3afa-57e4-8c63-69bb9c615c96/scratchpad
d=$1; seed=$2; n=$3; title=$4; shift 4
A="min_age=8000 cooldown=3000 w_deliver=7 w_field=0.05 w_depth=2 rim=15 cap=60 fresh_kid=1 drip=20 mutation=neutral kevents=10 w_home_eat=2 kid_cap=4 retire=1 hall=30000 fprice=tank w_hungry=10 progress=1 turn=30000 kid_grants=3"
C="frames=200000 dist=25 spread=10 grants=3 window=4000 sel=off $A"
B=/home/user/cloud/target/release/examples/inrun; F=$S/rp/$n; rm -rf $F; mkdir -p $F/v
if [ "$d" = old ]; then f="founders=/home/user/cloud/scratch/inrun/genomes/hA1.raw.txt:1 mutants=30:0.03:1.0:7 nomind=1"; else
f="founders=$(python3 -I -c "
import csv
r=list(csv.DictReader(open('$d/index.csv'))); r.sort(key=lambda x:int(x['id'])); r.sort(key=lambda x:(-float(x['score']), int(x['id']))); r=r[:30]; n=len(r)
print(','.join(f\"$d/{x['id']}.raw.txt@{x['family']}.{x['id']}:{30//n + (1 if i < 30%n else 0)}\" for i,x in enumerate(r)))")"; fi
$B $f seed=$seed $C film=50 "$@" out=$F/run > $F/run.log 2>&1
python3 -I $S/film/mk.py $F/run $F/v "$title"
ffmpeg -loglevel error -y -framerate 30 -i $F/v/v%05d.png -c:v libx264 -pix_fmt yuv420p -crf 23 $F/$n.mp4
python3 -I $S/film/data.py $F/run $F/$n-data.json > /dev/null
cp $F/v/v03999.png $F/last.png; rm -rf $F/v
python3 -I - $F/run <<'P'
import sys; sys.path.insert(0,'/home/user/cloud/scratch/inrun'); import nestcount, roomshape, csv, collections
d=sys.argv[1]; n=nestcount.count(d,200000); c=collections.Counter()
for e in csv.DictReader(open(d+'/events.csv')):
    if e['kind']=='deliver': c['home']+=abs(int(e['x'])-128)<=8
    elif e['kind']=='death': c['starved']+=e['v']=='1'
print(f"food {c['home']} starved {c['starved']} nest {n['nest']} kept-cuts {n['nestcuts']} late {n['nestcuts_late']} crater {n['crater']} spoil {n['spoil']} shape {roomshape.score(d)}")
P
rm -rf $F/run/events.csv $F/run/map_f* $F/run/ants_f* $F/run/film
