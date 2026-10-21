"""M1 step 2 (watering): per arm over seeds. Rows every 10k frames.
Spells are 20k wet / 40k dry from frame 0, so f % 60000 == 0 is the end of a
dry spell and f % 60000 == 20000 the end of a wet one."""
import glob, os, re, sys
from collections import defaultdict
D = sys.argv[1]
SP = ["creeper", "grass", "herb", "shrub"]

def parse(path):
    rows = []
    for line in open(path):
        if not line.startswith("f="):
            continue
        parts = line.rstrip("\n").split(" | ")
        r = {"f": int(parts[0][2:]), "sp": {}}
        for p in parts[1:]:
            m = re.match(r"([a-z]+) plants=(\d+) bank=(\d+) leaf=(\d+) est=(\d+) rootwet=([-0-9.]+) rootdepth=([-0-9.]+)", p)
            if m:
                r["sp"][m.group(1)] = dict(plants=int(m.group(2)), leaf=int(m.group(4)), est=int(m.group(5)), rootwet=float(m.group(6)), rootdepth=float(m.group(7)))
                continue
            for k, v in re.findall(r"([a-z_0-9]+)=([^ ]+)", p):
                r[k] = v
        rows.append(r)
    return rows

def med(v):
    v = sorted(x for x in v if x == x)
    return v[len(v) // 2] if v else float("nan")

runs = defaultdict(dict)
for path in sorted(glob.glob(os.path.join(D, "runs", "*.log"))):
    if "plant_m0 done" not in open(path).read():
        continue
    arm, seed = re.match(r"(\w+)-s(\d+)\.log", os.path.basename(path)).groups()
    runs[arm][int(seed)] = parse(path)

for arm in ["uniform", "patchy"]:
    seeds = runs.get(arm, {})
    if not seeds:
        continue
    print(f"\n=== {arm}: seeds {sorted(seeds)}")
    at = lambda rows, f: next((r for r in rows if r["f"] == f), None)
    for f in [120000, 240000, 360000, 440000, 450000, 480000]:
        rs = [at(rows, f) for rows in seeds.values()]
        rs = [r for r in rs if r]
        if not rs:
            continue
        st = [int(r["standing"]) for r in rs]
        bands = [[float(x) for x in r["water"].split(",")] for r in rs]
        prof = ",".join(f"{med([b[i] for b in bands]):.2f}" for i in range(len(bands[0])))
        tsd = med([float(r["topsd"]) for r in rs])
        bsd = [[float(x) for x in r["bandsd"].split(",")] for r in rs]
        bprof = ",".join(f"{med([b[i] for b in bsd]):.3f}" for i in range(len(bsd[0])))
        nut, nutsd = med([float(r["nut"]) for r in rs]), med([float(r["nutsd"]) for r in rs])
        print(f"  f={f:>6} {rs[0].get('spell','none'):>4}  standing per seed {st}  soil water by band (med) {prof}")
        print(f"           spread across columns by band (med) {bprof}  nutrient share {nut:.3f} spread {nutsd:.3f}")
    end = [rows[-1] for rows in seeds.values()]
    print(f"  rained / drained (cells, per seed): {[(int(r['rained']), int(r['drained'])) for r in end]}")
    print(f"  standing max over the run, per seed: {[max(int(r['standing']) for r in rows) for rows in seeds.values()]}")
    print("  per species at 480k (median over seeds): leaf | plants | established in last 100k")
    for s in SP:
        leaf = [rows[-1]["sp"][s]["leaf"] for rows in seeds.values()]
        pl = [rows[-1]["sp"][s]["plants"] for rows in seeds.values()]
        est = [sum(r["sp"][s]["est"] for r in rows[-10:]) for rows in seeds.values()]
        print(f"    {s:<8} leaf {leaf} plants {pl} est {est}")
    # Root water at matched phases, last two cycles: end of dry (f%60k==0) and end of wet (f%60k==20k).
    print("  water around roots (median over seeds and the last two cycles) / mean deepest root, rows")
    for s in SP:
        def pick(ph):
            return med([r["sp"][s]["rootwet"] for rows in seeds.values() for r in rows if r["f"] >= 360000 and r["f"] % 60000 == ph and r["sp"][s]["rootwet"] >= 0])
        depth = med([r["sp"][s]["rootdepth"] for rows in seeds.values() for r in rows if r["f"] >= 360000 and r["sp"][s]["rootdepth"] >= 0])
        print(f"    {s:<8} after dry {pick(0):.3f}  after wet {pick(20000):.3f}  depth {depth:.1f}")
