"""Judge brains on the reserved test worlds, the way the brain lane does since 2026-10-21.

usage: python3 -I judge.py S91_FETCH S91_TRAIL NAME:SOIL:FETCH:TRAIL [NAME:SOIL:FETCH:TRAIL ...]

Each file is the output of `examples/train.rs ... test=96 score=<genome> noise=1` (soil: `challenge=soil step=4
shaft=8 foot=1 budget=2400 frames=12000`; fetch: `challenge=fetch dist=10,30 budget=1200`; trail: `challenge=trail`).
Prints full loads a world (soil) with a standard error per mirrored pair (worlds 2i and 2i+1 share a seed and often
run alike, so the pair is the unit), and fetch and trail as the worse side of each pair -- the trainer's own fold --
as a share of s91 (`training/starts/fretrain-s91.txt`) on the same worlds. The kept-skill floor is 0.8. The first
named brain is the baseline the others are compared with (unpaired; a brain that behaves differently decorrelates
from its source world by world, so pairing buys nothing)."""
import ast
import statistics as st
import sys


def rows(path, full_loads):
    out = []
    for line in open(path):
        if line.startswith("NOISE seed"):
            out.append(ast.literal_eval(line[line.index("["):])[24] if full_loads else float(line.split()[3]))
    return out


def per_pair(v):
    p = [(v[2 * i] + v[2 * i + 1]) / 2 for i in range(len(v) // 2)]
    return st.fmean(p), st.stdev(p) / len(p) ** 0.5


def worse_side(v):
    return st.fmean(min(v[2 * i], v[2 * i + 1]) for i in range(len(v) // 2))


s91_fetch, s91_trail = worse_side(rows(sys.argv[1], False)), worse_side(rows(sys.argv[2], False))
base = None
print(f"{'brain':12s} {'full loads':>15s} {'vs first':>15s} {'fetch':>6s} {'trail':>6s} worlds")
for spec in sys.argv[3:]:
    name, soil, fetch, trail = spec.split(":")
    v = rows(soil, True)
    m, se = per_pair(v)
    diff = "" if base is None else f"{m - base[0]:+7.2f} +- {(se ** 2 + base[1] ** 2) ** 0.5:4.2f}"
    base = base or (m, se)
    f = worse_side(rows(fetch, False)) / s91_fetch
    t = worse_side(rows(trail, False)) / s91_trail
    print(f"{name:12s} {m:7.2f} +- {se:4.2f} {diff:>15s} {f:6.2f} {t:6.2f} {len(v)}")
