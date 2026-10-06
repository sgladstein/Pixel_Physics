"""digtrace.py RUN... [--from F] [--to T] -- what the dig gate read and what it did, by place (deep trace lane, 2026-10-06).

Reads deeptrace `dig=1`'s digrows.csv.gz (one row per ant per decision). Every row is booked by where the head was:
below the old ground line by the scorecard's bands (knot = within 4 columns of the door and 10 rows of the ground
line; top = the rest of the top 10 rows; shaft = the door column deeper; deep = off the door column, deeper), above it
by the census zone (mound_in, mound_top, surface, food). Columns used: at_nest and crowding (the brain's AtNest and
Crowding inputs, as the ant read them that tick), dig (how far the dig got: creature::DigWhy), dig_p (the urge).

`Crowding` is what CROWDING_LOCAL changes, and only where AtNest > 0 and the room gate is on (creature.rs, the
`inputs[I::Crowding]` assignment): shipped, the nest's room census (`NestRoom::occupancy`), one value for every ant at
that nest; `near`, the share of the 24 cells round the head holding another ant.

Per run and place: decisions, the share with AtNest > 0, Crowding there (mean, p10/p50/p90), the mean urge there,
digs asked (the dig was reached), rolls won (cue, roof, nothing to cut, face trip or a cut) and cuts. Then, for rows
with AtNest > 0, the same by Crowding tenth, which is the switch's own question: does the urge, and the cutting,
follow how crowded it is where the ant stands?
"""
import sys, gzip, csv, collections


def founded(run):
    for L in open(f"{run}/events.txt"):
        if " FOUNDED " in L:
            kv = dict(t.split("=", 1) for t in L.split() if "=" in t)
            return int(kv["nest_x"]), int(kv["ground_y"])
    raise SystemExit(f"{run}: no FOUNDED line")


def place(x, y, zone, nx, gy):
    d = y - gy
    if d > 0:
        door = abs(x - nx) <= 4
        if d <= 10:
            return "knot" if door else "top"
        return "shaft" if door else "deep"
    return zone


WON = {"cue", "roof", "no_ground", "cut", "face"}
PLACES = ["knot", "top", "shaft", "deep", "mound_in", "mound_top", "surface", "food"]


def q(v, p):
    if not v:
        return float("nan")
    s = sorted(v)
    return s[min(len(s) - 1, int(p * len(s)))]


def main():
    a = sys.argv[1:]
    lo = int(a[a.index("--from") + 1]) if "--from" in a else 50000
    hi = int(a[a.index("--to") + 1]) if "--to" in a else 100000
    runs = [x for j, x in enumerate(a) if not x.startswith("--") and (j == 0 or a[j - 1] not in ("--from", "--to"))]
    for run in runs:
        nx, gy = founded(run)
        P = collections.defaultdict(lambda: collections.Counter())
        crowd = collections.defaultdict(list)   # place -> Crowding readings at AtNest > 0 (sampled 1 in 7)
        urge = collections.defaultdict(float)
        B = collections.defaultdict(lambda: collections.Counter())
        Burge = collections.defaultdict(float)
        last = 0
        try:
            with gzip.open(f"{run}/digrows.csv.gz", "rt") as fh:
                for i, r in enumerate(csv.DictReader(fh)):
                    f = int(r["frame"])
                    if f < lo:
                        continue
                    if f > hi:
                        break
                    last = f
                    pl = place(int(r["hx"]), int(r["hy"]), r["zone"], nx, gy)
                    c = P[pl]
                    c["rows"] += 1
                    dig = r["dig"]
                    asked = dig != "not_asked"
                    c["asked"] += asked
                    c["won"] += dig in WON
                    c["cut"] += dig == "cut"
                    if float(r["at_nest"] or 0) > 0:
                        c["nest"] += 1
                        cr = float(r["crowding"] or 0)
                        # dig_p is blank when the dig was not reached (not_asked); the urge is averaged over asked rows
                        p = float(r["dig_p"]) if r["dig_p"] else 0.0
                        c["nest_asked"] += asked
                        urge[pl] += p
                        if i % 7 == 0:
                            crowd[pl].append(cr)
                        b = min(9, int(cr * 10))
                        B[b]["rows"] += 1
                        B[b]["asked"] += asked
                        B[b]["won"] += dig in WON
                        B[b]["cut"] += dig == "cut"
                        B[b][pl] += 1
                        Burge[b] += p
        except EOFError:
            pass
        name = "/".join(run.rstrip("/").split("/")[-1:])
        print(f"{name}: decisions {lo}-{last} (digrows.csv.gz)")
        print(f"  {'place':10} {'decisions':>10} {'AtNest>0':>9} {'Crowding p10/p50/p90 there':>27} {'mean':>5} {'urge':>6} {'asked':>8} {'won':>7} {'cuts':>6}")
        tot = collections.Counter()
        for pl in PLACES + sorted(set(P) - set(PLACES)):
            c = P.get(pl)
            if not c:
                continue
            tot.update(c)
            v = crowd.get(pl, [])
            n = c["nest"]
            mean = sum(v) / len(v) if v else float("nan")
            print(f"  {pl:10} {c['rows']:>10} {n / c['rows']:>8.0%} {q(v, .1):>9.2f} {q(v, .5):>8.2f} {q(v, .9):>8.2f} {mean:>5.2f} "
                  f"{(urge[pl] / c['nest_asked'] if c['nest_asked'] else float('nan')):>6.3f} {c['asked']:>8} {c['won']:>7} {c['cut']:>6}")
        print(f"  {'all':10} {tot['rows']:>10} {tot['nest'] / max(1, tot['rows']):>8.0%} {'':>27} {'':>5} {'':>6} {tot['asked']:>8} {tot['won']:>7} {tot['cut']:>6}")
        print(f"  AtNest > 0, by Crowding tenth: rows, mean urge (asked rows), won per asked, cuts per 10k rows; where (knot/top/shaft/deep/above)")
        for b in range(10):
            c = B.get(b)
            if not c or not c["rows"]:
                continue
            above = c["rows"] - sum(c[k] for k in ("knot", "top", "shaft", "deep"))
            print(f"    {b / 10:.1f}-{(b + 1) / 10:.1f} {c['rows']:>9} {Burge[b] / max(1, c['asked']):>7.3f} {c['won'] / max(1, c['asked']):>7.3f} "
                  f"{1e4 * c['cut'] / c['rows']:>8.1f}   {c['knot']}/{c['top']}/{c['shaft']}/{c['deep']}/{above}")


if __name__ == "__main__":
    main()
