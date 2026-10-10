#!/usr/bin/env python3
"""Every table in playtest-2026-10-09-herb-ant/README.md, from the run directories `run.sh` writes.

    python3 tables.py RUNS_DIR [PLAYTEST_DIR]

RUNS_DIR holds s<seed>_<arm>/ (+ .log beside each). PLAYTEST_DIR defaults to ./playtest (the chronicle's census).
An arm or seed that is missing is skipped, so a partial set of runs prints what it can. Needs pandas and numpy.

Definitions that the README relies on, stated once:
  adult-tick   one adult alive for one tick; adult-Mticks are summed from probe.csv's `adults` x probe interval,
               from tick 85,000 (the colonies are placed at 83,271 and 84,889) to the window's end.
  low foliage  leaf cells within 15 rows of the ground (probe.csv leaf_h0_3 + leaf_h4_15).
  established  a plant of >= 50 cells (plants_F.csv); smaller organisms are seedlings and seeds.
  near a nest  plant centroid within 64 columns of x=728 or x=284.
  wood share   of the dig decisions whose roll was won AND whose target was soil, packed soil, spoil, wood, root,
               deadwood or log, the share whose target was wood, root, deadwood or log (digs.csv).
"""
import glob
import os
import sys

import numpy as np
import pandas as pd

RUNS = sys.argv[1]
PLAY = sys.argv[2] if len(sys.argv) > 2 else "playtest"
pd.set_option("display.width", 220)
pd.set_option("display.max_columns", 60)


def run_dir(seed, arm):
    d = f"{RUNS}/s{seed}_{arm}"
    return d if os.path.isdir(d) and "replay done" in open(d + ".log").read() else None


def probe(d):
    return pd.read_csv(f"{d}/probe.csv").set_index("frame")


def census(d):
    return pd.read_csv(sorted(glob.glob(f"{d}/chron/*.census.csv"))[-1]).set_index("frame")


def adult_mticks(p, until):
    ad = p["adults"]
    ad = ad[(ad.index <= until) & (ad.index >= 85000)]
    return float((ad * (p.index[1] - p.index[0])).sum()) / 1e6


def seeds_of(arm):
    return [s for s in range(1, 9) if run_dir(s, arm)]


def med(rows, key):
    return float(np.median([r[key] for r in rows])) if rows else float("nan")


def section(t):
    print("\n" + "=" * 100 + f"\n{t}\n" + "=" * 100)


# ---------------------------------------------------------------------------------------------------------------
section("S3  Stand at tick 250,000 (plants / low foliage / total leaf / adults) and peak adults, per seed and arm")
for arm in ("alone", "ants", "noleaf", "noseed", "noleafseed", "leaf20", "leaf160", "doortree"):
    rows = []
    for s in seeds_of(arm):
        p = probe(run_dir(s, arm))
        if 250000 not in p.index:
            continue
        rows.append(dict(seed=s, plants=int(p.loc[250000, "plants"]),
                         low=int(p.loc[250000, "leaf_h0_3"] + p.loc[250000, "leaf_h4_15"]),
                         leaf=int(p.loc[250000, "leaf_cells"]), adults=int(p.loc[250000, "adults"]),
                         peak=int(p["adults"].max())))
    if rows:
        df = pd.DataFrame(rows).set_index("seed")
        print(f"\n{arm}  (median: plants {med(rows,'plants'):.0f}, low {med(rows,'low'):.0f}, leaf {med(rows,'leaf'):.0f},"
              f" adults {med(rows,'adults'):.0f}, peak {med(rows,'peak'):.0f})")
        print(df.T.to_string())

# ---------------------------------------------------------------------------------------------------------------
section("S4  What a colony pays (first 300,000 ticks): per million adult-ticks unless said")
FIN = 300000


