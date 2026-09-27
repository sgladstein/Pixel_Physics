#!/usr/bin/env python3
"""Read `digbox`'s nest scoreboard over many runs: is the colony building a
nest, or digging no better than random?

`digbox` prints, at every stop, a `SCORE ... colony` line (the excavation's
own shape metrics), one `SCORE ... null=<name>` line per null model (the
colony's percentile among that null's draws: 0.5 = like random digging, 1.0 =
more nest-like than every draw) and a `SPEC` line (the owner's 2026-09-20
geometry, absolute). This script pools them by arm, stop and null, and reads
the order statistics a chaotic system needs -- the median over seeds, the
WORST seed, and how many seeds clear 0.9 -- never a pooled mean.

It reads the NEST FUNNEL the same way (`FUNNEL` and `LEDGER` lines, or
`--funnel` for those alone): the share of ants that ever reached each stage
of nest work, the share of cuts that built something, and where every cut and
every pellet went -- each a per-seed ratio, medianed over seeds. The ledger's
two can't-place counts are printed as the worst seed's share, because they
are the instrument's own error bar.

Logs are named `<arm>-s<seed>.log` (the pattern every nest-lane run script
writes). Before any table it prints the key's cardinality against what was
found, because a parse keyed on fewer dimensions than the run swept pools
them silently (CLAUDE.md: "A parse is a measurement").

    python3 scripts/nestscore.py DIR [DIR ...] [--stop 12000] [--arms default,mouth2]
    python3 scripts/nestscore.py --selftest
"""
import argparse
import glob
import os
import re
import statistics
import sys

METRICS = ["mouths", "reach", "largest", "roofed", "depth90", "iqr", "wide"]
NULLS = ["uniform", "rows", "eden", "walkers"]
NAME = re.compile(r"^(?P<arm>.+)-s(?P<seed>\d+)\.log$")
SCORE = re.compile(r"^SCORE frame=(?P<frame>\d+) n=(?P<n>\d+) (?P<rest>.*)$")
SPEC = re.compile(r"^SPEC frame=(?P<frame>\d+) (?P<rest>.*?)(\s+--.*)?$")
KV = re.compile(r"(\w+)=([^\s]+)")
FUNNEL_HEAD = re.compile(r"^FUNNEL frame=(?P<frame>\d+) ants=(?P<ants>\d+)")
FUNNEL_STAGE = re.compile(r"^FUNNEL   (?P<name>.+?)\s+(?P<n>\d+)  of prev\s+[\d.]+%  of all\s+[\d.]+%$")
FUNNEL_BUILT = re.compile(r"^FUNNEL   cuts that built (?P<built>\d+) of (?P<placed>\d+) placed cuts")
LEDGER_DIG = re.compile(
    r"^LEDGER frame=(?P<frame>\d+) cuts (?P<cuts>\d+) \+ target mismatch (?P<mismatch>\d+) = engine digs (?P<digs>\d+): "
    r"above the old surface (?P<above>\d+), a pellet or refill cut again (?P<again>\d+), new ground open to the sky (?P<open>\d+), "
    r"new ground under a roof (?P<roofed>\d+)"
    r"(?:.*?the cells cut again were spoil (?P<a_spoil>\d+), soil (?P<a_soil>\d+), lining (?P<a_lining>\d+), other (?P<a_other>\d+))?"
)
LEDGER_REFILL = re.compile(
    r"^LEDGER frame=(?P<frame>\d+) dug cells refilled: by a pellet (?P<pellet>\d+), fell in (?P<fell>\d+) "
    r"\(spoil (?P<spoil>\d+), soil (?P<soil>\d+), other (?P<other>\d+); from the cell above (?P<above>\d+), from the side (?P<side>\d+)\)"
)
LEDGER_PUT = re.compile(
    r"^LEDGER frame=(?P<frame>\d+) pellets put down (?P<put>\d+) \+ died holding (?P<died>\d+) \+ site not found (?P<lost>\d+) = "
    r"engine spoil_dumped (?P<dumped>\d+) \+ spoil_lost (?P<slost>\d+): beside the head (?P<beside>\d+), posted up the column (?P<lifted>\d+); "
    r"landed above the old surface (?P<out>\d+), below it (?P<below>\d+) \(into a dug cell (?P<refill>\d+)\)"
)


