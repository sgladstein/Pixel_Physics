# usage: run.sh ARM GENOME SEED [ENV...] -- one bed colony, 200k frames, sel=off, events kept in l/ARMSEED
cd /home/user/cloud
S=/tmp/claude-0/-home-user-Pixel-Physics/759d2b83-5c44-53ec-a7d9-b404cb2c99a4/scratchpad/cr
A=$1; G=$2; SD=$3; shift 3
CTL="min_age=8000 cooldown=3000 w_deliver=7 w_field=0.05 w_depth=2 rim=15 cap=60 fresh_kid=1 drip=20 mutation=neutral kevents=10 w_home_eat=2 kid_cap=4 retire=1 hall=30000 fprice=tank w_hungry=10 progress=1 turn=30000"
rm -rf $S/l/$A$SD
env RAYON_NUM_THREADS=1 "$@" target/release/examples/inrun founders=$G:1 mutants=30:0.03:1.0:7 seed=$SD frames=200000 dist=25 spread=10 grants=3 window=4000 sel=off $CTL out=$S/l/$A$SD > $S/l/$A$SD.log 2>&1
rm -f $S/l/$A$SD/ants_f*