def colony_row(d):
    c, p = census(d), probe(d)
    at = adult_mticks(p, FIN)
    r = c.loc[FIN]
    L = pd.read_csv(f"{d}/ledger.csv")
    L = L[L.frame == FIN]
    g = L[~L.key.str.startswith("diet:")].groupby("key").value.sum()
    intake = L[L.key.str.startswith("diet:")].value.sum()
    rr = c[(c.index >= 84000) & (c.index < 250000)]
    tot = rr.ants_under + rr.ants_mound_in + rr.ants_mound_top + rr.ants_afield
    keep = tot >= 10
    return dict(eggs=r.eggs_laid / at, hatched=r.births / at, egg_fail_pct=100 * r.larvae_starved / max(r.eggs_laid, 1),
                starved=r.starved / at, oldage=r.oldage_deaths / at, intake_J_per_adult_tick=intake / (at * 1e6),
                metabolized=g.get("Metabolized", 0) / (at * 1e6), shared=g.get("SharedOut", 0) / (at * 1e6),
                afield_pct=100 * float((rr.ants_afield[keep] / tot[keep]).mean()) if keep.any() else np.nan,
                hungry_out_kpulls=r.hungry_out_pulls / at / 1000, seeds_carried=r.seeds_carried / at,
                returns_pct=100 * r.forage_returns / max(r.forage_trips, 1), digs=r.digs / at)


tab = {}
for arm in ("ants", "noleaf", "noseed", "noleafseed", "leaf20", "leaf160", "doortree"):
    rows = [colony_row(run_dir(s, arm)) for s in seeds_of(arm) if 300000 in census(run_dir(s, arm)).index]
    if rows:
        tab[arm] = {k: med(rows, k) for k in rows[0]}
        tab[arm]["n"] = len(rows)
print(pd.DataFrame(tab).round(3).to_string())

# ---------------------------------------------------------------------------------------------------------------
section("S4  Diet (as played, whole run): share of energy and of bites, median over seeds")
cal, bit = [], []
for s in seeds_of("ants"):
    d = run_dir(s, "ants")
    L = pd.read_csv(f"{d}/ledger.csv")
    L = L[L.frame == L.frame.max()]
    dd = L[L.key.str.startswith("diet:")].groupby("key").value.sum()
    tot = dd.sum()
    gv = lambda k: float(dd.get(k, 0))
    cal.append(dict(leaf=gv("diet:leaf") + gv("diet:grassblade"), litter=gv("diet:litter") + gv("diet:deadleaf"),
                    crumbs=gv("diet:crumbs"), seed=gv("diet:seed") + gv("diet:pip")))
    cal[-1] = {k: 100 * v / tot for k, v in cal[-1].items()}
    b = pd.read_csv(f"{d}/bites.csv")
    bs = b.groupby("material").bites.sum()
    n = bs.sum()
    bit.append(dict(leaf=100 * (bs.get("leaf", 0) + bs.get("grassblade", 0)) / n,
                    litter=100 * (bs.get("litter", 0) + bs.get("deadleaf", 0)) / n,
                    crumbs=100 * bs.get("crumbs", 0) / n, seed=100 * (bs.get("seed", 0) + bs.get("pip", 0)) / n))
if cal:
    # medians per category, then renormalised to 100 (the shares of the medians, as diet.png draws them)
    for name, rows in (("energy %", cal), ("bites  %", bit)):
        m = pd.DataFrame(rows).median()
        m = (m / m.sum() * 100)
        print(f"{name}:", m.round(1).to_dict(), "(other = the remainder, ~1%)")

# ---------------------------------------------------------------------------------------------------------------
section("S3  Established plants within 64 columns of a nest at 250,000 (summed over seeds)")
for arm in ("alone", "ants", "noleaf", "doortree"):
    herbs = trees = 0
    for s in seeds_of(arm):
        fp = f"{run_dir(s, arm)}/plants_250000.csv"
        if not os.path.exists(fp):
            continue
        d = pd.read_csv(fp)
        d = d[d.cells >= 50]
        near = d[np.minimum((d.cx - 728).abs(), (d.cx - 284).abs()) <= 64]
        herbs += int((near.species == "herb").sum())
        trees += int(near.species.isin(["tree", "conifer"]).sum())
    print(f"{arm:9s} herbs {herbs:4d}   trees+conifers {trees:3d}")

