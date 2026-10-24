# usage: dlog.sh GENOME SEED -- the genome acts (hcs build, PP_HOMEVEC=40:30 PP_BODYRULES=both); the old ant labels its empty dig states
S=/tmp/claude-0/-home-user-Pixel-Physics/d8079a03-3861-5d88-a8a6-9ab879927dc0/scratchpad/ol/h
N=$(basename $1 .raw.txt)
A="min_age=8000 cooldown=3000 w_deliver=7 w_field=0.05 w_depth=2 rim=15 cap=60 fresh_kid=1 drip=20 mutation=neutral kevents=10 w_home_eat=2 kid_cap=4 retire=1 hall=30000 fprice=tank w_hungry=10 progress=1 turn=30000 kid_grants=3"
cd /home/user/e5c
env RAYON_NUM_THREADS=1 PP_HOMEVEC=40:30 PP_BODYRULES=both PP_DIGFEAT=1 PP_OLDLAB=1 PP_OLDLAB_EMPTY=1 $ENVX PP_OLDLAB_K=1 PP_COPY_SAMPLE=50:1 PP_COPY_LOG=$S/logs5/${TAG:-$N}-$2.log /home/user/e5ct/release/examples/inrun founders=$1:1 mutants=30:0.03:1.0:7 seed=$2 frames=200000 dist=25 spread=10 grants=3 window=4000 sel=off $A out=$S/logs5/${TAG:-$N}-$2 > $S/logs5/${TAG:-$N}-$2.out 2>&1
rm -f $S/logs5/${TAG:-$N}-$2/map_f* $S/logs5/${TAG:-$N}-$2/ants_f*
