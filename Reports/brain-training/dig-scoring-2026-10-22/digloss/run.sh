arm=$1; s=$2; S=/tmp/claude-0/-home-user-Pixel-Physics/d8079a03-3861-5d88-a8a6-9ab879927dc0/scratchpad
A="min_age=8000 cooldown=3000 w_deliver=7 w_field=0.05 w_depth=2 rim=15 cap=60 fresh_kid=1 drip=20 mutation=neutral kevents=10 w_home_eat=2 kid_cap=4 retire=1 hall=30000 fprice=tank w_hungry=10 progress=1 turn=30000 kid_grants=3"
cd /home/user/shortr
case $arm in plain) E="";; veto) E="PP_RIMVETO=3";; room) E=""; A="$A w_room=2";; esac
env $E target/release/examples/inrun founders=scratch/inrun/genomes/hA1.raw.txt:1 mutants=30:0.03:1.0:7 seed=$s frames=200000 dist=25 spread=10 grants=3 window=4000 sel=score $A out=$S/tm/${arm}_$s > /dev/null 2>&1
echo $arm $s done
