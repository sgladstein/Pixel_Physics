#!/usr/bin/env python3
"""The lab box's pre-ship regression check: two arms of `labforage` logs,
paired seed by seed, with a sign test.

    # one log per seed, named <arm>-<seed>.log, in one directory (24 seeds):
    labforage scenario=played_bed frames=120000 seed=N > dir/base-N.log
    python3 scripts/labpair.py dir base new
    python3 scripts/labpair.py --selftest

**The gate** (reworked 2026-09-29 from a test-bed review, owner-approved):
births; food eaten (`intake`); ant-frames lived (the 900-frame table's ants
times the sample interval, summed over its rows); died of old age; and
**starved per million ant-frames** -- per seed, starved / ant-frames, then
paired by seed -- with raw starved printed beside it. Raw starved scales with
how many ants there were to starve, so an arm that grows more colony starves
more ants in total without being worse for it; the rate divides that out.

**Crash timing, not a gate:** died out, under 10 at the end, alive at the end.
The box grazes itself out -- on the 2026-09-29 review the median box peaked
near frame 90,000 and 9 of 24 fell below a quarter of their peak, mostly after
frame 100,000 -- so an end-of-run count reads *when* the crash landed, not
harm. It is printed in its own block with the peak and the crash frame.

**Net food into home** (`deliveries - pickups_at_nest`) led this script until
2026-09-29 and is now last and labelled: it **overcounts 3.4-4.6x**
(`Reports/ant-scenes-2026-09-23.md` s22j) -- a crumb a cell or two from home is
picked up from outside with no debit and delivered again. Raw `deliveries` is
worse still: 86% of the default's are food picked up at home and put back.

A parse is a measurement. Logs are keyed on (arm, seed); a file counts for an
arm only if it is named exactly `<arm>-<digits>.log` (so `sl-ret-1.log` never
lands in arm `sl` and overwrites seed 1), and the seed in the name must agree
with the SUMMARY line's. It prints the key's cardinality, the seeds it paired,
logs with no SUMMARY yet (still running, or died), and the header settings
that differ between the arms or within one -- two configs inside one arm is
two experiments pooled.
"""
import glob
import math
import os
import re
import statistics as st
import sys
import tempfile

# A row of labforage's sampled table: frame, ants, plants, edible, worth, ...
# The file is ~750k lines of per-tick `frame ...` trace around ~134 of these.
TABLE_ROW = re.compile(rb'\s+(\d+)\s+(\d+)\s+\d+\s+\d+\s+\d+\s')
HEADER_KV = re.compile(rb'(\w+) ?= ?([^\s;]+)')


def read_log(path):
    """One log -> dict, or a dict with 'summary': None if the run never finished."""
    hdr, rows, summary, in_header = {}, [], None, True
    with open(path, 'rb') as fh:
        for line in fh:
            if line.startswith(b'frame'):          # per-tick trace: the bulk of the file
                continue
            if line.startswith(b'SUMMARY seed='):
                if summary is None:
                    summary = line.decode(errors='replace').strip()
                continue
            if summary is None:
                m = TABLE_ROW.match(line)
                if m:
                    rows.append((int(m.group(1)), int(m.group(2))))
                    in_header = False
                    continue
            if in_header:
                for k, v in HEADER_KV.findall(line):
                    if k != b'seed':
                        hdr[k.decode()] = v.decode(errors='replace')
    out = {'summary': summary, 'hdr': hdr}
    if summary is None:
        return out
    row = {k: float(v) for k, v in re.findall(r' (\w+)=(-?[\d.]+)(?= |$)', summary)}
    dm = re.search(r'deaths_by=ant\[([^\]]*)\]', summary) or re.search(r'deaths_by=\S*?\[([^\]]*)\]', summary)
    deaths = {k: float(v) for k, v in re.findall(r'([A-Z_]+):(\d+)', dm.group(1))} if dm else {}
    sample = int(hdr.get('sample', 900))
    frames = [f for f, _ in rows]
    peak = max((a for _, a in rows), default=0)
    peak_at = next((f for f, a in rows if a == peak), None) if peak else None
    crash_at = None
    if peak:
        after = False
        for f, a in rows:
            after = after or f == peak_at
            if after and a < peak / 4:
                crash_at = f
                break
    row.update(
        seed_summary=int(re.search(r'seed=(\d+)', summary).group(1)),
        starved=deaths.get('STARVED', 0.0),
        starved_aloft=deaths.get('STARVED_ALOFT', 0.0),
        killed=deaths.get('KILLED', 0.0),
        old_age=deaths.get('OLD_AGE', 0.0),
        net_home=row.get('deliveries', 0) - row.get('pickups_at_nest', 0),
        antframes=sum(a for _, a in rows) * sample,
        table_rows=len(rows),
        table_ok=all(f % sample == 0 for f in frames) and frames == sorted(set(frames)),
        peak=float(peak), peak_at=peak_at, crash_at=crash_at,
    )
    row['antframes_m'] = row['antframes'] / 1e6
    row['starve_rate'] = row['starved'] / row['antframes_m'] if row['antframes'] else None
    out['row'] = row
    return out


