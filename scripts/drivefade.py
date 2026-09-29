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


if __name__ == '__main__':
    mode = sys.argv[1]
    if mode == 'pulsed':
        pulsed(sys.argv[2], sys.argv[3], int(sys.argv[4]))
    elif mode == 'unlimited':
        unlimited(sys.argv[2], sys.argv[3], int(sys.argv[4]))
    else:
        reach80(sys.argv[2])
