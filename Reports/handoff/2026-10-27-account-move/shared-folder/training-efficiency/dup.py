"""Find arms that are exact reruns of an earlier arm (determinism): colsel arms by colsel.csv rows, in-run arms by per-seed nest/stock rows."""
import glob, os, re, hashlib, collections, csv, sys
from lib import D
# colsel arms
sig = {}
for p in sorted(glob.glob(f"{D}/*/*/colsel.csv")):
    b, a = p.split("/")[-3], p.split("/")[-2]
    rows = list(csv.reader(open(p)))
    hdr = rows[0]; keep = [i for i, h in enumerate(hdr) if h in ("home8","late_cuts","starved","alive","cuts","nest","crater")]
    per = collections.defaultdict(list)
    for r in rows[1:]:
        per[int(r[0])].append((int(r[1]), tuple(r[i] for i in keep)))
    sig[(b, a)] = [hashlib.md5(repr(sorted(per[k])).encode()).hexdigest() for k in sorted(per)]
keys = sorted(sig)
print("COLSEL exact-prefix matches (rounds identical from round 0):")
for i, x in enumerate(keys):
    for y in keys[i+1:]:
        if x[0] == y[0]: continue
        n = 0
        for u, v in zip(sig[x], sig[y]):
            if u != v: break
            n += 1
        if n >= 2:
            print(f"  {x[0]}/{x[1]} ({len(sig[x])} rounds) == {y[0]}/{y[1]} ({len(sig[y])} rounds) for first {n} rounds")
# in-run arms
arm = collections.defaultdict(dict)
for p in glob.glob(f"{D}/*/*.inrun.csv"):
    b = p.split("/")[-2]; job = os.path.basename(p)[:-10]
    m = re.match(r"(.*)_(\d+)$", job)
    if not m: continue
    rows = sorted(open(p).read().splitlines()[1:])
    arm[(b, m.group(1))][int(m.group(2))] = hashlib.md5("\n".join(rows).encode()).hexdigest()
ks = sorted(arm)
print("IN-RUN arms with identical per-seed outputs (share of seeds identical):")
for i, x in enumerate(ks):
    for y in ks[i+1:]:
        if x == y: continue
        common = set(arm[x]) & set(arm[y])
        if not common: continue
        same = sum(arm[x][s] == arm[y][s] for s in common)
        if same >= max(4, len(common) // 2):
            print(f"  {x[0]}/{x[1]} == {y[0]}/{y[1]}: {same}/{len(common)} seeds identical")
