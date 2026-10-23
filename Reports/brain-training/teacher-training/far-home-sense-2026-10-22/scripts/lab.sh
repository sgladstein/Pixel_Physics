# usage: lab.sh TAG GENOME SEED "ENV" -- the genome acts with PP_HOMEVEC on; the teachers only label (copy log)
T=$1; G=$2; SD=$3; E="$4"
cd /home/user/hv
S=/home/user/hvs
CTL="min_age=8000 cooldown=3000 w_deliver=7 w_field=0.05 w_depth=2 rim=15 cap=60 fresh_kid=1 drip=20 mutation=neutral kevents=10 w_home_eat=2 kid_cap=4 retire=1 hall=30000 fprice=tank w_hungry=10 progress=1 turn=30000"
rm -rf $S/logs/$T-$SD
env RAYON_NUM_THREADS=1 PP_COPY_LOG=$S/logs/$T-$SD.log PP_COPY_SAMPLE=30:1 $E target/release/examples/inrun founders=$G:1 mutants=30:0.03:1.0:7 seed=$SD frames=200000 dist=25 spread=10 grants=3 window=4000 sel=off $CTL out=$S/logs/$T-$SD > $S/logs/$T-$SD.out 2>&1
rm -rf $S/logs/$T-$SD
