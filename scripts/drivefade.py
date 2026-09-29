"""Does the `returns` forage drive fade when food stops, and stay up while it comes?

Reads two arms of `trailfollow decisioncsv` traces (off and on, 24 seeds each; `.csv` or `.csv.gz`, with the
2026-09-29 columns `trip_load` and `bite_x`) and prints, per arm:
  pulsed  <off dir> <on dir> <gap>   a pulsed pile (`food=30 refill=6000`): bookings by source (pile = the marking
      bite within 10 of the pile; home = the rest), the drive over the last third of each refill cycle (row mean,
      share at >= 0.9), the drive once the pile has stopped paying (`[last pile return + 1,400, refill)`, median
      over seed-cycles), the drive while the pile is worked, frames from refill to the first pile return, and the
      paired per-seed ratio of pile returns;
  unlimited <off dir> <on dir> <gap> an unlimited pile: the pooled row drive over frames 6,000-24,000, the worst
      seed's mean, and the most time any seed spends below 0.5;
  reach80 <dir>                      loose pickups past the roam gate within 26 of a door, and the share at 17-26
      (is the trip reach cutting through a real spread of food?).
  setouts <off dir> <on dir>         a pulsed pile: set-outs by fed ants (at_nest high -> low while empty, energy_j >=
      200) by third of the refill cycle, pooled and paired by seed (the effect the drive should move).
  gap <dir>                          marking pickups past the roam gate by Chebyshev distance of the FOOD CELL from the
      nearest door (`bite_door`, loose food only), 10-26: where the trip reach's 16 sits.
  trace <dir> <gap> <seed>...        per seed, rows of fed driven ants with drive < 0.5 (frames >= 6,000), and the share of
      them -- against the share of all fed driven rows -- while another ant, seen in the last 30 frames, carries a
      trip load marked at the pile along the road home (nest+26 < x < nest+gap-10): a stand-down while food is on
      its way (false) or in a real lull.
Rows are foraged ants' decisions with drive > 0; cycles 2-4 of the pulsed pile. Built for
`Reports/ant-scenes-2026-09-23.md` section 22u (the trip reach); its off arm reproduced the offline replay's
0.933 / 84.6% / 0.994 / 966 / 2,112 exactly.
usage: python3 scripts/drivefade.py pulsed csv/off-90 csv/on-90 90
"""
import csv, glob, gzip, sys, statistics as st, collections as C
CYC = 6000; END = 24000


def rows(path):
    with (gzip.open(path, 'rt') if path.endswith('.gz') else open(path)) as fh:
        rd = csv.reader(fh); h = next(rd); ix = {k: i for i, k in enumerate(h)}
        for r in rd:
            yield ix, r


def walk(d, gap):
    """Per seed: drive rows [(frame, drive)], pile bookings [frame], home bookings [frame]."""
    out = {}
    for f in sorted(glob.glob(d + '/*.csv*')):
        seed = int(f.split('seed')[1].split('-')[0])
        drv = []; pile = []; home = []; prev = {}; mark = {}
        for ix, r in rows(f):
            fr = int(r[ix['frame']]); a = r[ix['id']]
            d_ = r[ix['drive']]
            if d_ not in ('NaN', 'nan', '') and float(d_) > 0:
                drv.append((fr, float(d_)))
            t = r[ix['trip_load']] == '1'
            bx = r[ix['bite_x']]
            p = prev.get(a)
            if t and not (p and p[0]) and bx != '-':
                mark[a] = int(bx) - int(r[ix['nest_x']])
            if p and p[0] and not t:
                dx = mark.pop(a, None)
                (pile if dx is not None and dx >= gap - 10 else home).append(fr)
            prev[a] = (t,)
        out[seed] = (drv, sorted(pile), sorted(home))
    return out


def q(v, p):
    v = sorted(v); return v[min(len(v) - 1, int(p * len(v)))] if v else float('nan')


