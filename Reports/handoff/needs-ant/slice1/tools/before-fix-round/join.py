#!/usr/bin/env python3
"""Join the walk trace (walk.csv.gz) with the full tick trace (ticks.csv.gz) for traced ids.
Writes a pickle-free TSV of joined rows: one per decision of each id, with walk and tick fields.
usage: join.py RUNDIR OUT.tsv [from_frame] [to_frame]"""
import csv, gzip, sys
run, out = sys.argv[1], sys.argv[2]
f0 = int(sys.argv[3]) if len(sys.argv) > 3 else 0
f1 = int(sys.argv[4]) if len(sys.argv) > 4 else 10**9
ids = set()
walk = {}
with gzip.open(f"{run}/walk.csv.gz", "rt") as fh:
    r = csv.DictReader(fh)
    for row in r:
        fr = int(row["frame"])
        if fr < f0 or fr > f1:
            continue
        walk[(fr, row["id"])] = row
tickf = ["frame","id","age","zone","hx","hy","energy_j","crop_cells","crop_j","spoil","since_nest","hungry_home","lunch","nest_bound",
         "i_Energy","i_AtNest","i_CarryingFood","i_Carrying","o_Move","o_Drop","o_Feed","o_DropSpoil","o_Dig","p_move","roll_move","moved","drive",
         "outcome","drop","pick","trip_load","d_bites","d_deliveries","d_crop_cells","d_crop_j","spoil_after","crop_mat","since_trip"]
wf = ["drive","job","hunger","hold","forage","threshold","depth","pref","p_move","stall","target_x","target_y"]
n = 0; miss = 0
with gzip.open(f"{run}/ticks.csv.gz", "rt") as fh, open(out, "w") as o:
    r = csv.DictReader(fh)
    o.write("\t".join(tickf + ["w_" + k for k in wf]) + "\n")
    for row in r:
        fr = int(row["frame"])
        if fr < f0 or fr > f1:
            continue
        w = walk.get((fr + 1, row["id"]))
        if w is None:
            miss += 1
            wv = [""] * len(wf)
        else:
            wv = [w[k] for k in wf]
        o.write("\t".join([row.get(k, "") for k in tickf] + wv) + "\n")
        n += 1
print(f"rows {n}, ticks without a walk row {miss}, walk rows {len(walk)}", file=sys.stderr)
