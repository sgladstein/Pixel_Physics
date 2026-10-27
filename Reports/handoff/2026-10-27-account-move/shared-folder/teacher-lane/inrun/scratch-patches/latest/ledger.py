"""SCRATCH (teacher lane, in-run selection pilot, 2026-10-21): the per-ant score from an `inrun` run directory.

usage: python3 -I ledger.py RUNDIR [RUNDIR...] [--k 2] [--w food=1,dig=1,clear=1] [--json OUT]

score_i = (1/T_i) * sum over ant i's events of  w_j * p_j(t) * s
  jobs: deliver (food home), dig (half on the cut, half when that cell leaves the nest), clear (carry a pellet out),
        eat (a debit, in delivered-cell units: worth eaten / mean worth of a field cell)
  p_j(t) = 1 + k * max(0, 1 - stock_j(t) / target_j), read at the event's frame:
        food: delivered worth / eaten worth over the last 5,000 frames, target 1
        dig : open nest cells below the datum per live ant (from the maps), target 25
        clear: pellets let go inside, net of re-cuts, per live ant, as a shortfall in reverse (more loose soil -> higher)
  T_i: thousands of frames alive (to death or the run's end)
Exchange rates w: default 1 each; `--w baseline` sets each job so the colony's total credit from it is equal on these runs.
"""
import sys, json, glob, os, re, bisect
import numpy as np


def load(run):
    ants = {}
    for ln in open(os.path.join(run, "ants.csv")).read().split("\n")[1:]:
        if ln:
            i, t = ln.split(",")
            ants[int(i)] = t
    ev = []
    for ln in open(os.path.join(run, "events.csv")).read().split("\n")[1:]:
        if ln:
            f, i, k, x, y, v, c = ln.split(",")
            ev.append((int(f), int(i), k, int(x), int(y), float(v), int(c)))
    surf = [int(s) for s in open(os.path.join(run, "surface.txt")).read().split()]
    maps = []
    for p in sorted(glob.glob(os.path.join(run, "map_f*.txt"))):
        fr = int(re.search(r"map_f(\d+)", p).group(1))
        L = open(p).read().split("\n")
        x0, y0, w, h = map(int, L[0].split())
        open_ = 0
        for yy, row in enumerate(L[1:1 + h]):
            y = y0 + yy
            for xx, ch in enumerate(row):
                if ch in ".a" and y >= surf[x0 + xx]:
                    open_ += 1
        alive = sum(1 for _ in open(os.path.join(run, f"ants_f{fr:07d}.txt"))) if os.path.exists(os.path.join(run, f"ants_f{fr:07d}.txt")) else None
        maps.append((fr, open_, alive))
    return ants, ev, surf, maps


def ledger(run, k=2.0, w=None):
    ants, ev, surf, maps = load(run)
    w = w or dict(deliver=1.0, dig=1.0, clear=1.0, eat=1.0)
    end = max(e[0] for e in ev) if ev else 0
    death = {e[1]: e[0] for e in ev if e[2] == "death"}
    cell_worth = np.mean([e[5] for e in ev if e[2] == "field"]) if any(e[2] == "field" for e in ev) else 1.0
    # stocks through the run
    mf = [m[0] for m in maps]
    base_open = maps[0][1] if maps else 0

    def dig_price(f):
        i = bisect.bisect_right(mf, f) - 1
        if i < 0:
            return 1 + k
        fr, op, alive = maps[i]
        per = (op - base_open) / max(alive or 1, 1)
        return 1 + k * max(0.0, 1 - per / 25.0)

    deliv = [(e[0], cell_worth) for e in ev if e[2] == "deliver"]
    eats = [(e[0], e[5]) for e in ev if e[2] == "eat"]
    df, dw = np.array([d[0] for d in deliv] or [0]), np.cumsum([d[1] for d in deliv] or [0])
    ef, ew = np.array([d[0] for d in eats] or [0]), np.cumsum([d[1] for d in eats] or [0])

    def window_sum(fr_arr, cum, f, span=5000):
        hi = bisect.bisect_right(fr_arr, f) - 1
        lo = bisect.bisect_right(fr_arr, f - span) - 1
        a = cum[hi] if hi >= 0 else 0.0
        b = cum[lo] if lo >= 0 else 0.0
        return a - b

    def food_price(f):
        got, ate = window_sum(df, dw, f), window_sum(ef, ew, f)
        bal = got / ate if ate > 0 else 1.0
        return 1 + k * max(0.0, 1 - bal)

    loose = 0
    loose_at = []
    for e in ev:
        if e[2] == "in":
            loose += 1
        elif e[2] == "recut" and e[6] != 0:
            loose -= 1
        loose_at.append(loose)

    credit = {i: dict(deliver=0.0, dig=0.0, clear=0.0, eat=0.0, n_deliver=0, n_dig=0, n_clear=0, n_pair=0) for i in ants}
    halves = {i: [0.0, 0.0] for i in ants}
    for idx, (f, i, kind, x, y, v, c) in enumerate(ev):
        if kind == "deliver" and i in credit:
            g = w["deliver"] * food_price(f)
            credit[i]["deliver"] += g; credit[i]["n_deliver"] += 1
        elif kind == "eat" and i in credit:
            g = -w["eat"] * food_price(f) * v / cell_worth
            credit[i]["eat"] += g
        elif kind == "dig" and i in credit:
            g = 0.5 * w["dig"] * dig_price(f)
            credit[i]["dig"] += g; credit[i]["n_dig"] += 1
        elif kind == "out":
            alive = maps[min(len(maps) - 1, max(0, bisect.bisect_right(mf, f) - 1))][2] if maps else 30
            pc = 1 + k * min(1.0, loose_at[idx] / max(2.0 * (alive or 30), 1))
            if i in credit:
                g = w["clear"] * pc
                credit[i]["clear"] += g; credit[i]["n_clear"] += 1
                born = 0
                half = 0 if f < (born + death.get(i, end)) / 2 else 1
                halves[i][half] += g
            if c and c in credit:
                g2 = 0.5 * w["dig"] * dig_price(f)
                credit[c]["dig"] += g2
                if c != i:
                    credit[c]["n_pair"] += 1
            continue
        else:
            continue
        if i in credit:
            half = 0 if f < death.get(i, end) / 2 else 1
            halves[i][half] += g
    rows = []
    for i, t in ants.items():
        T = death.get(i, end) / 1000.0
        cr = credit[i]
        tot = cr["deliver"] + cr["dig"] + cr["clear"] + cr["eat"]
        L = death.get(i, end)
        rows.append(dict(run=run, id=i, tag=t, life=L, score=tot / max(T, 0.1),
                         h1=halves[i][0] / max(L / 2000.0, 0.05), h2=halves[i][1] / max(L / 2000.0, 0.05), **cr))
    return rows


if __name__ == "__main__":
    args = sys.argv[1:]
    k = 2.0
    out = None
    runs = []
    i = 0
    while i < len(args):
        if args[i] == "--k":
            k = float(args[i + 1]); i += 2
        elif args[i] == "--json":
            out = args[i + 1]; i += 2
        else:
            runs.append(args[i]); i += 1
    rows = [r for run in runs for r in ledger(run, k)]
    if out:
        json.dump(rows, open(out, "w"))
    for r in rows[:5]:
        print(r)
