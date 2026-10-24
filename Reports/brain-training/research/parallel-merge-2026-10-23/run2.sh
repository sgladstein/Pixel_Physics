# usage: run2.sh ARM SEED -- a garden of the arm's 30 founders (e2.arms), 200k frames, sel=off, colsel20 env, rules off
cd /home/user/cw
S=/tmp/claude-0/merge/runs2; A=$1; SD=$2
L=$(grep "^$A " /tmp/claude-0/merge/e2.arms | cut -d' ' -f2)
X="min_age=8000 cooldown=3000 w_deliver=7 w_field=0.05 w_depth=2 rim=15 cap=60 fresh_kid=1 drip=20 mutation=neutral kevents=10 w_home_eat=2 kid_cap=4 retire=1 hall=30000 fprice=tank w_hungry=10 progress=1 turn=30000 kid_grants=3 cut_join=32"
mkdir -p $S/$A; rm -rf $S/$A/$SD
env RAYON_NUM_THREADS=1 target/release/examples/inrun founders=$L seed=$SD frames=200000 dist=25 spread=10 grants=3 window=4000 sel=off $X setenv=PP_HOMEVEC=40:30,PP_DIGHEAD=scratch/inrun/genomes/dh7.head,PP_DIGHEAD_GATE=1,PP_DIGHEAD_EVOLVE=1,PP_DIGLIGHT=1 out=$S/$A/$SD > $S/$A/$SD.log 2>&1
rm -f $S/$A/$SD/ants_f*
