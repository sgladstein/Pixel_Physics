cd /tmp/claude-0/-home-user-Pixel-Physics/8a2e7e11-3afa-57e4-8c63-69bb9c615c96/scratchpad/inrun
until [ -f p12/ALLDONE ]; do sleep 10; done
mkdir -p p13
X="min_age=8000 cooldown=3000 w_deliver=7 w_field=0.05 w_depth=2 rim=15 cap=60 turn=30000 fresh_kid=1 drip=20 mutation=neutral kevents=10 w_home_eat=2 kid_cap=4 retire=1 synapse=0.00000007"
for s in 1 2 3 4 5 6 7 8; do echo "bash rounds.sh p13/E_$s $s 3 200000 score $X"; echo "bash rounds.sh p13/Erand_$s $s 3 200000 random $X"; done | xargs -P 4 -I{} sh -c '{} > /dev/null 2>&1'
mkdir -p p13l; for a in E Erand; do for s in 1 2 3 4 5 6 7 8; do [ -d p13/${a}_$s/r2 ] && ln -sfn $PWD/p13/${a}_$s/r2 p13l/${a}_$s; done; done
for a in E Erand; do bash fresh_for.sh p13l $a fresh13; done | sed 's/ seed=/ synapse=0.00000007 seed=/' > p13/fresh_jobs.txt
cat p13/fresh_jobs.txt | xargs -P 4 -I{} sh -c '{} > /dev/null 2>&1'
echo done > p13/ALLDONE