def load(d, arm):
    """Every `<arm>-<seed>.log` in d, keyed by seed, with what was left out and why."""
    name = re.compile(re.escape(arm) + r'-(\d+)\.log')
    rep = {'seeds': {}, 'files': 0, 'other_prefix': 0, 'unfinished': [], 'mismatch': [], 'dupes': [],
           'configs': {}}
    seen = {}
    for f in sorted(glob.glob(os.path.join(glob.escape(d), glob.escape(arm) + '-*.log'))):
        m = name.fullmatch(os.path.basename(f))
        if not m:
            rep['other_prefix'] += 1
            continue
        rep['files'] += 1
        seed = int(m.group(1))
        seen.setdefault(seed, []).append(f)
        r = read_log(f)
        cfg = tuple(sorted(r['hdr'].items()))
        rep['configs'].setdefault(cfg, []).append(seed)
        if r['summary'] is None:
            rep['unfinished'].append(seed)
            continue
        if r['row']['seed_summary'] != seed:
            rep['mismatch'].append((os.path.basename(f), r['row']['seed_summary']))
            continue
        rep['seeds'][seed] = r['row']
    rep['dupes'] = sorted(s for s, fs in seen.items() if len(fs) > 1)
    for s in rep['dupes']:                        # never last-write-wins: drop the seed
        rep['seeds'].pop(s, None)
    return rep


def sign_p(up, dn):
    n, k = up + dn, min(up, dn)
    return min(1.0, 2 * sum(math.comb(n, i) for i in range(k + 1)) / 2 ** n) if n else 1.0


def fmt(v):
    if v is None or (isinstance(v, float) and math.isnan(v)):
        return 'n/a'
    return f"{v:,.0f}" if abs(v) >= 100000 else f"{v:,.1f}"


def fmt_frame(v):
    return 'n/a' if v is None or (isinstance(v, float) and math.isnan(v)) else f"{v:,.0f}"


def med(v):
    return st.median(v) if v else float('nan')


def paired(A, B, seeds, k):
    s = [x for x in seeds if A[x].get(k) is not None and B[x].get(k) is not None]
    x = [A[i][k] for i in s]
    y = [B[i][k] for i in s]
    up = sum(j > i for i, j in zip(x, y))
    dn = sum(j < i for i, j in zip(x, y))
    return s, x, y, up, dn


GATE = [  # key, label, which way is harm (None: context, not gated)
    ('born', 'births', 'lower'),
    ('intake', 'food eaten, J (intake)', 'lower'),
    ('antframes_m', 'ant-frames lived, millions', 'lower'),
    ('old_age', 'died of old age', 'lower'),
    ('starve_rate', 'starved per million ant-frames', 'higher'),
    ('starved', '  starved, raw (scales with ant-frames)', None),
]
NOT_GATED = [
    ('net_home', 'net food into home (OVERCOUNT, s22j)'),
    ('deliveries', 'deliveries, raw'),
    ('pickups_at_nest', 'pickups at home'),
    ('bgen', 'deepest breeding generation'),
    ('starved_aloft', 'starved aloft (not in starved)'),
    ('killed', 'killed'),
]


