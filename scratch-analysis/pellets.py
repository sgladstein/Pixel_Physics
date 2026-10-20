"""Every pellet's fate and every trip's phases, from a PP_MIND_LOG of the soil arena (scratch investigation).

usage: pellets.py LOG [SHAFT=4] [FRAMES=6000]
Rebuilds soil_trip_at's per-ant jaws logic: a cut fills the jaws, a put-down empties them.
"""
import sys, collections, statistics as st

LOG = sys.argv[1]
SHAFT = int(sys.argv[2]) if len(sys.argv) > 2 else 4
FRAMES = int(sys.argv[3]) if len(sys.argv) > 3 else 6000
CTX = ["energy", "crop", "spoil", "at_nest", "a", "b", "alarm", "d_a", "d_b", "d_alarm", "d_energy", "home_seen", "home_dist",
       "d_home", "since_nest", "age", "self", "crowd", "depth", "roofed", "roof_cut", "hole_drop"]
VERBS = ["walk", "turn", "rest", "dig", "pickup", "drop", "eat", "feed", "attack"]

worlds, cur = [], {"M": [], "C": [], "D": []}
for line in open(LOG):
    k = line[0]
    p = line.rstrip("\n").split(",")
    if k == "M":
        ctx = [float(x) for x in p[10].split()]
        cur["M"].append({"f": int(p[2]), "id": int(p[3]), "x": int(p[4]), "y": int(p[5]), "verb": int(p[6]), "t": (int(p[7]), int(p[8])),
                         "ctx": dict(zip(CTX, ctx)), "feats": [float(x) for x in p[14].split()]})
    elif k == "C":
        cur["C"].append({"f": int(p[2]), "id": int(p[3]), "x": int(p[4]), "y": int(p[5]), "d": int(p[6]), "g0": p[7] == "1"})
    elif k == "D":
        cur["D"].append({"f": int(p[2]), "id": int(p[3]), "x": int(p[4]), "y": int(p[5]), "away": p[6] == "1"})
    elif k == "E":
        cur["full"] = int(p[2]); worlds.append(cur); cur = {"M": [], "C": [], "D": []}
print(f"worlds parsed: {len(worlds)}; decisions {sum(len(w['M']) for w in worlds)}")

def fates(w):
    """one record per cut: class, fate, frames to put-down"""
    seen, out = set(), []
    ev = sorted([("c", c) for c in w["C"]] + [("d", d) for d in w["D"]], key=lambda e: (e[1]["f"], e[0] == "d"))
    jaws = {}
    for kind, e in ev:
        if kind == "c":
            first = e["g0"] and (e["x"], e["y"]) not in seen
            seen.add((e["x"], e["y"]))
            cls = "recut" if not first else ("foot" if e["d"] >= SHAFT else ("wall" if e["d"] > 0 else "rim"))
            if e["id"] in jaws:
                jaws[e["id"]]["fate"] = "replaced while held"
            rec = {"cls": cls, "cut": e, "fate": "held at end"}
            out.append(rec); jaws[e["id"]] = rec
        else:
            rec = jaws.pop(e["id"], None)
            if rec is None:
                continue
            c = rec["cut"]; lift = min(c["y"] - e["y"], c["d"])
            rec["put"] = e
            rec["fate"] = ("FULL LOAD" if e["away"] and lift >= SHAFT else
                           "away, short lift" if e["away"] else "put down in/near the hole")
    return out

allrec = []
for i, w in enumerate(worlds):
    r = fates(w); allrec += [(i, x) for x in r]
    w["rec"] = r
    assert sum(x["fate"] == "FULL LOAD" and x["cls"] == "foot" for x in r) == w["full"], (i, "rebuild disagrees with the scorer")
print("rebuild reproduces every world's full loads: yes")

# ---- funnel of cuts
tab = collections.Counter((x["cls"], x["fate"]) for _, x in allrec)
print("\nCUTS BY CLASS AND FATE (24 worlds, totals; per world = /24)")
for cls in ("foot", "wall", "rim", "recut"):
    n = sum(v for (c, _), v in tab.items() if c == cls)
    print(f"  {cls:6s} {n:4d} ({n/24:5.2f}/world): " + ", ".join(f"{f} {v}" for (c, f), v in sorted(tab.items()) if c == cls))