def parse(path):
    """One log -> {frame: {'n':, 'colony': {...}, 'null': {name: {...}}, 'spec': {...}, 'funnel': {...}}}."""
    out = {}
    funnel = None  # the FUNNEL block being read: stage lines carry no frame of their own
    with open(path) as f:
        for line in f:
            line = line.rstrip("\n")
            m = FUNNEL_HEAD.match(line)
            if m:
                fr = int(m["frame"])
                d = out.setdefault(fr, {"n": 0, "colony": None, "null": {}, "spec": None})
                funnel = d["funnel"] = {"ants": int(m["ants"]), "stages": [], "built": None, "dig": None, "put": None}
                continue
            m = FUNNEL_STAGE.match(line)
            if m and funnel is not None:
                funnel["stages"].append((m["name"], int(m["n"])))
                continue
            m = FUNNEL_BUILT.match(line)
            if m and funnel is not None:
                funnel["built"] = (int(m["built"]), int(m["placed"]))
                continue
            m = LEDGER_DIG.match(line)
            if m:
                out.setdefault(int(m["frame"]), {"n": 0, "colony": None, "null": {}, "spec": None}).setdefault("funnel", {})["dig"] = {k: int(v) for k, v in m.groupdict().items() if v is not None}
                continue
            m = LEDGER_REFILL.match(line)
            if m:
                out.setdefault(int(m["frame"]), {"n": 0, "colony": None, "null": {}, "spec": None}).setdefault("funnel", {})["refill"] = {k: int(v) for k, v in m.groupdict().items()}
                continue
            m = LEDGER_PUT.match(line)
            if m:
                out.setdefault(int(m["frame"]), {"n": 0, "colony": None, "null": {}, "spec": None}).setdefault("funnel", {})["put"] = {k: int(v) for k, v in m.groupdict().items()}
                continue
            m = SCORE.match(line)
            if m:
                fr = int(m["frame"])
                d = out.setdefault(fr, {"n": int(m["n"]), "colony": None, "null": {}, "spec": None})
                # digbox prints the FUNNEL block before the SCORE lines, and
                # that block has already made this frame's entry with n=0.
                d["n"] = int(m["n"])
                kv = dict(KV.findall(m["rest"]))
                if m["rest"].startswith("colony"):
                    d["colony"] = {k: float(v) for k, v in kv.items() if k in METRICS}
                elif "null" in kv:
                    d["null"][kv["null"]] = {
                        **{k: float(kv[k]) for k in METRICS if k in kv},
                        "median": float(kv.get("median", "nan")),
                        "ties": [] if kv.get("ties", "-") == "-" else kv["ties"].split(","),
                    }
                continue
            m = SPEC.match(line)
            if m:
                fr = int(m["frame"])
                kv = dict(KV.findall(m["rest"]))
                out.setdefault(fr, {"n": 0, "colony": None, "null": {}, "spec": None})["spec"] = {k: float(v) for k, v in kv.items()}
    return out


def load(dirs, arms_wanted):
    runs = {}
    for d in dirs:
        for path in sorted(glob.glob(os.path.join(d, "*.log"))):
            m = NAME.match(os.path.basename(path))
            if not m:
                continue
            arm, seed = m["arm"], int(m["seed"])
            if arms_wanted and arm not in arms_wanted:
                continue
            key = (arm, seed)
            if key in runs:
                sys.exit(f"nestscore: two logs for arm {arm} seed {seed} ({path}); key the directories apart")
            runs[key] = parse(path)
    return runs


def fmt(x):
    return "  -  " if x is None or x != x else f"{x:5.2f}"