def compare(ra, rb, base, new):
    A, B = ra['seeds'], rb['seeds']
    seeds = sorted(set(A) & set(B))
    res = {}
    L = [f"labpair: {base} -> {new}"]
    L.append(f"keyed on (arm, seed): {base} {ra['files']} logs -> {len(A)} seeds, "
             f"{new} {rb['files']} logs -> {len(B)} seeds; paired {len(seeds)}: {seeds}")
    for arm, r, other in ((base, ra, B), (new, rb, A)):
        extra = [f"unpaired {sorted(set(r['seeds']) - set(other))}"] if set(r['seeds']) - set(other) else []
        if r['unfinished']:
            extra.append(f"no SUMMARY (still running, or died) {sorted(r['unfinished'])}")
        if r['mismatch']:
            extra.append(f"name/SUMMARY seed disagree, dropped {r['mismatch']}")
        if r['dupes']:
            extra.append(f"two logs for one seed, dropped {r['dupes']}")
        if r['other_prefix']:
            extra.append(f"ignored {r['other_prefix']} files sharing the '{arm}-' prefix but not named {arm}-<seed>.log")
        if len(r['configs']) > 1:
            extra.append(f"POOLS {len(r['configs'])} DIFFERENT HEADER CONFIGS in one arm")
        if extra:
            L.append(f"  {arm}: " + '; '.join(extra))
    rows = [A[s]['table_rows'] for s in seeds] + [B[s]['table_rows'] for s in seeds]
    bad_tables = [s for s in seeds if not (A[s]['table_ok'] and B[s]['table_ok'])]

    def cfg_of(r):
        return dict(max(r['configs'], key=lambda c: len(r['configs'][c]))) if r['configs'] else {}
    ca, cb = cfg_of(ra), cfg_of(rb)
    run = ' '.join(f"{k}={ca.get(k, '?')}" for k in ('scenario', 'frames', 'sample'))
    L.append(f"  run: {run} | table rows per log {min(rows, default=0)}..{max(rows, default=0)}"
             + (f" | TABLE OUT OF ORDER on seeds {bad_tables}" if bad_tables else ""))
    diff = sorted(k for k in set(ca) | set(cb) if ca.get(k) != cb.get(k))
    L.append("  header settings that differ between the arms: "
             + (', '.join(f"{k} {ca.get(k, '-')}->{cb.get(k, '-')}" for k in diff) if diff
                else "none (the arms differ in the binary, not a knob)"))

    L.append("")
    L.append(f"GATE -- pre-ship regression check. median {base} -> {new}; seeds where {new} is higher/lower; sign p")
    for k, label, harm in GATE:
        s, x, y, up, dn = paired(A, B, seeds, k)
        res[k] = (up, dn)
        worse = (dn if harm == 'lower' else up) if harm else None
        tail = f"worse on {worse}/{len(s)} ({harm} is harm)" if harm else "context, not gated"
        nnote = f" [n={len(s)}]" if len(s) != len(seeds) else ""
        L.append(f"  {label:40} {fmt(med(x)):>11} -> {fmt(med(y)):>11}   {up:>2}/{dn:<2}  p {sign_p(up, dn):.3f}   {tail}{nnote}")

    L.append("")
    L.append("CRASH TIMING -- not a gate: the box grazes out, so these say when the crash landed, not harm")
    ext = tuple(sum(D[s].get('alive', 1) == 0 for s in seeds) for D in (A, B))
    u10 = tuple(sum(D[s].get('alive', 0) < 10 for s in seeds) for D in (A, B))
    res['died_out'], res['under10'] = ext, u10
    L.append(f"  {'died out (alive=0 at the end)':40} {ext[0]:>11} -> {ext[1]:>11}   boxes of {len(seeds)}")
    L.append(f"  {'under 10 at the end (incl. died out)':40} {u10[0]:>11} -> {u10[1]:>11}   boxes of {len(seeds)}")
    _, x, y, up, dn = paired(A, B, seeds, 'alive')
    res['alive'] = (up, dn)
    L.append(f"  {'alive at the end':40} {fmt(med(x)):>11} -> {fmt(med(y)):>11}   {up:>2}/{dn:<2}  p {sign_p(up, dn):.3f}")
    for k, label, f in (('peak', 'peak ants', fmt), ('peak_at', 'frame of the peak', fmt_frame)):
        _, x, y, up, dn = paired(A, B, seeds, k)
        L.append(f"  {label:40} {f(med(x)):>11} -> {f(med(y)):>11}   {up:>2}/{dn:<2}  p {sign_p(up, dn):.3f}")
    cr = []
    for D in (A, B):
        c = [D[s]['crash_at'] for s in seeds if D[s]['crash_at'] is not None]
        cr.append((len(c), med(c), sum(f >= 100000 for f in c)))
    res['crashed'] = (cr[0][0], cr[1][0])
    L.append(f"  {'fell below a quarter of peak (boxes)':40} {cr[0][0]:>11} -> {cr[1][0]:>11}   "
             f"median frame {fmt_frame(cr[0][1])} -> {fmt_frame(cr[1][1])}; at/after 100,000: {cr[0][2]} -> {cr[1][2]}")

    L.append("")
    L.append("NOT GATED -- kept for the record. Net food into home overcounts 3.4-4.6x "
             "(Reports/ant-scenes-2026-09-23.md s22j); raw deliveries are mostly home->home")
    for k, label in NOT_GATED:
        _, x, y, up, dn = paired(A, B, seeds, k)
        res[k] = (up, dn)
        L.append(f"  {label:40} {fmt(med(x)):>11} -> {fmt(med(y)):>11}   {up:>2}/{dn:<2}  p {sign_p(up, dn):.3f}")
    return L, res


