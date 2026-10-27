"""margin.py RUN [RUN...] -- is an ant's energy reserve about one foraging round trip?

Reads deeptrace runs made with `ants=60 hungry=1 mapevery=1000` (focal ants' every decision in
ticks.csv.gz; every death in events.txt; every life in ledger.csv).

Per run, over the window [LO, HI) frames (default 100k-300k, the settled colony):
  burn   -- J per frame an ant loses when nothing is digesting and its crop is empty
            (median of -d_energy / frames between consecutive decisions of one ant).
  reserve-- for every focal decision: (energy_j + crop_j) / burn, in frames of living.
            Also at the moment a forager leaves home on a trip.
  trips  -- for focal ants: leave home (nest/mound_in/mound_top) -> first decision at the heap
            ('food' zone) -> first decision back home with food in the crop. Outbound, return,
            round trip in frames; energy+crop at departure.
  deaths -- by cause over the window, from events.txt.
"""
import csv, gzip, sys, statistics, collections

import os
LO, HI = int(os.environ.get("LO", 100_000)), int(os.environ.get("HI", 300_000))
HOME = {'nest', 'mound_in', 'mound_top'}


def q(xs, p):
    if not xs:
        return float('nan')
    xs = sorted(xs)
    return xs[min(len(xs) - 1, int(p * len(xs)))]


def run(path):
    burn_samples = []
    reserve_j = []
    last = {}
    trips = []
    state = {}  # id -> dict(phase, t_leave, e_leave, t_food)
    f = gzip.open(f"{path}/ticks.csv.gz", "rt")
    r = csv.reader(f)
    h = next(r)
    ix = {k: i for i, k in enumerate(h)}
    I = lambda k: ix[k]
    i_frame, i_id, i_zone, i_e, i_cj, i_dig, i_cc = I('frame'), I('id'), I('zone'), I('energy_j'), I('crop_j'), I('digesting'), I('crop_cells')
    try:
        for row in r:
            fr = int(row[i_frame])
            if fr >= HI:
                break
            aid = row[i_id]
            zone = row[i_zone]
            e = float(row[i_e] or 0)
            cj = float(row[i_cj] or 0)
            dg = float(row[i_dig] or 0)
            p = last.get(aid)
            if p is not None and fr >= LO:
                pf, pe, pcj, pdg = p
                dt = fr - pf
                if 0 < dt <= 12 and pcj == 0 and cj == 0 and pdg == 0 and dg == 0 and e < pe:
                    burn_samples.append((pe - e) / dt)
            last[aid] = (fr, e, cj, dg)
            if fr >= LO:
                reserve_j.append(e + cj)
            # trips
            s = state.setdefault(aid, {'phase': 'home' if zone in HOME else 'out', 't_leave': None, 'e_leave': None, 't_food': None})
            if zone in HOME:
                if s['phase'] == 'back' and cj > 0 and s['t_leave'] is not None and s['t_leave'] >= LO:
                    trips.append((s['t_food'] - s['t_leave'], fr - s['t_food'], fr - s['t_leave'], s['e_leave']))
                s.update(phase='home', t_leave=None, t_food=None)
                s['last_home'] = (fr, e + cj)
            else:
                if s['phase'] == 'home':
                    lh = s.get('last_home', (fr, e + cj))
                    s.update(phase='out', t_leave=lh[0], e_leave=lh[1])
                if zone == 'food' and s['phase'] == 'out' and s['t_leave'] is not None:
                    s.update(phase='back', t_food=fr)
    except EOFError:
        pass
    burn = statistics.median(burn_samples) if burn_samples else float('nan')
    deaths = collections.Counter()
    starved_zone = collections.Counter()
    for line in open(f"{path}/events.txt"):
        parts = line.split()
        if len(parts) > 1 and parts[1] == 'DIED':
            fr = int(parts[0])
            if LO <= fr < HI:
                kv = dict(p.split('=', 1) for p in parts[2:] if '=' in p)
                deaths[kv.get('cause', '?')] += 1
                if kv.get('cause') == 'STARVED':
                    starved_zone[kv.get('zone', '?')] += 1
    return burn, burn_samples, reserve_j, trips, deaths, starved_zone


def main():
    for path in sys.argv[1:]:
        burn, bs, res, trips, deaths, sz = run(path)
        name = path.rstrip('/').split('/')[-1]
        print(f"== {name}  window {LO}-{HI}")
        print(f"  burn J/frame: median {burn:.4f}  p10 {q(bs,.1):.4f}  p90 {q(bs,.9):.4f}  (n {len(bs)})")
        rf = [x / burn for x in res]
        print(f"  reserve, focal decisions (frames of living): p10 {q(rf,.1):,.0f}  median {q(rf,.5):,.0f}  p90 {q(rf,.9):,.0f}  (n {len(rf)})")
        print(f"  share of focal decisions with reserve under 2,000 frames: {sum(1 for x in rf if x < 2000)/max(1,len(rf)):.1%}")
        if trips:
            out = [t[0] for t in trips]; back = [t[1] for t in trips]; rt = [t[2] for t in trips]; el = [t[3] / burn for t in trips]
            ratio = [t[3] / burn / max(1, t[2]) for t in trips]
            print(f"  trips (n {len(trips)}): out median {q(out,.5):,}  back median {q(back,.5):,}  round trip median {q(rt,.5):,}  p90 {q(rt,.9):,}")
            print(f"  reserve at departure (frames): p10 {q(el,.1):,.0f}  median {q(el,.5):,.0f}")
            print(f"  reserve / that trip's round trip: p10 {q(ratio,.1):.1f}  median {q(ratio,.5):.1f}  share under 1: {sum(1 for x in ratio if x<1)/len(ratio):.1%}")
        else:
            print("  trips: none")
        print(f"  deaths: {dict(deaths)}   starved by zone: {dict(sz)}")


if __name__ == '__main__':
    main()