def report(runs, stop):
    arms = sorted({a for a, _ in runs})
    seeds = {a: sorted(s for b, s in runs if b == a) for a in arms}
    frames = sorted({fr for r in runs.values() for fr in r})
    print(f"nestscore: {len(runs)} runs keyed (arm, seed): arms {arms}; seeds per arm {[len(seeds[a]) for a in arms]}; stops seen {frames}")
    unscored = [(a, s) for (a, s), r in runs.items() if not r or all(v["colony"] is None for v in r.values())]
    if unscored and not any(v.get("funnel") for r in runs.values() for v in r.values()):
        print(f"  WARNING: {len(unscored)} runs carry no SCORE lines (built before the scoreboard, or nulls=0): {unscored[:6]}")
    stops = [stop] if stop else frames
    for fr in stops:
        print(f"\n=== frame {fr} ===")
        for a in arms:
            rows = [runs[(a, s)].get(fr) for s in seeds[a]]
            rows = [r for r in rows if r and r["colony"] is not None]
            if not rows:
                continue
            n = statistics.median(r["n"] for r in rows)
            col = {k: statistics.median(r["colony"][k] for r in rows) for k in METRICS}
            print(f"\n{a}: {len(rows)} seeds, dug cells median {n:.0f}")
            print("  the excavation (median):  " + "  ".join(f"{k} {col[k]:g}" for k in METRICS))
            spec = [r["spec"] for r in rows if r["spec"]]
            if spec:
                ch = [s.get("chambers", 0) for s in spec]
                print(f"  spec: 1 mouth on {sum(1 for s in spec if s.get('mouths') == 1)} of {len(spec)} seeds; "
                      f"chambers on {sum(1 for c in ch if c > 0)}; passage median {statistics.median(s.get('passage', 0) for s in spec):g}; "
                      f"mouths median {statistics.median(s.get('mouths', 0) for s in spec):g}")
            print("  percentile among the null's draws (median over seeds / worst seed; 0.5 = like random digging):")
            print("  " + " " * 9 + "".join(f"{k:>14}" for k in METRICS) + f"{'panel':>14}  seeds>=0.9")
            for nm in NULLS:
                cells = []
                for k in METRICS:
                    vals = [r["null"][nm][k] for r in rows if nm in r["null"] and k in r["null"][nm]]
                    tied = sum(1 for r in rows if nm in r["null"] and k in r["null"][nm]["ties"])
                    if not vals:
                        cells.append(f"{'':>14}")
                    elif tied == len(vals):
                        cells.append(f"{'tie':>14}")
                    else:
                        cells.append(f"{fmt(statistics.median(vals)):>7}/{fmt(min(vals)):<6}")
                med = [r["null"][nm]["median"] for r in rows if nm in r["null"]]
                hi = sum(1 for v in med if v >= 0.9)
                panel = f"{fmt(statistics.median(med))}/{fmt(min(med))}" if med else ""
                print(f"  {nm:>9}" + "".join(cells) + f"{panel:>14}  {hi} of {len(med)}")


def med_worst(vals, worst=min):
    return f"{fmt(statistics.median(vals))}/{fmt(worst(vals))}" if vals else "  -  "


