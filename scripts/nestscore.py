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


def parse(path):
    """One log -> {frame: {'n':, 'colony': {...}, 'null': {name: {...}}, 'spec': {...}}}."""
    out = {}
    with open(path) as f:
        for line in f:
            m = SCORE.match(line)
            if m:
                fr = int(m["frame"])
                d = out.setdefault(fr, {"n": int(m["n"]), "colony": None, "null": {}, "spec": None})
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
    if unscored:
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
    body = (
        "SCORE frame=100 n=50 colony mouths=3.000 reach=0.500 largest=0.400 roofed=0.300 depth90=5.000 iqr=20.000 wide=0.100\n"
        + "".join(
            f"SCORE frame=100 n=50 null={nm} k=10 mouths=0.500 reach=0.900 largest=0.900 roofed=0.100 depth90=0.500 iqr=0.800 wide=0.500 median=0.800 ties=depth90\n"
            for nm in NULLS
        )
        + "SPEC frame=100 mouths=3 chambers=0 passage=2 contrast=0.0 widest=0   -- spec\n"
    )
    for arm in ("a", "b"):
        for s in (1, 2):
            with open(os.path.join(d, f"{arm}-s{s}.log"), "w") as f:
                f.write(body)
    runs = load([d], None)
    assert len(runs) == 4, f"expected 4 keyed runs, got {len(runs)}"
    r = runs[("a", 1)][100]
    assert r["colony"]["iqr"] == 20.0 and r["null"]["eden"]["median"] == 0.8 and r["null"]["rows"]["ties"] == ["depth90"], r
    assert r["spec"]["chambers"] == 0 and r["spec"]["mouths"] == 3, r["spec"]
    # A duplicate key must refuse, not pool (last write wins is the failure).
    try:
        load([d, d], None)
        raise AssertionError("a duplicate (arm, seed) was pooled silently")
    except SystemExit:
        pass
    print("nestscore selftest: PASS -- parses colony, null and SPEC lines, and refuses a duplicate key")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("dirs", nargs="*")
    ap.add_argument("--stop", type=int, default=None, help="one stop only (default: every stop)")
    ap.add_argument("--arms", default=None, help="comma-separated arms to include")
    ap.add_argument("--base", default=None, help="an arm to pair every other arm against, by seed")
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
    report(runs, a.stop)
    if a.base:
        stop = a.stop or max(fr for r in runs.values() for fr in r)
        paired(runs, a.base, stop)


if __name__ == "__main__":
    main()
