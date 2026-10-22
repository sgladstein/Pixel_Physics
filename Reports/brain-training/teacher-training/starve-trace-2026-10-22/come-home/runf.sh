# usage: runf.sh ARM SEED GENOME "ENV..." -- the foraging-loop tree (9d19121c + shadow-rim patch + PP_HOMEFAR), runl.sh settings
cd /home/user/fl
S=/tmp/claude-0/-home-user-Pixel-Physics/27939a3f-efe9-5f8e-aa38-c8f353d2e900/scratchpad/st
A=$1; SD=$2; G=$3; E="$4"
CTL="min_age=8000 cooldown=3000 w_deliver=7 w_field=0.05 w_depth=2 rim=15 cap=60 fresh_kid=1 drip=20 mutation=neutral kevents=10 w_home_eat=2 kid_cap=4 retire=1 hall=30000 fprice=tank w_hungry=10 progress=1 turn=30000"
O=$S/f/$A$SD; rm -rf $O; mkdir -p $S/f
env RAYON_NUM_THREADS=1 PP_PACK=1 $E target/release/examples/inrun founders=$G:1 mutants=30:0.03:1.0:7 seed=$SD frames=200000 dist=25 spread=10 grants=3 window=4000 sel=off $CTL trace=50 out=$O > $O.log 2>&1
rm -f $O/map_* $O/ants_f*
