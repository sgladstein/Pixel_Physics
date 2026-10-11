"""tripclass.py RUN [FROM TO]: every spell with food in the crop (ticks.csv.gz only, so it reads a
shipped run and a walk run the same way), keyed by where the food was picked up, and how the spell
ended: delivered (a delivery counted in the step the crop fell), put down (crop fell by >50 J in
one step with no delivery), digested (crop ran out without a big step), or still open at the window's
end. A spell counts in the window if it STARTED in [FROM, TO]."""
import sys, csv, gzip, collections, statistics
run = sys.argv[1]
a = int(sys.argv[2]) if len(sys.argv) > 2 else 80000
b = int(sys.argv[3]) if len(sys.argv) > 3 else 100000
by = collections.defaultdict(list)
for r in csv.DictReader(gzip.open(f"{run}/ticks.csv.gz", "rt")):
    by[r["id"]].append(r)
def fl(x):
    try: return float(x)
    except: return 0.0
spells = []
for id_, rows in by.items():
    rows.sort(key=lambda r: int(r["frame"]))
    cur = None
    for r in rows:
        f = int(r["frame"])
        c0 = fl(r["crop_cells"]); c1 = c0 + fl(r["d_crop_cells"])
        if cur is None and c0 == 0 and c1 > 0:
            cur = {"id": id_, "start": f, "zone0": r["zone"], "nb": r["nest_bound"], "e0": fl(r["energy_j"]),
                   "j0": fl(r["crop_j"]) + fl(r["d_crop_j"]), "zones": collections.Counter(), "deliv": 0}
            continue
        if cur is None:
            continue
        cur["zones"][r["zone"]] += 1
        cur["deliv"] += fl(r["d_deliveries"]) > 0
        if c0 > 0 and c1 == 0:
            if fl(r["d_deliveries"]) > 0:
                how = "delivered"
            elif fl(r["d_crop_j"]) < -50:
                how = "put down"
            else:
                how = "digested"
            cur.update(end=f, how=how, zone1=r["zone"], e1=fl(r["energy_j"]))
            spells.append(cur)
            cur = None
    if cur is not None:
        cur.update(end=None, how="open", zone1=rows[-1]["zone"], e1=fl(rows[-1]["energy_j"]))
        spells.append(cur)
sel = [s for s in spells if a <= s["start"] <= b]
print(f"{run}: {len(by)} traced ants, {len(sel)} crop spells starting {a}-{b}")
tab = collections.defaultdict(collections.Counter)
for s in sel:
    tab[s["zone0"]][s["how"]] += 1
hows = ["delivered", "put down", "digested", "open"]
print(f"{'picked up in':<12} " + " ".join(f"{h:>10}" for h in hows))
for z in sorted(tab, key=lambda z: -sum(tab[z].values())):
    print(f"{z:<12} " + " ".join(f"{tab[z][h]:>10}" for h in hows))
for z in ("food",):
    for h in hows:
        ss = [s for s in sel if s["zone0"] == z and s["how"] == h and s["end"]]
        if ss:
            lens = [s["end"] - s["start"] for s in ss]
            ez = collections.Counter(s["zone1"] for s in ss)
            spent = collections.Counter()
            for s in ss:
                spent.update(s["zones"])
            tot = sum(spent.values()) or 1
            print(f"  from {z}, {h}: n={len(ss)} frames median {statistics.median(lens):.0f}; ended in {dict(ez)}; "
                  f"time in " + ", ".join(f"{k} {100*v/tot:.0f}%" for k, v in spent.most_common()))