def pulsed(off, on, gap):
    res = {}
    for name, d in (('off', off), ('on', on)):
        W = walk(d, gap)
        late = []; stop = []; working = []; first = []; hp = C.Counter()
        for seed, (drv, pile, home) in W.items():
            hp['pile'] += len(pile); hp['home'] += len(home)
            for c in range(1, 4):
                lo, hi = c * CYC, (c + 1) * CYC
                late += [x for fr, x in drv if lo + 4000 <= fr < hi]
                pc = [t for t in pile if lo <= t < hi]
                if not pc:
                    continue
                first.append(pc[0] - lo)
                w = [x for fr, x in drv if pc[0] <= fr <= pc[-1]]
                if w: working.append(st.mean(w))
                s = [x for fr, x in drv if pc[-1] + 1400 <= fr < hi]
                if s: stop.append(st.mean(s))
        res[name] = dict(late_mean=st.mean(late), late_ge09=sum(x >= 0.9 for x in late) / len(late), stop_med=st.median(stop), n_stop=len(stop),
                         working=st.mean(working), refill_first_med=st.median(first), refill_first_p90=q(first, 0.9), pile=hp['pile'], home=hp['home'],
                         pile_by_seed={s: len(v[1]) for s, v in W.items()})
    for name in ('off', 'on'):
        r = res[name]
        print(f"{name:3}: bookings pile {r['pile']} home {r['home']} | P3 last-third row mean {r['late_mean']:.3f}, share >=0.9 {r['late_ge09']:.1%}, stop-stretch median {r['stop_med']:.3f} (n {r['n_stop']}) | P4 working mean {r['working']:.3f} | P5 refill->first pile return median {r['refill_first_med']:.0f} p90 {r['refill_first_p90']:.0f}")
    ratios = [res['on']['pile_by_seed'][s] / res['off']['pile_by_seed'][s] for s in res['off']['pile_by_seed'] if res['off']['pile_by_seed'][s] and s in res['on']['pile_by_seed']]
    print(f"P2 pile returns per seed on/off: median ratio {st.median(ratios):.3f} over {len(ratios)} seeds; home-band bookings on {res['on']['home']} ({res['on']['home'] / max(1, res['on']['home'] + res['on']['pile']):.1%})")


def unlimited(off, on, gap):
    for name, d in (('off', off), ('on', on)):
        W = walk(d, gap)
        per = {s: [x for fr, x in v[0] if 6000 <= fr < END] for s, v in W.items()}
        pooled = [x for v in per.values() for x in v]
        worst = min((st.mean(v), s) for s, v in per.items() if v)
        low = max((sum(x < 0.5 for x in v) / len(v), s) for s, v in per.items() if v)
        print(f"{name:3}: seeds {len(per)} | pooled row mean drive {st.mean(pooled):.3f} | worst seed {worst[0]:.3f} (seed {worst[1]}) | most rows < 0.5 {low[0]:.1%} (seed {low[1]}) | bookings pile {sum(len(v[1]) for v in W.values())} home {sum(len(v[2]) for v in W.values())}")


def reach80(d):
    near = C.Counter(); n = 0
    for f in sorted(glob.glob(d + '/*.csv*')):
        for ix, r in rows(f):
            bd = r[ix['bite_door']]
            if bd in ('-', '') or r[ix['bite_tissue']] != '0' or int(r[ix['forage_max']]) < 8:
                continue
            b = int(bd)
            if b <= 26:
                n += 1; near['17-26' if b >= 17 else '<=16'] += 1
    print(f"P7 80 founders, off arm: loose pickups past the roam gate within 26 of a door {n}; at 17-26: {near['17-26']} ({near['17-26'] / max(1, n):.1%})")