# ---------------------------------------------------------------------------------------------------------------
section("S5  Dug space closed by plant tissue, worst census row (dug_roots + dug_plant over dug, rows with dug >= 200)")
pt = pd.read_csv(f"{PLAY}/census.csv").set_index("frame")


def worst(c):
    c = c[c.dug >= 200]
    return 100 * float(((c.dug_roots + c.dug_plant) / c.dug).max())


print(f"playtest: {worst(pt):.1f}%  (dug {int(pt.dug.max())}, dug_roots max {int(pt.dug_roots.max())},"
      f" dug_plant max {int(pt.dug_plant.max())})")
for arm in ("ants", "noleaf", "doortree"):
    v = [worst(census(run_dir(s, arm))) for s in seeds_of(arm)]
    if v:
        print(f"{arm:9s} per seed {np.round(v, 1).tolist()}  median {np.median(v):.1f}%")

# ---------------------------------------------------------------------------------------------------------------
section("S5  Wood share of dig attempts at solid ground, per colony (arms with digs.csv)")
SOLID, WOOD = {"soil", "packedsoil", "spoil"}, {"wood", "rootwood", "deadwood", "log"}
for arm in ("ants", "doortree"):
    shares = []
    for s in seeds_of(arm):
        fp = f"{run_dir(s, arm)}/digs.csv"
        if not os.path.exists(fp):
            continue
        d = pd.read_csv(fp)
        for c in (1, 2):
            g = d[(d.colony == c) & d.why.isin([3, 4, 5, 6, 7])]
            sol = g[g.material.isin(SOLID | WOOD)]["count"].sum()
            if sol:
                shares.append((s, c, round(float(100 * g[g.material.isin(WOOD)]["count"].sum() / sol), 1)))
    if shares:
        print(f"{arm:9s} (seed, colony, %):", shares, f" median {np.median([x[2] for x in shares]):.1f}")

# ---------------------------------------------------------------------------------------------------------------
section("S5  Door trees: peak adults and adults at 250k per colony, same seed and site (ants.csv, every 2,000 ticks)")


def per_colony(d):
    a = pd.read_csv(f"{d}/ants.csv")
    return a.groupby(["frame", "colony"]).size().unstack(fill_value=0)


for s in seeds_of("doortree"):
    if not run_dir(s, "ants"):
        continue
    b, t = per_colony(run_dir(s, "ants")), per_colony(run_dir(s, "doortree"))
    at250 = lambda x, c: int(x.loc[x.index[np.argmin(np.abs(x.index - 250000))], c]) if c in x.columns else 0
    for c in (1, 2):
        print(f"seed {s} colony {c}: peak {int(b[c].max())} -> {int(t[c].max())};  at 250k {at250(b, c)} -> {at250(t, c)}")

# ---------------------------------------------------------------------------------------------------------------
section("S6  The no-ant stand over a long run (plants / seed bank / leaf cells)")
for tag in ("alone_long", "rain_light_alone_long"):
    for s in range(1, 9):
        d = f"{RUNS}/s{s}_{tag}"
        if os.path.isdir(d) and "replay done" in open(d + ".log").read():
            p = probe(d)
            fr = [f for f in (100000, 400000, 700000, 1000000, 1300000, 1500000) if f in p.index]
            if p.index.max() >= 1000000:
                print(f"{tag} seed {s}: frames {fr}\n   plants {[int(p.loc[f,'plants']) for f in fr]}"
                      f"\n   bank   {[int(p.loc[f,'bank']) for f in fr]}\n   leaf   {[int(p.loc[f,'leaf_cells']) for f in fr]}")
print("playtest plants at 100k/500k/1M/1.5M/2M:", [int(pt.loc[f, "plants"]) for f in (100000, 500000, 1000000, 1500000, 2000000)])

# ---------------------------------------------------------------------------------------------------------------
section("S3a  A fixed defence (arms s<seed>_def<d>, the scratch hook of the README's section 9), medians at 200-220k")
DF = 220000


