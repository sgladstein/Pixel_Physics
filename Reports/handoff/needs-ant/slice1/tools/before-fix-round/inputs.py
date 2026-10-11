"""inputs.py ZONE RUN... [--from A --to B]: for empty-jawed decisions in ZONE, the median of every brain
input, the hidden units and o_Move/o_Dig/p_move, side by side across runs; rows where runs differ most first."""
import sys, csv, gzip, statistics
args = sys.argv[1:]
zone = args.pop(0)
a, b = 80000, 100000
if "--from" in args:
    i = args.index("--from"); a = int(args[i + 1]); del args[i:i + 2]
if "--to" in args:
    i = args.index("--to"); b = int(args[i + 1]); del args[i:i + 2]
cols = None; med = []
for run in args:
    vals = {}
    for r in csv.DictReader(gzip.open(f"{run}/ticks.csv.gz", "rt")):
        if cols is None:
            cols = [c for c in r if c.startswith("i_") or c.startswith("h") and c[1:].isdigit()] + ["o_Move", "o_Dig", "p_move", "energy_j"]
        f = int(r["frame"])
        if not (a <= f <= b) or r["zone"] != zone: continue
        if r["spoil"] not in ("0", "") or r["crop_cells"] not in ("0", ""): continue
        for c in cols:
            try: vals.setdefault(c, []).append(float(r[c]))
            except ValueError: pass
    med.append({c: statistics.median(v) for c, v in vals.items()})
    med[-1]["_n"] = len(vals.get("p_move", []))
print(f"empty-jawed decisions in {zone}, {a}-{b}; n = " + ", ".join(str(m["_n"]) for m in med))
rows = []
for c in cols:
    v = [m.get(c, float("nan")) for m in med]
    rows.append((max(v) - min(v), c, v))
for d, c, v in sorted(rows, key=lambda x: -x[0]):
    if d == 0 and all(x == 0 for x in v): continue
    print(f"  {c:<18} " + " ".join(f"{x:>8.3f}" for x in v))