def setouts(off, on):
    import math
    res = {}
    for name, d in (('off', off), ('on', on)):
        per = {}
        for f in sorted(glob.glob(d + '/*.csv*')):
            seed = int(f.split('seed')[1].split('-')[0]); last = {}; c = [0, 0, 0]
            for ix, r in rows(f):
                fr = int(r[ix['frame']]); an = float(r[ix['at_nest']]) > 0.5; k = r[ix['id']]; p = last.get(k); last[k] = an
                if fr < CYC or not p or an or r[ix['leg']] != 'empty' or float(r[ix['energy_j']]) < 200:
                    continue
                c[(fr % CYC) // 2000] += 1
            per[seed] = c
        res[name] = per
    for name in ('off', 'on'):
        t = [sum(v[i] for v in res[name].values()) for i in range(3)]
        print(f"{name:3}: set-outs by fed ants, thirds of the cycle (cycles 2-4): {t[0]} / {t[1]} / {t[2]}")
    hi = sum(res['on'][s][2] > res['off'][s][2] for s in res['off']); lo = sum(res['on'][s][2] < res['off'][s][2] for s in res['off'])
    n = hi + lo; p = min(1.0, 2 * sum(math.comb(n, k) for k in range(min(hi, lo) + 1)) / 2 ** n) if n else 1.0
    print(f"last third, per seed: on higher {hi} / lower {lo}, sign p {p:.3f}")


def gap(d):
    h = C.Counter()
    for f in sorted(glob.glob(d + '/*.csv*')):
        prev = {}
        for ix, r in rows(f):
            a = r[ix['id']]; t = r[ix['trip_src']] not in ('', '0'); p = prev.get(a, False); prev[a] = t
            bd = r[ix['bite_door']]
            if bd in ('-', '') or r[ix['bite_tissue']] != '0' or int(r[ix['forage_max']]) < 8:
                continue
            b = int(bd)
            if 10 <= b <= 26:
                h[b] += 1
    print('loose pickups past the roam gate, by distance of the food cell from the nearest door: ' + ' '.join(f"{b}:{h[b]}" for b in range(10, 27)))


def trace(d, gap_, seeds):
    for seed in seeds:
        f = glob.glob(f'{d}/*seed{seed}-gap{gap_}-*.csv*')[0]
        mark = {}; prev = {}; carry = {}; seen = {}; low = [0, 0]; allr = [0, 0]
        for ix, r in rows(f):
            fr = int(r[ix['frame']]); a = r[ix['id']]; t = r[ix['trip_load']] == '1'; bx = r[ix['bite_x']]; nx = int(r[ix['nest_x']]); x = int(r[ix['x']])
            seen[a] = fr
            if t and not prev.get(a, False) and bx != '-':
                mark[a] = int(bx) - nx
            if t and mark.get(a, 0) >= gap_ - 10 and r[ix['leg']] == 'laden' and nx + 26 < x < nx + gap_ - 10:
                carry[a] = fr
            else:
                carry.pop(a, None)
            prev[a] = t
            d_ = r[ix['drive']]
            if fr < 6000 or d_ in ('NaN', 'nan', '') or float(d_) <= 0 or float(r[ix['energy_j']]) < 200:
                continue
            live = any(fr - seen.get(b, 0) <= 30 for b in carry)
            allr[0] += 1; allr[1] += live
            if float(d_) < 0.5:
                low[0] += 1; low[1] += live
        print(f"seed {seed}: fed driven rows with drive < 0.5: {low[0]}; with a pile load on the road home {low[1] / max(1, low[0]):.0%}, against {allr[1] / max(1, allr[0]):.0%} of all fed driven rows")


if __name__ == '__main__':
    mode = sys.argv[1]
    if mode == 'pulsed':
        pulsed(sys.argv[2], sys.argv[3], int(sys.argv[4]))
    elif mode == 'unlimited':
        unlimited(sys.argv[2], sys.argv[3], int(sys.argv[4]))
    elif mode == 'setouts':
        setouts(sys.argv[2], sys.argv[3])
    elif mode == 'gap':
        gap(sys.argv[2])
    elif mode == 'trace':
        trace(sys.argv[2], int(sys.argv[3]), [int(x) for x in sys.argv[4:]])
    else:
        reach80(sys.argv[2])
