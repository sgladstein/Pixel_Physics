"""Summarise plant_m0 community logs: per arm (mix x ants), per seed, then quantiles."""
import glob, math, os, re, sys
from collections import defaultdict

D = sys.argv[1]
runs = {}
for path in sorted(glob.glob(os.path.join(D, "runs", "*.log"))):
    name = os.path.basename(path)[:-4]
    text = open(path).read()
    if "plant_m0 done" not in text:
        continue
    rows = []
    for line in text.splitlines():
        if not line.startswith("f="):
            continue
        parts = line.split(" | ")
        r = {"f": int(parts[0][2:]), "sp": {}}
        for p in parts[1:]:
            m = re.match(r"([a-z]+) plants=(\d+) bank=(\d+) leaf=(\d+) est=(\d+)", p)
            if m:
                r["sp"][m.group(1)] = tuple(int(m.group(i)) for i in range(2, 6))
                continue
            for k, v in re.findall(r"([a-z_0-9]+)=([-0-9.,]+)", p):
                r[k] = v
        rows.append(r)
    runs[name] = rows

def q(v, p):
    v = sorted(v)
    if not v:
        return float("nan")
    return v[min(len(v) - 1, int(round(p * (len(v) - 1))))]

def per_run(rows):
    end = rows[-1]
    species = sorted(end["sp"])
    out = {}
    leaf = {s: end["sp"][s][2] for s in species}
    out["alive100"] = sum(1 for s in species if leaf[s] >= 100)
    tot = sum(leaf.values())
    shares = [leaf[s] / tot for s in species if tot > 0 and leaf[s] > 0]
    out["evenness"] = (math.exp(-sum(p * math.log(p) for p in shares)) / len(species)) if shares else 0.0
    # reproducing: established in the last 100k (rows every 25k -> last 4 rows)
    last = rows[-4:]
    out["repro"] = sum(1 for s in species if leaf[s] >= 100 and sum(r["sp"][s][3] for r in last) > 0)
    # trend over the second half: share at end minus share at midpoint, per species, falling ones counted
    mid = rows[len(rows) // 2]
    mt = sum(mid["sp"][s][2] for s in species) or 1
    falling = []
    for s in species:
        a, b = mid["sp"][s][2] / mt, (leaf[s] / tot if tot else 0)
        if b < a * 0.5 and a > 0.02:
            falling.append(s)
    out["falling"] = falling
    out["leaf"] = leaf
    seeds = [int(r["seed"]) for r in rows]
    depth = [int(r["seedmax"]) for r in rows]
    out["seed_peak"], out["depth_peak"], out["seed_end"] = max(seeds), max(depth), seeds[-1]
    water = [[float(x) for x in r["water"].split(",")] for r in rows]
    out["water_end"] = water[-1]
    out["water_min_top"] = min(w[0] for w in water)
    out["water_min_any"] = min(min(w) for w in water)
    out["gaps_end"] = float(end["gaps100"])
    tips = sum(int(r["tips"]) for r in rows)
    out["contact"] = sum(int(r["contact_other"]) for r in rows) / tips if tips else float("nan")
    out["upblock"] = sum(int(r["up_blocked_other"]) for r in rows) / tips if tips else float("nan")
    lc = [float(r["light_cov"]) for r in rows if float(r["light_cov"]) >= 0]
    lo = [float(r["light_open"]) for r in rows if float(r["light_open"]) >= 0]
    out["light_canopy"] = sum(lc) / len(lc) if lc else float("nan")
    out["light_open"] = sum(lo) / len(lo) if lo else float("nan")
    out["tips_total"] = tips
    return out

arms = defaultdict(dict)
for name, rows in runs.items():
    m = re.match(r"(roles4|mix7)-s(\d+)-(.*)", name)
    arms[(m.group(1), m.group(3))][int(m.group(2))] = per_run(rows)

for (mx, arm), seeds in sorted(arms.items()):
    print(f"\n=== {mx} {arm}: {len(seeds)} seeds ({sorted(seeds)})")
    vals = list(seeds.values())
    def line(label, key, fmt="{:.2f}"):
        v = [x[key] for x in vals]
        print(f"  {label:<34} min " + fmt.format(min(v)) + "  p25 " + fmt.format(q(v, .25)) + "  med " + fmt.format(q(v, .5)) + "  max " + fmt.format(max(v)))
    line("kinds with 100+ leaf at 450k", "alive100", "{:.0f}")
    line("of those, reproducing (last 100k)", "repro", "{:.0f}")
    line("evenness (eff. kinds / sown)", "evenness")
    line("seed cells above soil, peak", "seed_peak", "{:.0f}")
    line("deepest seed column, peak", "depth_peak", "{:.0f}")
    line("seed cells above soil, 450k", "seed_end", "{:.0f}")
    line("soil water, top band, lowest", "water_min_top", "{:.3f}")
    line("soil water, any band, lowest", "water_min_any", "{:.3f}")
    line("gaps (3+ bare cols) per 100, 450k", "gaps_end")
    line("tips touching another plant", "contact")
    line("tips blocked upward by another", "upblock")
    line("surface light under canopy", "light_canopy", "{:.3f}")
    line("surface light, open", "light_open", "{:.3f}")
    sp = sorted(vals[0]["leaf"])
    print("  leaf at 450k, per species (med over seeds; present on how many seeds):")
    for s in sp:
        v = [x["leaf"][s] for x in vals]
        print(f"    {s:<10} med {q(v, .5):>7.0f}  p25 {q(v, .25):>7.0f}  max {max(v):>7.0f}  100+ on {sum(1 for x in v if x >= 100)}/{len(v)}")
    fall = defaultdict(int)
    for x in vals:
        for s in x["falling"]:
            fall[s] += 1
    print("  falling in the second half (share halved):", dict(fall))
    w = [x["water_end"] for x in vals]
    print("  soil water by band at 450k (median):", ", ".join(f"{q([r[i] for r in w], .5):.2f}" for i in range(len(w[0]))))
