#!/usr/bin/env python3
"""Compare `examples/mindprobe.rs` arms against their `arm=none` twin, paired by seed.

    python3 scripts/mindprobe_cmp.py DIR        # DIR holds <arm>_h<hold>_s<seed>.txt and none_s<seed>.txt

For each arm file, every sampled quantity is differenced against the twin on the same seed at the same sample, and
summarised over two spans: `early` (the first 600 frames after the press starts) and `all` (the whole window). Prints
one row per (arm, hold): the median and range over seeds of each paired difference, and the end-of-window delivered
difference. A number in `none`'s own column is the twin's mean, for scale.
"""
import glob
import os
import re
import statistics
import sys

KEYS = ["alive", "near", "out", "laden", "speed", "alarm15", "lean", "zone_n", "zone_mean", "zone_sd", "adults"]


def read(path):
    rows = []
    for line in open(path):
        if line.startswith("S "):
            d = dict(kv.split("=") for kv in line.split()[1:])
            rows.append({k: float(v) for k, v in d.items()})
    return rows


def span_mean(rows, key, upto):
    v = [r[key] for r in rows if r["t"] <= upto and key in r]
    return sum(v) / len(v) if v else float("nan")


def main():
    d = sys.argv[1]  # optional argv[2]: comma-separated columns, e.g. lean:all,zone_sd:early,fed:end
    twins = {}
    for p in glob.glob(os.path.join(d, "none_s*.txt")):
        s = re.search(r"_s(\d+)\.txt$", p).group(1)
        twins[s] = read(p)
    groups = {}
    for p in sorted(glob.glob(os.path.join(d, "*_h*_s*.txt"))):
        m = re.search(r"([a-z]+)_h(\d+)_s(\d+)\.txt$", os.path.basename(p))
        arm, hold, seed = m.group(1), int(m.group(2)), m.group(3)
        if seed not in twins:
            continue
        rows, twin = read(p), twins[seed]
        if len(rows) != len(twin) or not rows:
            print(f"skip {p}: {len(rows)} rows vs twin {len(twin)}")
            continue
        out = {}
        for span, upto in (("early", 600), ("all", 1e12)):
            for k in KEYS:
                out[f"{k}:{span}"] = span_mean(rows, k, upto) - span_mean(twin, k, upto)
        out["delivered:end"] = rows[-1]["delivered"] - twin[-1]["delivered"]
        out["fed:end"] = rows[-1]["fed_adult"] - twin[-1]["fed_adult"]
        out["dug:end"] = rows[-1]["dug"] - twin[-1]["dug"]
        if "trip" in rows[-1] and "trip" in twin[-1]:
            out["trip:end"] = rows[-1]["trip"] - twin[-1]["trip"]
        groups.setdefault((arm, hold), []).append(out)
    print(f"seeds with a twin: {sorted(twins)}")
    base = {k: statistics.mean(span_mean(t, k, 1e12) for t in twins.values()) for k in KEYS}
    base["delivered"] = statistics.mean(t[-1]["delivered"] for t in twins.values())
    print("twin means: " + " ".join(f"{k}={v:.3f}" for k, v in base.items()))
    cols = (sys.argv[2].split(",") if len(sys.argv) > 2 else
            ["near:early", "near:all", "out:early", "speed:early", "alarm15:early", "alive:all", "delivered:end", "fed:end", "dug:end"])
    print("arm    hold  n  " + "  ".join(f"{c:>22}" for c in cols))
    for (arm, hold), outs in sorted(groups.items()):
        cells = []
        for c in cols:
            v = sorted(o[c] for o in outs)
            cells.append(f"{statistics.median(v):+8.3f} [{v[0]:+.2f},{v[-1]:+.2f}]")
        print(f"{arm:6} {hold:5} {len(outs):2}  " + "  ".join(f"{x:>22}" for x in cells))


if __name__ == "__main__":
    main()
