# Toy model of colsel's survival step, calibrated to the measured numbers:
# world noise sd 1; between-colony genetic sd 0.25 (repeatability 0.06 of the composite);
# a child = donor + N(-load, spread) (children worse than their donor in ~58% of same-round pairs).
import random, statistics as st
def run(ncol=32, worlds=1, prior=None, load=0.28, spread=0.30, rounds=96, seed=0, ndonor=4):
    R=random.Random(seed)
    g=[0.0]*ncol; hist=[[] for _ in range(ncol)]; fresh=[True]*ncol
    child_top=[]
    for r in range(rounds):
        obs=[st.mean(g[c]+R.gauss(0,1) for _ in range(worlds)) for c in range(ncol)]
        for c in range(ncol): hist[c].append(obs[c])
        med=st.median(obs)
        if prior is None: score=obs
        else: score=[(sum(h)+prior*med)/(len(h)+prior) for h in hist]
        order=sorted(range(ncol), key=lambda c:-score[c])
        top,bot=order[:ncol//2],order[ncol//2:]
        child_top.append(sum(1 for c in top if fresh[c])/len(top))
        donors=top[:ndonor]
        for c in top: fresh[c]=False
        for i,b in enumerate(bot):
            d=donors[i%len(donors)]
            g[b]=g[d]-load+R.gauss(0,spread); hist[b]=[]; fresh[b]=True
    return st.mean(g), st.mean(child_top[1:])
for label,kw,cost in [('now: 32 colonies x 1 world',dict(),1),('memory (shrunk mean, prior 2)',dict(prior=2),1),('memory prior 1',dict(prior=1),1),
                      ('32 x 2 worlds',dict(worlds=2),2),('16 colonies x 2 worlds',dict(ncol=16,worlds=2,ndonor=2),1),('32 x 2 worlds + memory',dict(worlds=2,prior=2),2),
                      ('no selection info (random order)',dict(worlds=1,prior=None,load=0.28),1)]:
    if label.startswith('no selection'):
        continue
    v=[run(seed=s,**kw) for s in range(40)]
    print(f'{label:34s} cost x{cost}: genetic value after 96 rounds {st.mean(x[0] for x in v):+.2f} (sd over runs {st.pstdev([x[0] for x in v]):.2f}); children in top half {st.mean(x[1] for x in v):.0%}')
print('--- donors per round')
for nd in (1,2,4,8,16):
    v=[run(seed=s,ndonor=nd) for s in range(40)]
    print(f'donors {nd:2d}: {st.mean(x[0] for x in v):+.2f}')
print('--- load sensitivity (memory vs not)')
for load,spread in ((0.28,0.15),(0.1,0.3),(0.0,0.3)):
    a=st.mean(run(seed=s,load=load,spread=spread)[0] for s in range(40)); b=st.mean(run(seed=s,load=load,spread=spread,prior=2)[0] for s in range(40))
    print(f'load {load} spread {spread}: now {a:+.2f}  memory {b:+.2f}')