def funnel_report(runs, stop):
    """The nest funnel by arm: every figure a per-seed ratio, medianed over seeds."""
    arms = sorted({a for a, _ in runs})
    frames = sorted({fr for r in runs.values() for fr, d in r.items() if d.get("funnel", {}).get("dig")})
    if not frames:
        return
    for fr in [stop] if stop else frames:
        print(f"\n=== nest funnel, frame {fr}: median over seeds / worst seed ===")
        for a in arms:
            fs = [r[fr]["funnel"] for (b, _), r in runs.items() if b == a and fr in r and r[fr].get("funnel", {}).get("dig") and r[fr]["funnel"].get("stages")]
            if not fs:
                continue
            print(f"\n{a}: {len(fs)} seeds, ants median {statistics.median(f['ants'] for f in fs):g}")
            names = [nm for nm, _ in fs[0]["stages"]]
            for i, nm in enumerate(names):
                share = [f["stages"][i][1] / f["ants"] for f in fs if f["ants"] and len(f["stages"]) > i]
                print(f"  {nm:<62} of all ants {med_worst(share)}")
            built = [f["built"][0] / f["built"][1] for f in fs if f.get("built") and f["built"][1]]
            print(f"  cuts that built (roofed new ground, pellet out, open {'1,500'} frames on): {med_worst(built)} of placed cuts")
            dig = [f["dig"] for f in fs]
            cut = lambda d: max(d["cuts"], 1)
            print(
                "  where the cuts went (share of placed cuts):  "
                + "  ".join(f"{k} {med_worst([d[k] / cut(d) for d in dig], max)}" for k in ("above", "again", "open", "roofed"))
                + "   [above = in the heaps above the old surface; again = a cell already dug or filled]"
            )
            split = [d for d in dig if "a_lining" in d and d["again"]]
            if split:
                print(
                    "  what the re-cut cells were made of (share of re-cuts):  "
                    + "  ".join(f"{k[2:]} {med_worst([d[k] / d['again'] for d in split], max)}" for k in ("a_spoil", "a_soil", "a_lining", "a_other"))
                )
            put = [f["put"] for f in fs if f.get("put")]
            if put:
                pp = lambda d: max(d["put"], 1)
                print(
                    "  where the pellets went (share of placed pellets):  "
                    + "  ".join(f"{k} {med_worst([d[k] / pp(d) for d in put], max)}" for k in ("beside", "lifted", "out", "below", "refill"))
                )
            ref = [f["refill"] for f in fs if f.get("refill")]
            if ref:
                tot = lambda d: max(d["pellet"] + d["fell"], 1)
                fell = lambda d: max(d["fell"], 1)
                print(
                    f"  where a refilled hole's fill came from (share of refills):  a pellet {med_worst([d['pellet'] / tot(d) for d in ref], max)}  fell in {med_worst([d['fell'] / tot(d) for d in ref], max)};"
                    f"  of the falls: soil {med_worst([d['soil'] / fell(d) for d in ref], max)}  spoil {med_worst([d['spoil'] / fell(d) for d in ref], max)}  from above {med_worst([d['above'] / fell(d) for d in ref], max)}"
                )
            mism = [d["mismatch"] / max(d["digs"], 1) for d in dig]
            lost = [d["lost"] / max(d["dumped"] + d["slost"], 1) for d in put] if put else []
            print(f"  the instrument's own error: digs it could not place {med_worst(mism, max)}; pellets it could not place {med_worst(lost, max)}")


def paired(runs, base, stop):
    """Colony metrics, arm against `base`, paired by seed: how many seeds up/down."""
    arms = sorted({a for a, _ in runs} - {base})
    print(f"\n=== paired against {base}, frame {stop}: seeds where the arm's excavation is more nest-like / less ===")
    direction = {"mouths": -1, "reach": 1, "largest": 1, "roofed": 1, "depth90": 1, "iqr": -1, "wide": 1}
    for a in arms:
        common = [s for (b, s) in runs if b == a and (base, s) in runs]
        parts = []
        for k in METRICS:
            up = dn = 0
            for s in common:
                x, y = runs[(a, s)].get(stop), runs[(base, s)].get(stop)
                if not x or not y or not x["colony"] or not y["colony"]:
                    continue
                vx, vy = x["colony"][k], y["colony"][k]
                if k == "mouths":
                    vx, vy = abs(vx - 1), abs(vy - 1)
                d = (vx - vy) * direction[k]
                up += d > 0
                dn += d < 0
            parts.append(f"{k} {up}/{dn}")
        print(f"  {a:>12}: " + ", ".join(parts))


