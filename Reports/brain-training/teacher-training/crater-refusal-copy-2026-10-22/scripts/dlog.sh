# usage: rlog.sh GENOME SEED -- the genome acts; the crater refusal (PP_RIMVETO=3, label only) labels its states
cd /home/user/cloud
S=/tmp/claude-0/-home-user-Pixel-Physics/759d2b83-5c44-53ec-a7d9-b404cb2c99a4/scratchpad/cr
N=$(basename $1 .raw.txt)
CTL="min_age=8000 cooldown=3000 w_deliver=7 w_field=0.05 w_depth=2 rim=15 cap=60 fresh_kid=1 drip=20 mutation=neutral kevents=10 w_home_eat=2 kid_cap=4 retire=1 hall=30000 fprice=tank w_hungry=10 progress=1 turn=30000"
env RAYON_NUM_THREADS=1 PP_RIMVETO=3 PP_SPOILVETO=5 PP_SPOILDROP=15 PP_RIMVETO_LABEL=1 PP_COPY_LOG=$S/logs/d$N-$2.log PP_COPY_SAMPLE=30:1 target/release/examples/inrun founders=$1:1 mutants=30:0.03:1.0:7 seed=$2 frames=200000 dist=25 spread=10 grants=3 window=4000 sel=off $CTL out=$S/logs/d$N-$2 > $S/logs/d$N-$2.out 2>&1
