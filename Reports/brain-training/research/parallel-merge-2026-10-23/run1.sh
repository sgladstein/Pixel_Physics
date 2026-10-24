# usage: run1.sh ARM GENOME SEED [ENV...] -- one bed colony, 200k frames, sel=off (crater-refusal recipe)
cd /home/user/cw
S=/tmp/claude-0/merge/runs
A=$1; G=$2; SD=$3; shift 3
CTL="min_age=8000 cooldown=3000 w_deliver=7 w_field=0.05 w_depth=2 rim=15 cap=60 fresh_kid=1 drip=20 mutation=neutral kevents=10 w_home_eat=2 kid_cap=4 retire=1 hall=30000 fprice=tank w_hungry=10 progress=1 turn=30000"
rm -rf $S/$A/$SD; mkdir -p $S/$A
env RAYON_NUM_THREADS=1 "$@" target/release/examples/inrun founders=$G:1 mutants=30:0.03:1.0:7 seed=$SD frames=200000 dist=25 spread=10 grants=3 window=4000 sel=off $CTL out=$S/$A/$SD > $S/$A/$SD.log 2>&1
rm -f $S/$A/$SD/ants_f*
