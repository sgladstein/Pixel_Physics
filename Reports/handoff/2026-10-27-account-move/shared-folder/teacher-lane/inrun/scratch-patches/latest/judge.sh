# usage: judge.sh NAME GENOME   -- the standard checks for a founder genome (teacher lane, 2026-10-21)
#  1 hunger probe: 8 colonies of clones, banks set to 1.5 tanks at 20k; departures + starvations in the 6k after, vs untouched
#  2 clone colonies: 30 requested clones, selection off, 200k, seeds 11-14: survivors, deliveries
#  3 in-run selection: founder + 30 mutants, 3 x 200k rounds (hall, tank price, hungry pay 10), seeds 1-8: colonies alive at end
cd /tmp/claude-0/-home-user-Pixel-Physics/8a2e7e11-3afa-57e4-8c63-69bb9c615c96/scratchpad/inrun
N=$1; G=$2; O=judge/$N; mkdir -p $O
B=/home/user/tp/target/release/examples/inrun
P="frames=26000 dist=25 spread=10 grants=3 window=2000 sel=off cap=60 drip=20 w_hungry=0.0001 hungry=100 hungry_gap=1"
X="min_age=8000 cooldown=3000 w_deliver=7 w_field=0.05 w_depth=2 rim=15 cap=60 turn=30000 fresh_kid=1 drip=20 mutation=neutral kevents=10 w_home_eat=2 kid_cap=4 retire=1 hall=30000 fprice=tank w_hungry=10 progress=1"
{
for s in 1 2 3 4 5 6 7 8; do
  echo "$B founders=$G:30 $P seed=$s starve_at=20000:1.5 out=$O/st_$s"
  echo "$B founders=$G:30 $P seed=$s out=$O/ctl_$s"
done
for s in 11 12 13 14; do echo "$B founders=$G:30 frames=200000 dist=25 spread=10 grants=3 window=4000 sel=off cap=60 drip=20 seed=$s out=$O/clone_$s"; done
for s in 1 2 3 4 5 6 7 8; do echo "bash rounds2.sh $O/sel_$s $s 3 200000 score $G $X"; done
} | xargs -P 4 -I{} sh -c '{} > /dev/null 2>&1'
python3 -I judgesum.py $O | tee $O/SUMMARY.txt
