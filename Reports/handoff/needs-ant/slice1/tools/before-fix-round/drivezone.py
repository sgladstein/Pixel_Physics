"""drivezone.py RUN [FROM TO]: where each drive's ants stand (colony.csv zone), joined on (frame, id) with
walk_colony.csv, summed over census frames in [FROM, TO]. Also crop/spoil/energy medians per drive."""
import sys, csv, collections, statistics
run = sys.argv[1]
a = int(sys.argv[2]) if len(sys.argv) > 2 else 60000
b = int(sys.argv[3]) if len(sys.argv) > 3 else 140000
pos = {}
for r in csv.DictReader(open(f"{run}/colony.csv")):
    f = int(r["frame"])
    if a <= f <= b:
        pos[(f, r["id"])] = r
cnt = collections.defaultdict(collections.Counter)
en = collections.defaultdict(list)
frames = set()
for r in csv.DictReader(open(f"{run}/walk_colony.csv")):
    f = int(r["frame"])
    if not (a <= f <= b):
        continue
    frames.add(f)
    p = pos.get((f, r["id"]))
    if p is None:
        continue
    key = f"{r['drive']}/{r['job']}"
    cnt[key][p["zone"]] += 1
    cnt[key]["_n"] += 1
    cnt[key]["_spoil"] += p["spoil"] == "1"
    cnt[key]["_crop"] += p["crop_cells"] != "0"
    en[key].append(float(p["energy_j"]))
zones = sorted({z for c in cnt.values() for z in c if not z.startswith("_")})
k = len(frames)
print(f"{run} frames {a}-{b}, {k} censuses; mean ants per census")
print(f"{'drive/job':<16} {'ants':>6} " + " ".join(f"{z[:9]:>9}" for z in zones) + f" {'pellet':>7} {'crop':>6} {'E med':>7}")
for key in sorted(cnt, key=lambda x: -cnt[x]["_n"]):
    c = cnt[key]
    print(f"{key:<16} {c['_n']/k:>6.1f} " + " ".join(f"{c[z]/k:>9.1f}" for z in zones) + f" {c['_spoil']/k:>7.1f} {c['_crop']/k:>6.1f} {statistics.median(en[key]):>7.0f}")
