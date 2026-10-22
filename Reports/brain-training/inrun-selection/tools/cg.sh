# common garden: ARM SEED -> 200k frames, sel off, fresh world seed*1000+99, top 30 of the arm's last round
S=/tmp/claude-0/-home-user-Pixel-Physics/8a2e7e11-3afa-57e4-8c63-69bb9c615c96/scratchpad
a=$1; s=$2; d=$S/cg/fin/${a}_$s; out=$S/cg/run/${a}_$s
A="min_age=8000 cooldown=3000 w_deliver=7 w_field=0.05 w_depth=2 rim=15 cap=60 fresh_kid=1 drip=20 mutation=neutral kevents=10 w_home_eat=2 kid_cap=4 retire=1 hall=30000 fprice=tank w_hungry=10 progress=1 turn=30000 kid_grants=3"
C="frames=200000 dist=25 spread=10 grants=3 window=4000 sel=off $A"
B=/home/user/cloud/target/release/examples/inrun
if [ $a = base ]; then list=/home/user/cloud/scratch/inrun/genomes/hA1.raw.txt:1; extra="mutants=30:0.03:1.0:7"; elif [ $a = old ]; then list=/home/user/cloud/scratch/inrun/genomes/hA1.raw.txt:1; extra="mutants=30:0.03:1.0:7 nomind=1"; else
list=$(python3 -I -c "
import csv
r=list(csv.DictReader(open('$d/index.csv'))); r.sort(key=lambda x:int(x['id'])); r.sort(key=lambda x:(-float(x['score']), int(x['id']))); r=r[:30]; n=len(r)
print(','.join(f\"$d/{x['id']}.raw.txt@{x['family']}.{x['id']}:{30//n + (1 if i < 30%n else 0)}\" for i,x in enumerate(r)) if n else '')" 2>/dev/null); extra=""; fi
[ -n "$list" ] || { echo "$a $s EXTINCT"; exit 0; }
mkdir -p $S/cg/run; $B founders=$list $extra seed=$((s*1000+99)) $C out=$out > /dev/null 2>&1
python3 -I - $out <<'P'
import csv,sys,collections
o=sys.argv[1]; c=collections.Counter()
for e in csv.DictReader(open(o+'/events.csv')):
    k=e['kind']
    if k=='deliver': c['home8']+=abs(int(e['x'])-128)<=8
    elif k=='death': c['starved']+=e['v']=='1'
    elif k=='dig' and (int(e['x'])-128)**2<=400: c['cuts']+=1
    elif k=='field': c['field']+=1
print(o.rsplit('/',1)[1], c['home8'], c['starved'], c['cuts'], c['field'])
P
rm -f $out/map_f* $out/ants_f*
