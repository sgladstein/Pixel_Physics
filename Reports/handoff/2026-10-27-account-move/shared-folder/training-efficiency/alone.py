"""For designated idea-vs-control pairs: call vs control (final garden) and call vs the idea arm's own first garden (what 'run it alone' sees)."""
import csv, statistics as st, json
from lib import D, binom_p, call, HIGHER_BETTER
P = [("colsel3-1022","cs3lexlesinh","cs3lex",["nest","starved"]),
     ("colsel8-1022","cs8hcs","cs8hA1",["starved","nest"]),
     ("colsel9b-1022","cs9joincr","cs9hcs",["nest","home8"]),
     ("colsel15-1023","cs15roof","cs15ctl",["nest","crater"]),
     ("colsel15-1023","cs15both","cs15ctl",["nest","crater"]),
     ("colsel20-1023","cs20light","cs20wean",["nest"]),
     ("colsel23-1023","cs23inh","cs23ctl",["nest"]),
     ("brood1b-1023","brood1crutch","brood1ctl",["crater","nest"]),
     ("broodplant1c-1024","bp1","bp1pile",["starved","nest"]),
     ("stdcheck-1024b","stdoff","stdwean",["nest"]),
     ("farfood1-1024","farramp","farctl",["lab","nest"]),
     ("farfood2-1024","ramp2far","ramp2",["lab","nest"])]
HB = dict(HIGHER_BETTER, lab=1, late_cuts=1)
def load(b, a):
    by = {}
    try:
        for r in csv.DictReader(open(f"{D}/{b}/{a}/cg.csv")):
            by.setdefault(int(r["round"]), {})[int(r["colony"])] = r
    except FileNotFoundError:
        return None
    return by
out = []
for b, x, y, ms in P:
    A, B = load(b, x), load(b, y)
    if not A or not B: print("missing", b, x, y); continue
    R = max(set(A) & set(B)); r1 = min(A)
    for m in ms:
        if m not in A[R][next(iter(A[R]))]: continue
        s = HB[m]
        def v(T, r, c):
            try: return float(T[r][c][m])
            except (KeyError, ValueError): return None
        cc = [c for c in A[R] if v(A, R, c) is not None and v(B, R, c) is not None]
        dv = [s * (v(A, R, c) - v(B, R, c)) for c in cc]
        c1 = [c for c in A[R] if v(A, R, c) is not None and v(A, r1, c) is not None]
        da = [s * (v(A, R, c) - v(A, r1, c)) for c in c1]
        db = [s * (v(B, R, c) - v(B, r1, c)) for c in c1 if v(B, R, c) is not None and v(B, r1, c) is not None]
        kv, pv = call(dv); ka, pa = call(da); kb, pb = call(db)
        out.append(dict(batch=b, idea=x, ctl=y, metric=m, R=R, vs_ctl=kv, p_ctl=pv, alone=ka, p_alone=pa, ctl_drift=kb,
            med_idea=st.median(v(A, R, c) for c in cc), med_ctl=st.median(v(B, R, c) for c in cc),
            med_idea_r1=st.median(v(A, r1, c) for c in c1), med_ctl_r1=st.median(v(B, r1, c) for c in c1 if v(B, r1, c) is not None)))
        o = out[-1]
        print(f"{b:17s} {x:13s} vs {y:10s} {m:8s} r{R}: idea {o['med_idea_r1']:.0f}->{o['med_idea']:.0f}  ctl {o['med_ctl_r1']:.0f}->{o['med_ctl']:.0f} | vs ctl {kv:+d} (p {pv:.3f})  alone {ka:+d}  ctl drift {kb:+d}")
json.dump(out, open("alone.json", "w"), indent=0)
