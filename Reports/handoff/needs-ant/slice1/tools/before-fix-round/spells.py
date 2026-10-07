"""spells.py RUN JOB [FROM TO]: spells of one job in the sampled decisions (walk.csv.gz): length in frames,
share of decisions that stepped, distance to the pull's target at the start and the end, depth, and
how the spell ended (the next job and drive)."""
import sys, csv, gzip, collections, statistics
run, job = sys.argv[1], sys.argv[2]
a = int(sys.argv[3]) if len(sys.argv) > 3 else 60000
b = int(sys.argv[4]) if len(sys.argv) > 4 else 100000
by = collections.defaultdict(list)
for r in csv.DictReader(gzip.open(f"{run}/walk.csv.gz", "rt")):
    f = int(r["frame"])
    if a <= f <= b:
        by[r["id"]].append(r)
spells = []
for id_, rows in by.items():
    cur = []
    for r in rows:
        if r["job"] == job:
            cur.append(r)
        else:
            if cur:
                spells.append((cur, r))
                cur = []
    if cur:
        spells.append((cur, None))
def dist(r):
    if not r["target_x"]:
        return None
    return max(abs(int(r["target_x"]) - int(r["tx"])), abs(int(r["target_y"]) - int(r["ty"])))
done = [s for s in spells if s[1] is not None]
lens = sorted(int(c[-1]["frame"]) - int(c[0]["frame"]) + 6 for c, _ in done)
print(f"{len(spells)} {job} spells ({len(done)} ended inside the window) over {len(by)} sampled ants")
if lens:
    q = lambda p: lens[min(len(lens) - 1, int(p * len(lens)))]
    print(f"length frames: median {q(0.5)}, p25 {q(0.25)}, p75 {q(0.75)}, p90 {q(0.9)}, max {lens[-1]}")
    tot = sum(lens)
    print(f"share of the job's decision-time in spells over 1,000 frames: {sum(l for l in lens if l > 1000) / tot:.0%}")
ends = collections.Counter((n["job"], n["drive"]) for _, n in done)
print("ended into:", ends.most_common(8))
moved = [statistics.mean(int(r["moved"]) for r in c) for c, _ in spells]
print(f"share of decisions that stepped, median over spells: {statistics.median(moved):.2f}")
d0 = [dist(c[0]) for c, _ in spells if dist(c[0]) is not None]
d1 = [dist(c[-1]) for c, _ in spells if dist(c[-1]) is not None]
if d0:
    print(f"distance to target at start: median {statistics.median(d0)}, at end: median {statistics.median(d1)}")
drv = collections.Counter(r["drive"] for c, _ in spells for r in c)
print("drives during the spells:", drv.most_common())
dep = collections.Counter((int(r["depth"]) // 5) * 5 if r["depth"] else None for c, _ in spells for r in c)
print("depth (rows below ground, 5-row bins; negative = above):", sorted(dep.items(), key=lambda x: (x[0] is None, x[0]))[:20])
