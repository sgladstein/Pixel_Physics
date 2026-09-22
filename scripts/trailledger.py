#!/usr/bin/env python3
"""Read a `trailfollow` sweep off the LEDGER columns, never off `DELIVERED`.

`trailfollow`'s own doc disqualifies `DELIVERED` as a provisioning measure:
`CreatureStats::deliveries` increments on ANY drop while `at_nest`, whatever
was dropped and wherever it came from, and the harness proves it -- at gap 300
`near on` is 0 in all six seeds (not one ant reached the food) while `deliv on`
reads 0, 0, 6, 4, 13, 10.  A change that drives laden ants to the nest inflates
that counter by construction.  `ate J` is the ledger measure that cannot be
fooled, and `came back` is the exact column 7.47's null was about.
"""
import re, statistics as st, glob, os, sys, signal
signal.signal(signal.SIGPIPE, signal.SIG_DFL)
HEAD = re.compile(r"^\s+(\d+)\s+(\d+)\s+(hand)\s+(\d+)/(\d+)\s+(\d+)\s+(\d+)\s+(\d+)\s+(\d+)\s+(\d+)")
RET  = re.compile(r"RETURN LEDGER reached food\s+(\d+) of\s+(\d+) ants \| came back\s+(\d+) \| trips laden\s+(\d+)")
TUMB = re.compile(r"tumbles\s+(\d+) \(homeward\s+(\d+),\s+([\d.]+)%\)")
CARN = re.compile(r"carry->nest\s+(-?\d+)\s+born\s+(\d+) died\s+(\d+) \(starved\s+(\d+)\)")
def rows(p):
    out={}; cur=None
    for l in open(p):
        m=HEAD.match(l)
        if m:
            cur={'ate':int(m[6]), 'at_nest':int(m[9])}; out[int(m[2])]=cur; continue
        if cur is None: continue
        m=RET.search(l)
        if m: cur.update(reached=int(m[1]), back=int(m[3]), laden=int(m[4]))
        m=TUMB.search(l)
        if m: cur.update(homeward_pct=float(m[3]))
        m=CARN.search(l)
        if m: cur.update(carry_nest=int(m[1]), born=int(m[2]), starved=int(m[4]))
    return out
def sign(b,a,k):
    bt=w=t=0
    for s in sorted(set(b)&set(a)):
        if k not in b[s] or k not in a[s]: continue
        x,y=b[s][k],a[s][k]
        if y>x: bt+=1
        elif y<x: w+=1
        else: t+=1
    return bt,w,t
if __name__=="__main__":
    arms={os.path.basename(p)[:-4]: rows(p) for p in sorted(glob.glob(os.path.join(sys.argv[1],'*.log')))}
    base=arms[sys.argv[2]]
    print(f"baseline {sys.argv[2]}, {len(base)} seeds, `hand` arm, paired within seed\n")
    for k,label in (('homeward_pct','homeward tumble %'),
                    ('carry_nest','carry->nest (signed cells home)'),
                    ('back','CAME BACK (round trips)'),
                    ('laden','trips laden'),
                    ('reached','reached food'),
                    ('ate','ate J (LARDER INTAKE)'),
                    ('at_nest','carry@nest'),
                    ('born','born'),
                    ('starved','starved')):
        print(f"  {label:<32}", end="")
        for n in sorted(arms):
            v=[r[k] for r in arms[n].values() if k in r]
            if not v: print(f"  {n}: --", end=""); continue
            med=st.median(v)
            if n==sys.argv[2]: print(f"  {n}: {med:9.1f} (base)  ", end="")
            else:
                bt,w,t=sign(base,arms[n],k)
                print(f"  {n}: {med:9.1f} {bt:2d}/{w:2d}/{t:2d}  ", end="")
        print()