def selftest():
    d = tempfile.mkdtemp()

    def log(fname, seed, dl, pk, born, alive, starved, ants, summary=True):
        with open(os.path.join(d, fname), 'w') as fh:
            fh.write(f"labforage: frames={900 * len(ants)} sample=900 seed={seed} scenario=played_bed (x)\n")
            fh.write("  ant crop_capacity = 5760 face J; FORAGE_DRIVE=unset (ForageDrive { need: Always })\n")
            fh.write("  frame  ants  plnts  edible   worth(J)\n")
            for i, a in enumerate(ants):
                fh.write(f"frame {i * 900} ant 3 pos 12 40 state forage\n")      # trace: never a row
                fh.write(f"  {i * 900:>5} {a:>5}    18      18       2160     18\n")
            fh.write("       900:       5 /     171283\n")                    # histogram: never a row
            if summary:
                fh.write(f"SUMMARY seed={seed} deliveries={dl} pickups_at_nest={pk} intake=100 born={born} "
                         f"alive={alive} bgen=2 deaths_by=ant[STARVED:{starved}/OLD_AGE:1] moves=9\n")
    for s in (1, 2, 3):
        # base: 24 ants summed over the table -> 21,600 ant-frames; peak 12 at 1800, below 3 at 2700
        log(f"base-{s}.log", s, 100, 90, 5, 5, 10, [0, 10, 12, 2])
        # new: more colony (108,000 ant-frames) starves more ants in total at a LOWER rate
        log(f"new-{s}.log", s, 100, 70, 7, 5 if s < 3 else 0, 20, [0, 40, 40, 40])
    log("new-ret-1.log", 1, 100, 70, 1, 5, 20, [0, 40, 40, 40])   # another arm's log: must not overwrite new seed 1
    log("base-4.log", 4, 100, 90, 5, 5, 10, [0, 10])               # unpaired
    log("base-5.log", 5, 100, 90, 5, 5, 10, [0, 10])
    log("new-5.log", 5, 100, 70, 7, 5, 20, [0, 40], summary=False)   # still running
    log("base-6.log", 7, 100, 90, 5, 5, 10, [0, 10])               # name says 6, SUMMARY says 7
    log("new-6.log", 6, 100, 70, 7, 5, 20, [0, 40])
    ra, rb = load(d, 'base'), load(d, 'new')
    L, res = compare(ra, rb, 'base', 'new')
    txt = '\n'.join(L)
    A = ra['seeds']
    ig, ic, inh, ido = (next(i for i, l in enumerate(L) if l.startswith(p) or p in l)
                        for p in ('GATE', 'CRASH TIMING', 'net food into home', 'died out'))
    checks = [
        (sorted(set(A) & set(rb['seeds'])) == [1, 2, 3],
         f"only seeds 1-3 pair (4 unpaired, 5 unfinished, 6 misnamed); paired {sorted(set(A) & set(rb['seeds']))}"),
        (rb['unfinished'] == [5] and ra['mismatch'] == [('base-6.log', 7)] and rb['other_prefix'] == 1,
         f"diagnostics: unfinished {rb['unfinished']}, mismatch {ra['mismatch']}, other prefix {rb['other_prefix']}"),
        (res['born'] == (3, 0), f"new-ret-1.log must not overwrite new seed 1's births; read {res['born']}"),
        (A[1]['antframes'] == 21600 and A[1]['table_rows'] == 4,
         f"ant-frames = table ants x 900, trace and histogram lines skipped; read {A[1]['antframes']} over {A[1]['table_rows']} rows"),
        (res['starve_rate'] == (0, 3), f"starved per million ant-frames fell on all 3 seeds; read {res['starve_rate']}"),
        (res['starved'] == (3, 0), f"raw starved rose on all 3 (the rate is not the raw count); read {res['starved']}"),
        (res['net_home'] == (3, 0), f"net food into home rose on all 3 seeds; read {res['net_home']}"),
        (res['deliveries'] == (0, 0), f"raw deliveries are identical; read {res['deliveries']}"),
        (res['died_out'] == (0, 1), f"one new-arm box died out; read {res['died_out']}"),
        (res['crashed'] == (3, 0) and A[1]['crash_at'] == 2700, f"base boxes crash at 2700; read {res['crashed']}, {A[1]['crash_at']}"),
        (ig < ic < inh and ido > ic, f"order must be gate, crash timing, then net home; lines {ig}, {ic}, {inh}, died out {ido}"),
        ('none (the arms differ in the binary' in txt, "identical headers must say so"),
        ('POOLS 2 DIFFERENT HEADER CONFIGS' in txt, "an arm holding two frame counts must say it pools two configs"),
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
