"""SCRATCH: what `PIXEL_PHYSICS_SPOIL_ROLL` does to a colony's nest and mound, from paired deeptrace runs.

usage: python3 -I rollcheck.py ROOT        # ROOT holds {off,on}-s{N}/map_f*.txt and {off,on}-s{N}.log

Per map (deeptrace's `write_map` frame: ground at row 60, door at column 80):
- the general nest score (`nestscore.score`) and the cells dug;
- **door open**: the nest under the door reaches the open sky through open cells;
- **mound**: ground cells above the old ground row; its **overhang** cells (mound with open air straight beneath it --
  the lattice signature the spoil record names, `dead-ends.md` "putting a dug pellet back loose"); its tallest
  column; and the share of it within 6 columns of the door (soil standing at the mouth);
and from each log's last `spoil_roll:` line, the pellets put down and how many rolled -- the "did it fire" count, which
must read 0 on every `off` arm."""
import glob, json, os, re, sys
from collections import deque
import numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import nestscore as ns

G, DOOR = 60, 80


def door_open(m):
    h, w = m.shape
    seen = np.zeros_like(m)
    q = deque((y, x) for x in range(DOOR - 3, DOOR + 4) for y in range(G, G + 4) if m[y, x])
    for y, x in q:
        seen[y, x] = True
    while q:
        y, x = q.popleft()
        if y < G - 1:
            return True
        for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            ny, nx = y + dy, x + dx
            if 0 <= ny < h and 0 <= nx < w and m[ny, nx] and not seen[ny, nx]:
                seen[ny, nx] = True
                q.append((ny, nx))
    return False


def mound(m):
    solid = ~m[:G]
    cells = int(solid.sum())
    over = int((solid[:-1] & m[1:G]).sum())
    tall = int(max((G - np.argmax(solid[:, x]) if solid[:, x].any() else 0) for x in range(m.shape[1])))
    near = int(solid[:, max(0, DOOR - 6):DOOR + 7].sum())
    return cells, over, tall, near


def rolled(log):
    last = None
    for line in open(log):
        if "spoil_roll:" in line:
            last = line
    if not last:
        return None
    d = dict(kv.split("=") for kv in last.split(":", 1)[1].split())
    return int(d["dumped"]), int(d["rolled"])


def main(root):
    rows = []
    for run in sorted(glob.glob(f"{root}/*-s*/")):
        arm, seed = re.search(r"/(on|off)-s(\d+)/$", run).groups()
        for path in sorted(glob.glob(f"{run}/map_f*.txt")):
            frame = int(re.search(r"map_f(\d+)", path).group(1))
            _, m = ns.read_map(path)
            s, _ = ns.score(m, G, DOOR)
            c, o, t, n = mound(m)
            rows.append(dict(arm=arm, seed=int(seed), frame=frame, score=s["score"], dug=s["dug"], rooms=s["chambers"],
                             door_open=door_open(m), mound=c, overhang=o, tallest=t, at_door=n))
        r = rolled(run.rstrip("/") + ".log")
        if r:
            rows.append(dict(arm=arm, seed=int(seed), frame=-1, dumped=r[0], rolled=r[1]))
    for r in rows:
        print(json.dumps(r))
    # paired summary at the last common frame
    last = max(r["frame"] for r in rows)
    print("\npaired at frame", last)
    keys = ("score", "dug", "mound", "overhang", "tallest", "at_door")
    for k in keys + ("door_open",):
        d = []
        for seed in sorted({r["seed"] for r in rows}):
            a = [r for r in rows if r["seed"] == seed and r["frame"] == last]
            off = next((r[k] for r in a if r["arm"] == "off"), None)
            on = next((r[k] for r in a if r["arm"] == "on"), None)
            if off is not None and on is not None:
                d.append((seed, off, on))
        print(f"  {k:10s} " + "  ".join(f"s{s}: {o} -> {n}" for s, o, n in d))
    for r in rows:
        if r["frame"] == -1:
            print(f"  {r['arm']}-s{r['seed']}: pellets put down {r['dumped']}, rolled {r['rolled']}")


if __name__ == "__main__":
    main(sys.argv[1])
