#!/usr/bin/env python3
"""fedboundary.py RUN... : at the brood under the door, what FED ants (energy at or over the start
grant, e >= 1 -- the ants hungry.csv.gz never logs) do, by what is in the cell straight below them.

Door column x 253-260, head on row 155 or deeper (walk=1 records from 5 rows above the ground line).
Below = the cell straight down (nb[6]): brood (bare, or an ant standing on it) or open (empty, or an
ant in it). Prints per seed: decisions, ants at a time, stood still, steps by direction, the step
straight down offered and taken per offer, and the pull when the chooser ran.
"""
import gzip, sys, os, collections

DIRS = [(1, 0), (1, -1), (0, -1), (-1, -1), (-1, 0), (-1, 1), (0, 1), (1, 1)]
GROUP = {0: "side", 4: "side", 1: "up", 3: "up", 2: "up", 5: "down", 7: "down", 6: "down"}
DOOR = (253, 260)


def below(ch):
    return "brood" if ch in "bB" else ("open" if ch in ".a" else "other")


def band(dy):
    if dy < 0:
        return "mound, 1-5 rows above the door"
    if dy <= 2:
        return "rows 0-2 under the ground line"
    if dy <= 5:
        return "rows 3-5"
    return "rows 6 and deeper"


def main():
    for run in sys.argv[1:]:
        gy = 160
        for line in open(os.path.join(run, "events.txt")):
            if "FOUNDED" in line:
                gy = int(dict(t.split("=") for t in line.split() if "=" in t)["ground_y"])
                break
        C = collections.Counter()
        pulls = collections.Counter()
        frames = set()
        with gzip.open(os.path.join(run, "walkrows.csv.gz"), "rt") as f:
            head = f.readline().rstrip("\n").split(",")
            ix = {h: i for i, h in enumerate(head)}
            for line in f:
                r = line.rstrip("\n").split(",")
                frames.add(r[0])
                hx, hy = int(r[ix["hx"]]), int(r[ix["hy"]])
                if not (DOOR[0] <= hx <= DOOR[1]) or hy < gy - 5:
                    continue
                e = float(r[ix["e"]]) if r[ix["e"]] else 1.0
                fed = "fed" if e >= 1.0 else "hungry"
                nb = r[ix["nb"]]
                b = below(nb[6])
                if b == "other":
                    continue
                for key in ((fed, b, "all"), (fed, b, band(hy - gy))):
                    C[key + ("n",)] += 1
                    out = r[ix["outcome"]]
                    if out == "roll_failed_idle":
                        C[key + ("stood",)] += 1
                    elif r[ix["moved"]] == "1" and r[ix["hx_after"]] != "":
                        dx, dy = int(r[ix["hx_after"]]) - hx, int(r[ix["hy_after"]]) - hy
                        g = "down" if dy > 0 else ("side" if dy == 0 else "up")
                        C[key + ("step " + g,)] += 1
                    else:
                        C[key + ("other",)] += 1
                    chose = r[ix["chose"]]
                    if r[ix["pull"]] != "not scored" and chose != "":
                        C[key + ("chooser",)] += 1
                        opts = int(r[ix["opts"]])
                        if opts >> 6 & 1:
                            C[key + ("S offered",)] += 1
                            C[key + ("S taken",)] += int(chose) == 6
                        if key[2] == "all":
                            pulls[(fed, b, r[ix["pull"]])] += 1
        span = len(frames) and (max(map(int, frames)) - min(map(int, frames)) + 1)
        print(f"== {run}")
        for fed in ("fed", "hungry"):
            for b in ("brood", "open"):
                for bd in ("all", "mound, 1-5 rows above the door", "rows 0-2 under the ground line", "rows 3-5", "rows 6 and deeper"):
                    k = (fed, b, bd)
                    n = C[k + ("n",)]
                    if n < 200:
                        continue
                    so, st = C[k + ("S offered",)], C[k + ("S taken",)]
                    ch = C[k + ("chooser",)]
                    print(f"  {fed:6s} below={b:5s} {bd:32s} {n:7,} dec ({n*5/span:4.1f} ants)  stood {C[k+('stood',)]/n:4.0%}"
                          f"  down {C[k+('step down',)]/n:4.0%}  side {C[k+('step side',)]/n:4.0%}  up {C[k+('step up',)]/n:4.0%}  other {C[k+('other',)]/n:4.0%}"
                          f"  | straight down offered {so/max(ch,1):4.0%} of chooser decisions, taken {st/max(so,1):4.0%} per offer")
                tot = sum(v for (ff, bb, p), v in pulls.items() if ff == fed and bb == b)
                if tot:
                    print(f"      pulls ({fed}, {b} below, {tot:,} chooser decisions): " + ", ".join(f"{p} {v/tot:.0%}" for (ff, bb, p), v in sorted(pulls.items(), key=lambda t: -t[1]) if ff == fed and bb == b and v / tot >= 0.02))


if __name__ == "__main__":
    main()
