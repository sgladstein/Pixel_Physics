"""Part 2: the brain's wiring and the exact math of its decisions (scratch investigation).

usage: brainmath.py GENOME LOG
Layout constants mirror src/sim/creature/mind.rs (GENOME_LEN must come out 2797).
"""
import sys, collections, statistics as st

GEN, LOG = sys.argv[1], sys.argv[2]
VERBS, MEMORY, CONTEXT, UNITS, FEATURES, EMITS, HIDDEN = 9, 12, 22, 16, 45, 3, 8
SWITCH = 0; SHARP = SWITCH + VERBS; PULL = SHARP + 1; RATE = PULL + 1; S_CTX = RATE + MEMORY
S_IN = CONTEXT + MEMORY + 1; S_FEAT = S_CTX + UNITS * S_IN; S_VERB = S_FEAT + UNITS * FEATURES; S_OUT = S_VERB + UNITS * VERBS
JOB = S_OUT + UNITS + 1; J_IN = CONTEXT + MEMORY + 2; MEM = JOB + VERBS * J_IN; M_IN = CONTEXT + 2 + VERBS + MEMORY + 1
EMIT = MEM + MEMORY * M_IN; E_IN = CONTEXT + MEMORY + 1; HID = EMIT + EMITS * E_IN; H_IN = CONTEXT + MEMORY + 1
HID_OUT = HID + HIDDEN * H_IN; GENOME_LEN = HID_OUT + VERBS * HIDDEN
assert GENOME_LEN == 2797, GENOME_LEN
CTX = ["energy", "crop", "spoil", "at_nest", "a", "b", "alarm", "d_a", "d_b", "d_alarm", "d_energy", "home_seen", "home_dist",
       "d_home", "since_nest", "age", "self", "crowd", "depth", "roofed", "roof_cut", "hole_drop"]
VN = ["walk", "turn", "rest", "dig", "pickup", "drop", "eat", "feed", "attack"]
FN = {0: "AIR", 1: "SOIL", 2: "SPOIL", 3: "FOOD", 4: "BROOD", 5: "CORPSE", 6: "STONE", 7: "WATER", 8: "ANT", 9: "SCENT"}
FN.update({18: "BUILDING", 19: "STAND", 20: "FALL", 21: "CLIMB", 22: "CUT", 23: "HARD", 24: "WET", 25: "UP", 26: "DEPTH", 27: "DROP_BELOW",
           28: "HEADROOM", 29: "ROOFED", 30: "NESTMATE", 31: "STRANGER", 32: "LADEN", 33: "HUNGRY", 34: "LARVA", 35: "LARVA_HUNGER",
           36: "DEAD", 37: "CROWD", 38: "ROOM", 39: "HOME", 40: "LAST_FOOD", 41: "WARMTH", 42: "BACK", 43: "LINED", 44: "ROOF_DROP"})
for k in range(10, 18): FN.setdefault(k, f"F{k}")
EPS = 1e-6

g = [0.0] * GENOME_LEN
lines = open(GEN).read().splitlines()
for l in lines[1:]:
    i, v = l.split(); g[int(i)] = float(v)
sq = lambda x: x / (1 + abs(x))
sharp = min(max(1 + g[SHARP], 0.25), 4.0); pull = min(max(g[PULL], 0.0), 1.0)
on = [g[SWITCH + v] > 0 for v in range(VERBS)]
live = lambda w: abs(w) > EPS

# ------------------------------------------------------------------ 1. the wiring
print(f"verbs switched on: {[VN[v] for v in range(VERBS) if on[v]]}; sharpness {sharp:.2f}; pull {pull:.2f}")
blocks = {"scorer: situation -> units": (S_CTX, S_FEAT), "scorer: target features -> units": (S_FEAT, S_VERB), "scorer: verb -> units": (S_VERB, S_OUT),
          "scorer: units -> score": (S_OUT, JOB), "job head": (JOB, MEM), "memory": (MEM, EMIT), "scent emits": (EMIT, HID),
          "hidden units": (HID, HID_OUT), "hidden -> jobs": (HID_OUT, GENOME_LEN)}
tot = 0
for k, (a, b) in blocks.items():
    n = sum(live(g[i]) for i in range(a, b)); tot += n
    print(f"  {k:34s} {n:4d} live of {b - a}")
