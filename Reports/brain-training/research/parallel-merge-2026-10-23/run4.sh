# usage: run4.sh ARM SEED FOUNDERS -- garden, 200k frames, sel=off, PP_HOMEVEC, rules off
cd /home/user/cw
S=/tmp/claude-0/merge/runs4; A=$1; SD=$2; L=$3
X="min_age=8000 cooldown=3000 w_deliver=7 w_field=0.05 w_depth=2 rim=15 cap=60 fresh_kid=1 drip=20 mutation=neutral kevents=10 w_home_eat=2 kid_cap=4 retire=1 hall=30000 fprice=tank w_hungry=10 progress=1 turn=30000"
mkdir -p $S/$A; rm -rf $S/$A/$SD
env RAYON_NUM_THREADS=1 target/release/examples/inrun founders=$L seed=$SD frames=200000 dist=25 spread=10 grants=3 window=4000 sel=off $X setenv=PP_HOMEVEC=40:30 out=$S/$A/$SD > $S/$A/$SD.log 2>&1
rm -f $S/$A/$SD/ants_f*
