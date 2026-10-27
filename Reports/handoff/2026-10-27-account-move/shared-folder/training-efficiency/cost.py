import glob, csv, collections, os
from lib import D
tot = collections.Counter(); rows = []
for p in sorted(glob.glob(f"{D}/*/*.timing.csv")):
    b = p.split("/")[-2]; a = os.path.basename(p)[:-11]
    c = collections.Counter(); n = collections.Counter(); cg_rounds = set(); last = 0
    for r in csv.DictReader(open(p)):
        c[r["kind"]] += float(r["secs"]); n[r["kind"]] += 1
        if r["kind"] == "cg": cg_rounds.add(int(r["round"]))
        else: last = max(last, int(r["round"]))
    rows.append((b, a, c["train"], c["cg"], n["train"], n["cg"], len(cg_rounds), last + 1))
    tot["train"] += c["train"]; tot["cg"] += c["cg"]
for r in rows:
    print(f"{r[0]:18s} {r[1]:14s} train {r[2]/3600:6.1f} cpu-h ({r[4]} runs, {r[7]} rounds)  garden {r[3]/3600:5.1f} cpu-h ({r[5]} runs, {r[6]} gardens)  garden share {r[3]/(r[2]+r[3]):.0%}")
print(f"TOTAL train {tot['train']/3600:.0f} cpu-h, garden {tot['cg']/3600:.0f} cpu-h, garden share {tot['cg']/(tot['train']+tot['cg']):.0%}")