print(f"  total live links {tot}")
unit_on = [live(g[S_OUT + u]) for u in range(UNITS)]
hid_read = [any(live(g[HID_OUT + v * HIDDEN + h]) for v in range(VERBS) if on[v]) for h in range(HIDDEN)]
print(f"  scorer units that reach the score: {sum(unit_on)} of {UNITS}; hidden units some job reads: {sum(hid_read)} of {HIDDEN}")

print("\nEACH SITUATION INPUT: live links into the verb choice (job head, direct), via hidden units, into memory, into the scorer's live units")
for c in range(CONTEXT):
    job = sum(live(g[JOB + v * J_IN + c]) for v in range(VERBS) if on[v])
    hid = sum(live(g[HID + h * H_IN + c]) for h in range(HIDDEN) if hid_read[h])
    mem = sum(live(g[MEM + m * M_IN + c]) for m in range(MEMORY))
    sco = sum(live(g[S_CTX + u * S_IN + c]) for u in range(UNITS) if unit_on[u])
    print(f"  {CTX[c]:11s} job {job}  hidden {hid}  memory {mem}  scorer {sco}" + ("   (NO PATH)" if job + hid + mem + sco == 0 else ""))
print("\nTARGET FEATURES with any live link into a live scorer unit:")
fl = {}
for k in range(FEATURES):
    w = [g[S_FEAT + u * FEATURES + k] * g[S_OUT + u] for u in range(UNITS) if unit_on[u] and live(g[S_FEAT + u * FEATURES + k])]
    if w: fl[k] = (len(w), sum(abs(x) for x in w))
print("  " + ", ".join(f"{FN[k]} x{n}" for k, (n, _) in sorted(fl.items(), key=lambda kv: -kv[1][1])))
print("  none:", ", ".join(FN[k] for k in range(FEATURES) if k not in fl))

# ------------------------------------------------------------------ 2. reproduce, then attribute
rows = []
for line in open(LOG):
    if line[0] != "M": continue
    p = line.rstrip("\n").split(",")
    f = int(p[2])
    if f > 6000: continue
    rows.append({"f": f, "verb": int(p[6]), "last": int(p[9]), "ctx": [float(x) for x in p[10].split()], "mem": [float(x) for x in p[11].split()],
                 "jobs": [float(x) for x in p[12].split()], "best": [float(x) for x in p[13].split()], "feats": [float(x) for x in p[14].split()]})

def job_terms(r, v):
    row = g[JOB + v * J_IN: JOB + (v + 1) * J_IN]
    t = {"best target": row[CONTEXT + MEMORY] * r["best"][v], "bias": row[CONTEXT + MEMORY + 1]}
    for c in range(CONTEXT): t[CTX[c]] = row[c] * r["ctx"][c]
    t["memory"] = sum(row[CONTEXT + m] * r["mem"][m] for m in range(MEMORY))
    hid = []
    for h in range(HIDDEN):
        if not hid_read[h]: hid.append(0.0); continue
        hr = g[HID + h * H_IN: HID + (h + 1) * H_IN]
        s = hr[H_IN - 1] + sum(hr[c] * r["ctx"][c] for c in range(CONTEXT)) + sum(hr[CONTEXT + m] * r["mem"][m] for m in range(MEMORY))
        hid.append(sq(s))
    t["hidden units"] = sum(g[HID_OUT + v * HIDDEN + h] * hid[h] for h in range(HIDDEN))
    t["pull (same verb as last)"] = pull if r["last"] == v else 0.0
    return t

err = 0.0; n = 0
for r in rows:
    for v in range(VERBS):
        if r["jobs"][v] != r["jobs"][v]: continue  # nan: verb had no target
        j = sq(sharp * sum(job_terms(r, v).values()))
        err = max(err, abs(j - r["jobs"][v])); n += 1
print(f"\nREPRODUCTION: {n} logged job scores recomputed from the genome file; worst error {err:.5f} (logged to 4 decimals)")

def situation(r):
    c = dict(zip(CTX, r["ctx"]))
    return ("carrying" if c["spoil"] > 0.5 else "empty") + (", below ground" if c["depth"] > 0.01 else ", on the surface")