def def_row(d):
    c, p = census(d), probe(d)
    at = adult_mticks(p, DF)
    r = c.loc[DF]
    L = pd.read_csv(f"{d}/ledger.csv")
    return dict(plants80=p.loc[80000, "plants"], litter80=p.loc[80000, "litter_free"], plants200=p.loc[200000, "plants"],
                low200=p.loc[200000, "leaf_h0_3"] + p.loc[200000, "leaf_h4_15"], adults200=p.loc[200000, "adults"],
                peak=p["adults"].max(), eggs=r.eggs_laid / at, egg_fail_pct=100 * r.larvae_starved / max(r.eggs_laid, 1),
                starved=r.starved / at,
                intake_kJ=L[(L.frame == DF) & L.key.str.startswith("diet:")].value.sum() / 1000)


dtab = {}
for arm in ("ants", "def0.2", "def0.35", "def0.5", "def0.9", "noleaf"):
    rows = [def_row(run_dir(s, arm)) for s in seeds_of(arm) if s <= 3 and DF in census(run_dir(s, arm)).index]  # seeds 1-3, as the defence arms
    if rows:
        dtab[arm] = {k: med(rows, k) for k in rows[0]}
        dtab[arm]["n"] = len(rows)
if dtab:
    print(pd.DataFrame(dtab).round(1).to_string())

# ---------------------------------------------------------------------------------------------------------------
section("S6  The shipped light rain against rain off (plants / low foliage at 250k; seeds with both twins)")
for s in range(1, 9):
    for lab, arm in (("rain off, alone", "alone"), ("rain light, alone", "rain_light_alone"),
                     ("rain off, as played", "ants"), ("rain light, as played", "rain_light")):
        d = run_dir(s, arm)
        if d and 250000 in probe(d).index:
            p = probe(d)
            print(f"seed {s} {lab:22s} plants {int(p.loc[250000,'plants']):4d}  low {int(p.loc[250000,'leaf_h0_3'] + p.loc[250000,'leaf_h4_15']):5d}"
                  f"  leaf {int(p.loc[250000,'leaf_cells']):6d}")

# ---------------------------------------------------------------------------------------------------------------
section("S3  Plant defence of established plants, mutation ON (mean per seed), ants vs the no-ant twin")
for F in (150000, 250000, 300000):
    for arm in ("mut_on_alone", "mut_on"):
        v = []
        for s in seeds_of(arm):
            fp = f"{run_dir(s, arm)}/plants_{F}.csv"
            if os.path.exists(fp):
                d = pd.read_csv(fp)
                d = d[d.cells >= 50]
                if len(d):
                    v.append(d.defence.mean())
        if v:
            print(f"{F} {arm:13s} per seed {np.round(v, 3).tolist()} median {np.median(v):.3f}")

# ---------------------------------------------------------------------------------------------------------------
section("S1  The playtest's own census: headline numbers")
r = pt.iloc[-1]
print(f"peak adults {int(pt.ants.max())} at {pt.ants.idxmax()}; last frame with adults {pt[pt.ants > 0].index.max()};"
      f" peak larvae {int(pt.brood_larvae.max())} at {pt.brood_larvae.idxmax()}")
print(f"eggs {int(r.eggs_laid)}, hatched {int(r.births)}, larvae starved {int(r.larvae_starved)}"
      f" ({100*r.larvae_starved/r.eggs_laid:.0f}% of eggs); deaths {int(r.deaths)} = starved {int(r.starved)}"
      f" + old age {int(r.oldage_deaths)}")
print(f"forage trips {int(r.forage_trips)}, returns {int(r.forage_returns)} ({100*r.forage_returns/r.forage_trips:.1f}%)")
print(f"seeds carried {int(r.seeds_carried)}: set on nest {int(r.pips_set_on_nest)}, on soil {int(r.pips_set_on_soil)},"
      f" lost {int(r.seeds_lost_no_room)}")
print(f"starvers by place (under, near, afield): {int(r.starved_under)}, {int(r.starved_near)}, {int(r.starved_afield)}")
print("plants at 10k/30k/80k/90k/250k/370k/1.3M:", [int(pt.loc[f, "plants"]) for f in (10000, 30000, 80000, 90000, 250000, 370000, 1300000)])
