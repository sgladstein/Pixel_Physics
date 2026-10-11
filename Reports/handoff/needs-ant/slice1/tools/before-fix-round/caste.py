"""caste.py RUN [FROM TO]: the walk census split by caste (colony.csv `worker`: nest-bound for life).
Per caste: mean ants per census by drive/job and zone, share with p_move == 0, pellet/crop."""
import sys, csv, collections
run = sys.argv[1]
a = int(sys.argv[2]) if len(sys.argv) > 2 else 60000
b = int(sys.argv[3]) if len(sys.argv) > 3 else 140000
pos = {}
for r in csv.DictReader(open(f"{run}/colony.csv")):
    f = int(r["frame"])
    if a <= f <= b:
        pos[(f, r["id"])] = r
frames = set()
cnt = collections.defaultdict(collections.Counter)
for r in csv.DictReader(open(f"{run}/walk_colony.csv")):
    f = int(r["frame"])
    if not (a <= f <= b):
        continue
    frames.add(f)
    p = pos.get((f, r["id"]))
    if p is None:
        continue
    caste = "nestworker" if p["worker"] == "1" else "forager"
    key = (caste, f"{r['drive']}/{r['job']}")
    c = cnt[key]
    c[p["zone"]] += 1; c["_n"] += 1
    c["_p0"] += float(r["p_move"]) == 0.0 if r["p_move"] not in ("", "NaN") else 0
    c["_spoil"] += p["spoil"] == "1"; c["_crop"] += p["crop_cells"] != "0"
k = len(frames)
zones = sorted({z for c in cnt.values() for z in c if not z.startswith("_")})
print(f"{run} frames {a}-{b}, {k} censuses; mean ants per census")
for caste in ("forager", "nestworker"):
    keys = [x for x in cnt if x[0] == caste]
    tot = sum(cnt[x]["_n"] for x in keys) / max(k, 1)
    print(f"-- {caste}: {tot:.0f} ants")
    print(f"{'drive/job':<16} {'ants':>6} " + " ".join(f"{z[:9]:>9}" for z in zones) + f" {'p=0':>6} {'pellet':>7} {'crop':>6}")
    for key in sorted(keys, key=lambda x: -cnt[x]["_n"]):
        c = cnt[key]
        print(f"{key[1]:<16} {c['_n']/k:>6.1f} " + " ".join(f"{c[z]/k:>9.1f}" for z in zones) + f" {c['_p0']/k:>6.1f} {c['_spoil']/k:>7.1f} {c['_crop']/k:>6.1f}")
