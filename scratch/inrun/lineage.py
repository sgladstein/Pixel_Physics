"""SCRATCH (teacher lane, 2026-10-27): how many round-0 colonies still have descendants, round by round.

usage: lineage.py OUT/colsel.csv [...]   (follows refounded_from; 32 colonies assumed)
Found every lexicase arm of self1/elite1/drift1 down to ONE founder colony by round 7-15 (plain, with no refounding, keeps
32). Per-ant families at the last round: the family column of final/index.csv (elite1: one pool member per arm).
"""
import csv,sys
def lineages(path):
    rows=list(csv.DictReader(open(path)))
    R=max(int(r['round']) for r in rows); lin={c:c for c in range(1,33)}; out=[]
    by={}
    for r in rows: by.setdefault(int(r['round']),[]).append(r)
    for rd in range(R+1):
        new=dict(lin)
        for r in by.get(rd,[]):
            if r['refounded_from']: new[int(r['colony'])]=lin[int(r['refounded_from'])]
        lin=new; out.append((rd,len(set(lin.values()))))
    return out,lin
for p in sys.argv[1:]:
    o,lin=lineages(p)
    print(p.split('scratchpad/')[-1], ' '.join(f'r{r}:{n}' for r,n in o if r in (0,7,15,31,47,63,95,191) or r==o[-1][0]), '| final founders:', sorted(set(lin.values())))
