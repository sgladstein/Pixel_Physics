# common garden: GENOME_PATH SEED -> 200k frames, sel off, fresh world seed*1000+99, 24 ants from 30 mutants of the genome
S=/tmp/claude-0/-home-user-Pixel-Physics/d8079a03-3861-5d88-a8a6-9ab879927dc0/scratchpad/ol/cg
A="min_age=8000 cooldown=3000 w_deliver=7 w_field=0.05 w_depth=2 rim=15 cap=60 fresh_kid=1 drip=20 mutation=neutral kevents=10 w_home_eat=2 kid_cap=4 retire=1 hall=30000 fprice=tank w_hungry=10 progress=1 turn=30000 kid_grants=3"
N=$(basename $1 .raw.txt); out=$S/run/${N}_$2; mkdir -p $S/run
cd /home/user/colsel
env RAYON_NUM_THREADS=1 /home/user/shortr/target/release/examples/inrun founders=$1:1 mutants=30:0.03:1.0:7 seed=$(($2*1000+99)) frames=200000 dist=25 spread=10 grants=3 window=4000 sel=off $A out=$out > $out.log 2>&1
rm -f $out/map_f* $out/ants_f*