def selftest():
    import tempfile
    d = tempfile.mkdtemp()
    score = (
        "SCORE frame=100 n=50 colony mouths=3.000 reach=0.500 largest=0.400 roofed=0.300 depth90=5.000 iqr=20.000 wide=0.100\n"
        + "".join(
            f"SCORE frame=100 n=50 null={nm} k=10 mouths=0.500 reach=0.900 largest=0.900 roofed=0.100 depth90=0.500 iqr=0.800 wide=0.500 median=0.800 ties=depth90\n"
            for nm in NULLS
        )
        + "SPEC frame=100 mouths=3 chambers=0 passage=2 contrast=0.0 widest=0   -- spec\n"
    )
    funnel = (
        "FUNNEL frame=100 ants=40  (booked at the furthest stage each ant ever reached)\n"
        + "FUNNEL   lived                                                          40  of prev 100.0%  of all 100.0%\n"
        + "FUNNEL   cut a cell                                                     30  of prev  75.0%  of all  75.0%\n"
        + "FUNNEL   complete cycles per ant: 0: 36  1: 1  2: 2  3+: 1   (cuts still waiting on the lasting check: 32)\n"
        + "FUNNEL   cuts that built 9 of 400 placed cuts (2.2%); cuts per ant: median 9 max 28\n"
        + "LEDGER frame=100 cuts 400 + target mismatch 6 = engine digs 406: above the old surface 160, a pellet or refill cut again 112, new ground open to the sky 70, new ground under a roof 58 (of the new ground, tunnel lining 99; placed by elimination 3); mismatch: ahead refilled 1, ahead not ground 5; the cells cut again were spoil 6, soil 40, lining 66, other 0\n"
        + "LEDGER frame=100 pellets put down 397 + died holding 0 + site not found 2 = engine spoil_dumped 399 + spoil_lost 0: beside the head 223, posted up the column 174; landed above the old surface 374, below it 23 (into a dug cell 23)\n"
        + "LEDGER frame=100 dug cells refilled: by a pellet 23, fell in 377 (spoil 1, soil 375, other 1; from the cell above 190, from the side 187)\n"
    )
    # digbox's own order at a stop: the funnel block, then the scoreboard.
    body = funnel + score
    for arm in ("a", "b"):
        for s in (1, 2):
            with open(os.path.join(d, f"{arm}-s{s}.log"), "w") as f:
                f.write(body)
    runs = load([d], None)
    assert len(runs) == 4, f"expected 4 keyed runs, got {len(runs)}"
    r = runs[("a", 1)][100]
    assert r["n"] == 50, f"dug cells read {r['n']}, not 50: the FUNNEL block ahead of SCORE must not pin n at 0"
    assert r["colony"]["iqr"] == 20.0 and r["null"]["eden"]["median"] == 0.8 and r["null"]["rows"]["ties"] == ["depth90"], r
    assert r["spec"]["chambers"] == 0 and r["spec"]["mouths"] == 3, r["spec"]
    fu = r["funnel"]
    assert fu["ants"] == 40 and fu["stages"] == [("lived", 40), ("cut a cell", 30)], fu["stages"]
    assert fu["built"] == (9, 400) and fu["dig"]["above"] == 160 and fu["dig"]["digs"] == 406, fu
    assert (fu["dig"]["a_spoil"], fu["dig"]["a_soil"], fu["dig"]["a_lining"]) == (6, 40, 66), fu["dig"]
    assert fu["refill"]["fell"] == 377 and fu["refill"]["soil"] == 375 and fu["refill"]["above"] == 190, fu.get("refill")
    assert fu["put"]["beside"] == 223 and fu["put"]["refill"] == 23 and fu["put"]["dumped"] == 399, fu["put"]
    # A duplicate key must refuse, not pool (last write wins is the failure).
    try:
        load([d, d], None)
        raise AssertionError("a duplicate (arm, seed) was pooled silently")
    except SystemExit:
        pass
    print("nestscore selftest: PASS -- parses colony, null, SPEC, FUNNEL and LEDGER lines, and refuses a duplicate key")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("dirs", nargs="*")
    ap.add_argument("--stop", type=int, default=None, help="one stop only (default: every stop)")
    ap.add_argument("--arms", default=None, help="comma-separated arms to include")
    ap.add_argument("--base", default=None, help="an arm to pair every other arm against, by seed")
    ap.add_argument("--funnel", action="store_true", help="the nest funnel only, not the scoreboard")
    ap.add_argument("--selftest", action="store_true")
    a = ap.parse_args()
    if a.selftest:
        selftest()
        return
    if not a.dirs:
        ap.error("give at least one directory of <arm>-s<seed>.log files")
    runs = load(a.dirs, set(a.arms.split(",")) if a.arms else None)
    if not runs:
        sys.exit("nestscore: no <arm>-s<seed>.log files found")
    if not a.funnel:
        report(runs, a.stop)
    funnel_report(runs, a.stop)
    if a.base:
        stop = a.stop or max(fr for r in runs.values() for fr in r)
        paired(runs, a.base, stop)


if __name__ == "__main__":
    main()
