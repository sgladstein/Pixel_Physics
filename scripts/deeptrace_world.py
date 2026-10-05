#!/usr/bin/env python3
"""The coarse pass over a `deeptrace` run: the colony and the nest over time.

    python3 scripts/deeptrace_world.py OUT

Reads OUT/colony.csv (every live ant every `colonyevery=` frames), OUT/map_*.txt
(the ground round the nest) and OUT/events.txt (deaths), and prints / writes
OUT/world.json:

  per colony sample: live ants, workers, where they stand (nest, mound inside,
  mound top, food, surface), holding food / soil, mean energy, at home;
  per map: open cells under the old ground line joined to the nest, deepest
  open row, brood and food cells under ground, the mound (ground cells above
  the old line within 40 columns of the nest), surface water;
  per window: deaths by cause and where.
"""
import csv
import glob
import json
import os
import sys
from collections import Counter, defaultdict, deque


def main():
    out = sys.argv[1]
    nest_x = food_x = ground_y = None
    deaths = defaultdict(Counter)
    for e in open(f"{out}/events.txt"):
        p = e.split()
        if len(p) > 1 and p[1] == "FOUNDED":
            kv = dict(t.split("=") for t in p[2:] if "=" in t)
            nest_x, food_x, ground_y = int(kv["nest_x"]), int(kv["food_x"]), int(kv["ground_y"])
        elif len(p) > 1 and p[1] == "DIED":
            kv = dict(t.split("=", 1) for t in p[2:] if "=" in t)
            deaths[int(p[0]) // 10000 * 10000][f"{kv.get('cause','?')}@{kv.get('zone','?')}"] += 1

    samples = defaultdict(lambda: Counter())
    energy = defaultdict(list)
    with open(f"{out}/colony.csv") as fh:
        for r in csv.DictReader(fh):
            fr = int(r["frame"])
            s = samples[fr]
            s["ants"] += 1
            s["workers"] += int(r["worker"])
            s[f"at_{r['zone']}"] += 1
            s["food_in_crop"] += r["crop_cells"] != "0"
            s["soil_held"] += r["spoil"] != "0"
            s["home"] += int(r["home"])
            energy[fr].append(float(r["energy_j"]))
    for fr, es in energy.items():
        samples[fr]["energy_median"] = sorted(es)[len(es) // 2]

    maps = {}
    for path in sorted(glob.glob(f"{out}/map_f*.txt")):
        fr = int(os.path.basename(path)[5:11])
        lines = open(path).read().splitlines()
        x0, y0, w, h = map(int, lines[0].split())
        g = lines[1:]
        at = lambda x, y: g[y - y0][x - x0] if 0 <= y - y0 < h and 0 <= x - x0 < w else "#"
        # Open under ground joined to the nest's column.
        opn = lambda c: c in ".abf~"
        seen = set()
        q = deque()
        for y in range(ground_y + 1, ground_y + 6):
            for x in range(nest_x - 3, nest_x + 4):
                if opn(at(x, y)):
                    seen.add((x, y))
                    q.append((x, y))
        while q:
            x, y = q.popleft()
            for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                n = (x + dx, y + dy)
                if n not in seen and n[1] > ground_y and opn(at(*n)):
                    seen.add(n)
                    q.append(n)
        under = Counter(at(x, y) for x, y in seen)
        mound = sum(1 for y in range(y0, ground_y + 1) for x in range(nest_x - 40, nest_x + 41) if at(x, y) in "#s")
        # Ground above the old line in the flat box is only the colony's heap (row ground_y is the old surface).
        mound_cells = sum(1 for y in range(y0, ground_y) for x in range(nest_x - 40, nest_x + 41) if at(x, y) in "#s")
        top = min((y for y in range(y0, ground_y) for x in range(nest_x - 40, nest_x + 41) if at(x, y) in "#s"), default=ground_y)
        maps[fr] = {
            "nest_open": len(seen), "deepest_row": max((y - ground_y for _, y in seen), default=0),
            "brood_under": under["b"], "food_under": under["f"], "ants_under": under["a"], "water_under": under["~"],
            "mound_cells": mound_cells, "mound_height": ground_y - top, "_unused": mound,
        }
        maps[fr].pop("_unused")

    keys = ["ants", "workers", "at_nest", "at_mound_in", "at_mound_top", "at_food", "at_surface", "food_in_crop", "soil_held", "home", "energy_median"]
    print("frame  " + " ".join(f"{k:>12}" for k in keys))
    for fr in sorted(samples):
        if fr % 10000 == 0:
            print(f"{fr:6d} " + " ".join(f"{samples[fr][k]:>12.0f}" for k in keys))
    print()
    for fr, m in sorted(maps.items()):
        print(f"{fr:6d} " + ", ".join(f"{k} {v}" for k, v in m.items()))
    print()
    for fr in sorted(deaths):
        print(f"deaths {fr:6d}-: " + ", ".join(f"{k} {v}" for k, v in deaths[fr].most_common()))
    json.dump({"samples": {fr: dict(s) for fr, s in samples.items()}, "maps": maps, "deaths": {fr: dict(c) for fr, c in deaths.items()},
               "nest_x": nest_x, "food_x": food_x, "ground_y": ground_y}, open(f"{out}/world.json", "w"))


if __name__ == "__main__":
    main()
