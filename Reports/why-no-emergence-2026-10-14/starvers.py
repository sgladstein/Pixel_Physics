"""starvers.py RUN [RUN...] -- how did each starved adult die: held, stuck, or free and out of reserve?

Needs deeptrace `hungry=1` (events.txt death lines, hungry.csv.gz every decision of every hungry ant).
Window LO-HI (env, default 20k-300k).

For each STARVED death, its final hunger spell = its run of hungry decisions ending at death with no gap
over GAP frames. Over that spell:
  len     frames from the spell's first hungry decision to death
  e0      energy (as a share of start_energy, the `e` column) at the spell's first decision
  moved   share of decisions on which the ant moved
  pulls   the chooser's pull on each decision (creature::PULL_WHY_NAMES)
  heap    whether any decision was in the `food` zone, and whether it bit anything
Class:
  stuck   moved on under 10% of decisions
  held    over half the spell's decisions under a pull that does not lead to food
  free    otherwise (moving, mostly unpulled or pulled toward food)
"""
import csv, gzip, os, sys, collections, statistics

LO, HI = int(os.environ.get("LO", 20_000)), int(os.environ.get("HI", 300_000))
GAP = 60
FOOD_PULLS = {"hungry out", "nest store", "store sent out", "hungry home", "none", "not scored", ""}


def deaths(path):
    out = {}
    for line in open(f"{path}/events.txt"):
        p = line.split()
        if len(p) > 1 and p[1] == "DIED":
            fr = int(p[0])
            kv = dict(x.split("=", 1) for x in p[2:] if "=" in x)
            if kv.get("cause") == "STARVED" and LO <= fr < HI:
                out[kv["id"]] = (fr, kv.get("zone", "?"))
    return out


def classify(path):
    """Streams hungry.csv.gz once, keeping per starver only its current spell's running counts."""
    ds = deaths(path)
    ids = set(ds)
    sp = {}  # id -> [start_frame, last_frame, e0, n, moved, nonfood, heap, pulls Counter, zones Counter]
    f = gzip.open(f"{path}/hungry.csv.gz", "rt")
    r = csv.reader(f)
    h = next(r)
    ix = {k: i for i, k in enumerate(h)}
    i_f, i_id, i_e, i_mv, i_pull, i_zone = ix["frame"], ix["id"], ix["e"], ix["moved"], ix["pull"], ix["zone"]
    try:
        for row in r:
            aid = row[i_id]
            if aid not in ids:
                continue
            fr = int(row[i_f])
            cur = sp.get(aid)
            if cur is None or fr - cur[1] > GAP:
                cur = [fr, fr, float(row[i_e] or 0), 0, 0, 0, False, collections.Counter(), collections.Counter()]
                sp[aid] = cur
            cur[1] = fr
            cur[3] += 1
            cur[4] += row[i_mv] in ("1", "true", "True")
            pl = row[i_pull]
            cur[5] += pl not in FOOD_PULLS
            cur[6] = cur[6] or row[i_zone] == "food"
            cur[7][pl] += 1
            cur[8][row[i_zone]] += 1
    except EOFError:
        pass
    res = []
    for aid, (fr, zone) in ds.items():
        c = sp.get(aid)
        if c is None or c[3] == 0:
            res.append(dict(id=aid, cls="no rows", len=0, zone=zone))
            continue
        moved = c[4] / c[3]
        nonfood = c[5] / c[3]
        cls = "stuck" if moved < 0.10 else ("held" if nonfood > 0.5 else "free")
        res.append(dict(id=aid, cls=cls, len=fr - c[0], e0=c[2], moved=moved, top=c[7].most_common(2), heap=c[6],
                        zone=zone, zones=c[8].most_common(2)))
    return res


def main():
    for path in sys.argv[1:]:
        res = classify(path)
        name = path.rstrip("/").split("/")[-1]
        c = collections.Counter(r["cls"] for r in res)
        print(f"== {name}  starved {len(res)} in {LO}-{HI}: {dict(c)}")
        for cls in ("stuck", "held", "free"):
            g = [r for r in res if r["cls"] == cls]
            if not g:
                continue
            lens = [r["len"] for r in g]
            tops = collections.Counter(p for r in g for p, _ in r["top"][:1])
            zs = collections.Counter(z for r in g for z, _ in r["zones"][:1])
            dz = collections.Counter(r["zone"] for r in g)
            print(f"  {cls:5} n {len(g):4}  final spell median {statistics.median(lens):,.0f} frames  "
                  f"e0 median {statistics.median(r['e0'] for r in g):.2f}  reached heap {sum(r['heap'] for r in g)}  "
                  f"top pull {tops.most_common(3)}  spent most in {zs.most_common(3)}  died in {dz.most_common(3)}")


if __name__ == "__main__":
    main()
