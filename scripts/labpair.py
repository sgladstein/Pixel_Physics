#!/usr/bin/env python3
"""The lab box, paired seed by seed: two arms of `labforage` logs compared on
net food into home and the colony's life, with a sign test.

Built 2026-09-27 for the foraging loop (`Reports/lanes/foraging-loop.md`).
**Net food into home is the lab's honest delivery measure**: `deliveries`
counts every drop at home, and 86% of the default's are food picked up at home
and put straight back (the nest-mouth lane), so a bigger or busier home scores
more "deliveries" moving nothing. Net = `deliveries - pickups_at_nest`.

    # one log per seed, named <arm>-<seed>.log, in one directory:
    labforage scenario=played_bed frames=120000 seed=N > dir/base-N.log
    python3 scripts/labpair.py dir base new
    python3 scripts/labpair.py --selftest

Reads each log's `SUMMARY seed=` line. Pairs only seeds present in both arms and
prints which they were, so a missing run is visible rather than silently
dropped.
"""
import glob
import math
import os
import re
import statistics as st
import sys
import tempfile

KEYS = ['net_home', 'deliveries', 'pickups_at_nest', 'intake', 'born', 'alive', 'bgen', 'starved']
HIGHER_IS_BETTER = {'starved': False}


def load(d, arm):
    out = {}
    for f in glob.glob(os.path.join(d, f"{arm}-*.log")):
        m = re.match(r'.*-(\d+)\.log$', f)
        if not m:
            continue
        s = re.search(r'SUMMARY seed=\d+ .*', open(f).read())
        if not s:
            continue
        row = {k: float(v) for k, v in re.findall(r' (\w+)=(-?[\d.]+)(?= |$)', s.group(0))}
        row['net_home'] = row.get('deliveries', 0) - row.get('pickups_at_nest', 0)
        dm = re.search(r'deaths_by=ant\[([^\]]*)\]', s.group(0))
        sv = re.search(r'STARVED:(\d+)', dm.group(1)) if dm else None
        row['starved'] = float(sv.group(1)) if sv else 0.0
        out[int(m.group(1))] = row
    return out


def sign_p(better, worse):
    n, k = better + worse, min(better, worse)
    return min(1.0, 2 * sum(math.comb(n, i) for i in range(k + 1)) / 2 ** n) if n else 1.0


def compare(a, b, base, new):
    seeds = sorted(set(a) & set(b))
    lines = [f"{base} -> {new}: {len(seeds)} paired seeds {seeds}"]
    res = {}
    for k in KEYS:
        x = [a[s].get(k, 0) for s in seeds]
        y = [b[s].get(k, 0) for s in seeds]
        up = sum(j > i for i, j in zip(x, y))
        dn = sum(j < i for i, j in zip(x, y))
        good, bad = (up, dn) if HIGHER_IS_BETTER.get(k, True) else (dn, up)
        res[k] = (up, dn)
        med = lambda v: st.median(v) if v else float('nan')
        lines.append(f"  {k:<16} median {med(x):>10.0f} -> {med(y):>10.0f}   better/worse {good}/{bad}  sign p {sign_p(up, dn):.3f}")
    ext = (sum(a[s].get('alive', 1) == 0 for s in seeds), sum(b[s].get('alive', 1) == 0 for s in seeds))
    lines.append(f"  extinct (alive=0): {ext[0]} -> {ext[1]}")
    return lines, res, ext


def selftest():
    d = tempfile.mkdtemp()
    def log(arm, seed, dl, pk, alive, starved):
        with open(os.path.join(d, f"{arm}-{seed}.log"), 'w') as fh:
            fh.write(f"noise\nSUMMARY seed={seed} deliveries={dl} pickups_at_nest={pk} intake=100 born=5 alive={alive} bgen=2 deaths_by=ant[STARVED:{starved}/OLD_AGE:1] moves=9\n")
    for s in (1, 2, 3):
        log('base', s, 100, 90, 5, 10)          # net 10
        log('new', s, 100, 70, 5 if s < 3 else 0, 8)   # net 30: more deliveries net, same raw
    log('base', 4, 100, 90, 5, 10)             # unpaired: must be excluded
    a, b = load(d, 'base'), load(d, 'new')
    _, res, ext = compare(a, b, 'base', 'new')
    checks = [
        (sorted(set(a) & set(b)) == [1, 2, 3], "seed 4 exists in one arm only and must not be paired"),
        (res['net_home'] == (3, 0), f"net food into home rose on all 3 seeds, read {res['net_home']}"),
        (res['deliveries'] == (0, 0), f"raw deliveries are identical, read {res['deliveries']}"),
        (res['starved'] == (0, 3), f"starved fell on all 3, read {res['starved']}"),
        (ext == (0, 1), f"one new-arm seed went extinct, read {ext}"),
    ]
    bad = [m for ok, m in checks if not ok]
    for m in bad:
        print("SELFTEST FAIL:", m)
    print("labpair selftest:", "FAILED" if bad else f"all {len(checks)} checks passed")
    return 1 if bad else 0


if __name__ == '__main__':
    if sys.argv[1:] == ['--selftest']:
        sys.exit(selftest())
    if len(sys.argv) != 4:
        sys.exit(__doc__)
    d, base, new = sys.argv[1:4]
    for line in compare(load(d, base), load(d, new), base, new)[0]:
        print(line)
