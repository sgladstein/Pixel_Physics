#!/usr/bin/env python3
"""For decisions where the walk's drive is Forage: which shipped gate set p_move.
usage: forage_gate.py joined.tsv [from] [to]"""
import csv, sys, collections
f0 = int(sys.argv[2]) if len(sys.argv) > 2 else 0
f1 = int(sys.argv[3]) if len(sys.argv) > 3 else 10**9
rows = list(csv.DictReader(open(sys.argv[1]), delimiter="\t"))
def fnum(s, d=float("nan")):
    try: return float(s)
    except: return d
cls = collections.Counter(); per = collections.defaultdict(collections.Counter)
zone = collections.Counter(); stepped = collections.Counter()
for r in rows:
    fr = int(r["frame"])
    if fr < f0 or fr > f1 or r["w_drive"] != "forage":
        continue
    d = r["drive"]; p = fnum(r["p_move"]); mv = fnum(r["o_Move"])
    if d in ("", "NaN", "nan"):
        why = "felt filter (crop/spoil/hungry_home/not sensed empty)"
        if r["hungry_home"] == "1": why = "felt: hungry_home"
        elif fnum(r["crop_cells"], 0) > 0 and r["lunch"] != "1": why = "felt: crop not empty"
        elif r["spoil"] not in ("0", ""): why = "felt: spoil"
        elif fnum(r["i_CarryingFood"], 0) > 0: why = "felt: sensed carrying food"
    elif fnum(d) == 0.0:
        why = "drive 0 (not foraged / nest-bound)"
    elif abs(p - max(0.0, min(1.0, mv))) < 1e-4:
        why = "drive>0 but no lift (felt >= energy in)"
    else:
        why = "paced"
    cls[why] += 1; per[r["id"]][why] += 1
    zone[(why, r["zone"])] += 1
    stepped[(why, r["moved"])] += 1
tot = sum(cls.values())
print(f"forage-drive decisions {tot} in [{f0},{f1}]")
for k, v in cls.most_common():
    s1 = stepped[(k, "1")]
    print(f"  {v:6d} {100*v/tot:5.1f}%  stepped {100*s1/max(v,1):5.1f}%  {k}")
print("by zone:")
for (k, z), v in sorted(zone.items(), key=lambda t: -t[1])[:12]:
    print(f"  {v:6d}  {z:10s} {k}")
print("per ant:")
for i, c in sorted(per.items(), key=lambda t: -sum(t[1].values())):
    print(f"  {i:>8s} " + ", ".join(f"{k.split(' ')[0]}{'' if not k.startswith('felt') else ':'+k.split(': ')[-1] if ': ' in k else ''}={v}" for k, v in c.most_common()))
