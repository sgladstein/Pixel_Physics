import csv, json, statistics as st, random
from lib import D, call, HIGHER_BETTER
from alone import P, load, HB
rng = random.Random(5)
needn = {(r["batch"], r["a"], r["b"], r["metric"]): r for r in json.load(open("needn.json"))["rows"]}
out = []
for b, x, y, ms in P:
    A, B = load(b, x), load(b, y)
    if not A or not B: continue
    rs = sorted(set(A) & set(B)); R = rs[-1]
    for m in ms:
        if m not in A[R][next(iter(A[R]))]: continue
        s = HB[m]; calls = {}
        for r in rs:
            cc = [c for c in A[r] if c in B[r] and A[r][c].get(m, "") != "" and B[r][c].get(m, "") != ""]
            d = [s * (float(A[r][c][m]) - float(B[r][c][m])) for c in cc]
            calls[r] = call(d)[0] if len(d) >= 16 else None
        fin = calls[R]; stable = None
        for r in reversed(rs):
            if calls[r] == fin: stable = r
            else: break
        k = tuple(sorted([x, y])); nr = needn.get((b, k[0], k[1], m))
        out.append(dict(batch=b, idea=x, ctl=y, metric=m, final_round=R, final=fin, stable_from=stable,
                        calls={r: calls[r] for r in rs}, need=nr["need"] if nr else None, dz=nr["dz"] if nr else None))
        print(f"{b:17s} {x:13s} {m:8s} final r{R} {fin:+d}  stable from r{stable}  calls " + "".join({1:'+',-1:'-',0:'.',None:'?'}[calls[r]] for r in rs) + f"  |dz| {out[-1]['dz'] or 0:.2f} need N {out[-1]['need']}")
json.dump(out, open("early.json", "w"))