# the margin between the chosen verb and the strongest other verb, split into exact terms (pre-squash)
print("\nWHAT DECIDES THE VERB: mean |term(chosen) - term(strongest rival)|, pre-squash, by situation (top 7 terms)")
agg = collections.defaultdict(lambda: collections.defaultdict(list))
for r in rows:
    av = [v for v in range(VERBS) if r["jobs"][v] == r["jobs"][v]]
    if len(av) < 2: continue
    rival = max((v for v in av if v != r["verb"]), key=lambda v: r["jobs"][v])
    tc, tr = job_terms(r, r["verb"]), job_terms(r, rival)
    key = situation(r) + f" | chose {VN[r['verb']]} over {VN[rival]}"
    for k in tc: agg[key][k].append(tc[k] - tr[k])
for key in sorted(agg, key=lambda k: -len(next(iter(agg[k].values())))):
    d = agg[key]; nn = len(next(iter(d.values())))
    if nn < 150: continue
    top = sorted(d, key=lambda k: -st.mean(abs(x) for x in d[k]))[:7]
    print(f"  {key}  (n={nn})")
    print("     " + "  ".join(f"{k} {st.mean(d[k]):+.2f}" for k in top))

# ---- where the reproduction error sits
errs = []
for r in rows:
    for v in range(VERBS):
        if r["jobs"][v] != r["jobs"][v]: continue
        errs.append(abs(sq(sharp * sum(job_terms(r, v).values())) - r["jobs"][v]))
errs.sort()
print(f"\nreproduction error: median {errs[len(errs)//2]:.6f}, p99 {errs[int(.99*len(errs))]:.6f}, share over 0.001: {sum(e > 1e-3 for e in errs)/len(errs):.4%}")

# ---- the hidden circuit behind Drop, and what feeds each unit
def hid_vals(r):
    out = []
    for h in range(HIDDEN):
        hr = g[HID + h * H_IN: HID + (h + 1) * H_IN]
        terms = {"bias": hr[H_IN - 1]}
        for c in range(CONTEXT): terms[CTX[c]] = hr[c] * r["ctx"][c]
        terms["memory"] = sum(hr[CONTEXT + m] * r["mem"][m] for m in range(MEMORY))
        out.append((sq(sum(terms.values())), terms))
    return out
drop = VN.index("drop"); walk = VN.index("walk")
print("\nHIDDEN UNITS: weight into the drop job and the walk job")
for h in range(HIDDEN):
    if hid_read[h]:
        print(f"  h{h}: drop {g[HID_OUT + drop * HIDDEN + h]:+.2f}  walk {g[HID_OUT + walk * HIDDEN + h]:+.2f}  dig {g[HID_OUT + 3 * HIDDEN + h]:+.2f}")
car = [r for r in rows if r["ctx"][CTX.index("spoil")] > 0.5 and r["ctx"][CTX.index("depth")] <= 0.01]
for label, sel in (("chose drop", [r for r in car if r["verb"] == drop]), ("chose walk", [r for r in car if r["verb"] == walk])):
    hv = [hid_vals(r) for r in sel]
    print(f"\n  carrying on the surface, {label} (n={len(sel)}): hidden unit values and their biggest input terms")
    for h in range(HIDDEN):
        if not hid_read[h] or abs(g[HID_OUT + drop * HIDDEN + h]) < 0.5: continue
        vals = [x[h][0] for x in hv]
        terms = collections.defaultdict(list)
        for x in hv:
            for k, t in x[h][1].items(): terms[k].append(t)
        top = sorted(terms, key=lambda k: -abs(st.mean(terms[k])))[:5]
        print(f"    h{h} = {st.mean(vals):+.2f}   " + "  ".join(f"{k} {st.mean(terms[k]):+.2f}" for k in top))
# what the deciding inputs look like at drop vs walk
for k in ("home_dist", "since_nest", "d_home", "at_nest", "a", "d_a", "energy", "age"):
    i = CTX.index(k)
    a_ = [r["ctx"][i] for r in car if r["verb"] == drop]; b_ = [r["ctx"][i] for r in car if r["verb"] == walk]
    print(f"    {k:10s} at drop {st.mean(a_):+.3f}   at walk {st.mean(b_):+.3f}")
