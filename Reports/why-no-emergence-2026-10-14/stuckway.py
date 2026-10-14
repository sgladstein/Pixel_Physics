"""stuckway.py RUN -- for starvers whose final hunger spell was spent in the nest: did they get closer to the door?

Per starver (final spell, as starvers.py): depth below the ground line at the spell's start, its minimum and at death;
nearest approach to the door; share of decisions with a move; share of moves that went up; how many distinct cells
it visited; the share of decisions whose outcome was a fall. Ground line and door from the FOUNDED line.
"""
import csv, gzip, os, sys, collections, statistics, re

LO = int(os.environ.get("LO", 20_000))
GAP = 60


def main(path):
    founded = next(l for l in open(f"{path}/events.txt") if "FOUNDED" in l)
    print(founded.strip()[:300])
    ds = {}
    for line in open(f"{path}/events.txt"):
        p = line.split()
        if len(p) > 1 and p[1] == "DIED" and "cause=STARVED" in line and int(p[0]) >= LO:
            kv = dict(x.split("=", 1) for x in p[2:] if "=" in x)
            if kv.get("zone") == "nest":
                ds[kv["id"]] = int(p[0])
    gy = int(re.search(r"ground_y=(\d+)", founded).group(1))
    door = (int(re.search(r"nest_x=(\d+)", founded).group(1)), gy)
    sp = {}
    f = gzip.open(f"{path}/hungry.csv.gz", "rt")
    r = csv.reader(f)
    h = next(r)
    ix = {k: i for i, k in enumerate(h)}
    try:
        for row in r:
            aid = row[ix["id"]]
            if aid not in ds:
                continue
            fr = int(row[ix["frame"]])
            x, y = int(row[ix["hx"]]), int(row[ix["hy"]])
            cur = sp.get(aid)
            if cur is None or fr - cur["last"] > GAP:
                cur = dict(start=fr, last=fr, y0=y, ymin=y, dmin=1e9, n=0, mv=0, up=0, cells=set(), outc=collections.Counter(), y=y)
                sp[aid] = cur
            cur["last"] = fr
            cur["n"] += 1
            moved = row[ix["moved"]] in ("1", "true", "True")
            cur["mv"] += moved
            ya = row[ix["hy_after"]]
            if moved and ya and int(ya) < y:
                cur["up"] += 1
            cur["ymin"] = min(cur["ymin"], y)
            cur["dmin"] = min(cur["dmin"], abs(x - door[0]) + abs(y - door[1]))
            cur["cells"].add((x, y))
            cur["outc"][row[ix["outcome"]]] += 1
            cur["y"] = y
    except EOFError:
        pass
    g = [c for c in sp.values() if c["n"] > 0]
    if not g:
        print("no nest starvers")
        return
    med = lambda k: statistics.median(c[k] for c in g)
    print(f"nest starvers {len(g)} (of {len(ds)} died in nest)")
    print(f"  depth below ground at spell start median {statistics.median(c['y0']-gy for c in g):.0f}, shallowest reached median {statistics.median(c['ymin']-gy for c in g):.0f}, at death median {statistics.median(c['y']-gy for c in g):.0f}")
    print(f"  nearest approach to door (cells, manhattan) median {med('dmin'):.0f}; share that came within 5: {sum(1 for c in g if c['dmin']<=5)/len(g):.0%}")
    print(f"  moved on {statistics.median(c['mv']/c['n'] for c in g):.0%} of decisions (median); of moves, up {statistics.median(c['up']/max(1,c['mv']) for c in g):.0%}")
    print(f"  distinct cells visited median {statistics.median(len(c['cells']) for c in g):.0f} over median {statistics.median(c['n'] for c in g):.0f} decisions")
    oc = collections.Counter()
    for c in g:
        oc.update(c["outc"])
    tot = sum(oc.values())
    print("  outcomes: " + ", ".join(f"{k or '-'} {v/tot:.0%}" for k, v in oc.most_common(8)))


if __name__ == "__main__":
    for p in sys.argv[1:]:
        print("==", p.rstrip("/").split("/")[-1])
        main(p)
