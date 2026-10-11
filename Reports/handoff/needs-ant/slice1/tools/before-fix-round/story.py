#!/usr/bin/env python3
"""story.py joined.tsv ID [from] [to]: one ant's life under the walk as spells of the same
(drive, job, zone, load) with how many decisions, the share it stepped, step-chance median,
energy at start/end, and events (bites, deliveries, digs, pellet up/down) in the spell."""
import csv, sys, statistics
path, want = sys.argv[1], sys.argv[2]
f0 = int(sys.argv[3]) if len(sys.argv) > 3 else 0
f1 = int(sys.argv[4]) if len(sys.argv) > 4 else 10**9
rows = [r for r in csv.DictReader(open(path), delimiter="\t") if r["id"] == want and f0 <= int(r["frame"]) <= f1]
def load(r):
    s = []
    if r["spoil"] not in ("0", ""): s.append("pellet")
    try:
        if float(r["crop_cells"]) > 0: s.append("crop" + ("(lunch)" if r["lunch"] == "1" else ""))
    except: pass
    return "+".join(s) or "-"
spells = []
for r in rows:
    key = (r["w_drive"] or "?", r["w_job"] or "?", r["zone"], load(r))
    if not spells or spells[-1]["key"] != key:
        spells.append({"key": key, "rows": []})
    spells[-1]["rows"].append(r)
def f(x, d=0.0):
    try: return float(x)
    except: return d
print(f"ant {want}: {len(rows)} decisions, frames {rows[0]['frame'] if rows else '-'}..{rows[-1]['frame'] if rows else '-'}")
print(f"{'from':>6} {'to':>6} {'n':>4} {'drive':>7} {'job':>6} {'zone':>9} {'load':>12} {'step%':>5} {'p50':>5} {'E0':>6} {'E1':>6} {'hung':>5} events")
for s in spells:
    rs = s["rows"]; d, j, z, l = s["key"]
    st = sum(1 for r in rs if r["moved"] == "1") / len(rs)
    pm = statistics.median(f(r["p_move"]) for r in rs)
    ev = []
    for r in rs:
        if f(r["d_bites"]) > 0: ev.append(f"bite@{r['frame']}")
        if f(r["d_deliveries"]) > 0: ev.append(f"deliv@{r['frame']}")
        if r["spoil"] in ("0", "") and r["spoil_after"] not in ("0", ""): ev.append(f"pellet-up@{r['frame']}")
        if r["spoil"] not in ("0", "") and r["spoil_after"] in ("0", ""): ev.append(f"pellet-down@{r['frame']}")
    evs = " ".join(ev[:6]) + (f" (+{len(ev)-6})" if len(ev) > 6 else "")
    print(f"{rs[0]['frame']:>6} {rs[-1]['frame']:>6} {len(rs):>4} {d:>7} {j:>6} {z:>9} {l:>12} {100*st:>5.0f} {pm:>5.2f} {f(rs[0]['energy_j']):>6.0f} {f(rs[-1]['energy_j']):>6.0f} {f(rs[-1]['w_hunger']):>5.2f} {evs}")