# ---- time budget per world: decisions by situation (main run only)
def situation(m):
    c = m["ctx"]
    holding = c["spoil"] > 0.5
    where = "below ground" if c["depth"] > 0.01 else "surface"
    return f"{'carrying' if holding else 'empty'}, {where}"
bud = collections.Counter(); verb_by = collections.defaultdict(collections.Counter)
for w in worlds:
    for m in w["M"]:
        if m["f"] > FRAMES:
            continue
        s = situation(m); bud[s] += 1; verb_by[s][VERBS[m["verb"]]] += 1
tot = sum(bud.values())
print("\nTIME BUDGET (decisions in the 6,000 frames; one decision every ~%.1f frames)" % (FRAMES * 24 / tot))
for s, n in bud.most_common():
    print(f"  {s:22s} {n/tot:5.1%}  verbs: " + ", ".join(f"{v} {k/n:.0%}" for v, k in verb_by[s].most_common(4)))

# ---- trip phases for each foot cut that became a full load
legs = collections.defaultdict(list)
for i, w in enumerate(worlds):
    full = sorted([x for x in w["rec"] if x["fate"] == "FULL LOAD" and x["cls"] == "foot"], key=lambda x: x["cut"]["f"])
    prev_put = 0
    for x in full:
        legs["down: last put-down -> next foot cut"].append(x["cut"]["f"] - prev_put)
        legs["up: foot cut -> put-down away"].append(x["put"]["f"] - x["cut"]["f"])
        prev_put = x["put"]["f"]
print("\nTRIP PHASES, frames (median / mean)")
for k, v in legs.items():
    print(f"  {k:38s} {st.median(v):6.0f} / {st.mean(v):6.0f}   n={len(v)}")
first = [min((x["cut"]["f"] for x in w["rec"] if x["cls"] == "foot"), default=None) for w in worlds]
print(f"  first foot cut at frame (median) {st.median([f for f in first if f]):.0f}")

# ---- per world: what separates the weak worlds
print("\nPER WORLD: full loads, foot cuts, wall+rim cuts, frames wasted on wall/rim cuts' trips")
for i, w in enumerate(worlds):
    c = collections.Counter(x["cls"] for x in w["rec"])
    print(f"  world {i:2d}: full {w['full']:2d}  foot {c['foot']:2d}  wall {c['wall']:2d}  rim {c['rim']:2d}  recut {c['recut']:2d}")

# ---- every cut, traced to the decision that made it
FEAT = {22: "CUT", 25: "UP", 26: "DEPTH", 27: "DROP_BELOW", 28: "HEADROOM", 29: "ROOFED", 39: "HOME", 42: "BACK", 43: "LINED", 1: "SOIL", 2: "SPOIL"}
print("\nEVERY CUT, AT THE DECISION THAT MADE IT (ant depth in rows = ctx depth x 12; median over cuts)")
rows = collections.defaultdict(list)
for i, w in enumerate(worlds):
    dec = {(m["id"], m["t"], m["f"]): m for m in w["M"] if m["verb"] == 3}
    for x in w["rec"]:
        c = x["cut"]
        m = dec.get((c["id"], (c["x"], c["y"]), c["f"])) or dec.get((c["id"], (c["x"], c["y"]), c["f"] - 1))
        if m is None:
            rows["unmatched"].append(None); continue
        rows[x["cls"]].append((m["ctx"]["depth"] * 12, c["d"], m["feats"], m["y"] - c["y"], m["x"] - c["x"]))
print("  unmatched cuts:", len(rows.pop("unmatched", [])))
for cls in ("foot", "wall", "rim", "recut"):
    r = rows[cls]
    if not r: continue
    ad = collections.Counter(round(a) for a, *_ in r)
    side = collections.Counter(("below" if dy < 0 else "above" if dy > 0 else "beside") for *_, dy, dx in r)
    print(f"  {cls:6s} n={len(r):3d}  ant depth (rows) {dict(sorted(ad.items()))}  target is {dict(side)} the ant")
    print("          target feats (mean): " + "  ".join(f"{FEAT[k]} {st.mean(f[k] for _, _, f, *_ in r):.2f}" for k in sorted(FEAT)))
