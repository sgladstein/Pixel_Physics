"""dropchoice.py RUN... [--from F] [--to T] -- check C3: how often a soil pellet drop had a real choice between a cell
beside spoil and one without (deep trace lane, 2026-10-06). Needs deeptrace `drops=1` (drops.csv, mound.csv).

Every pellet put down (`dumped`, the world's `spoil_dumped`) goes one of four ways: beside the ant (the scan found a
cell that would hold it), up the column (`lifted`: no cell beside would hold it, so there is nothing to choose), the
lean drop, or beside a dying carrier's body. Only a drop beside the ant has candidates, and the engine counts, per
drop, the candidates and how many have spoil in reach; `discr` counts drops with some of each: a real choice.

The totals are the engine's own counters. The split of the other beside drops into "every candidate beside spoil"
and "none" needs one drop per frame, so it is taken over frames with exactly one dropper and nothing lifted, lean or
dead (the rest are booked as shared frames); the same frames give the zone the dropper stood in."""
import sys, csv, collections

def load(run, a, b):
    rows = []
    for r in csv.DictReader(open(f"{run}/drops.csv")):
        f = int(r["frame"])
        if a <= f <= b:
            rows.append(r)
    return rows

def main():
    args = sys.argv[1:]
    a, b = 100000, 300000
    if "--from" in args:
        i = args.index("--from"); a = int(args[i + 1]); del args[i:i + 2]
    if "--to" in args:
        i = args.index("--to"); b = int(args[i + 1]); del args[i:i + 2]
    keys = ["dumped", "lifted", "lean", "kept_no_lift", "cand", "by_spoil", "discr"]
    print(f"window {a}-{b}")
    for run in args:
        rows = load(run, a, b)
        T = {k: sum(int(r[k]) for r in rows) for k in keys}
        died = sum(int(r["died"]) for r in rows)
        dropper_n = sum(int(r["droppers"]) for r in rows)
        # one-drop frames
        single = collections.Counter(); zone_beside = collections.defaultdict(collections.Counter)
        shared_dumped = 0; shared_discr = 0; anomalies = 0; cand_hist = collections.Counter()
        for r in rows:
            d = {k: int(r[k]) for k in keys}
            n = int(r["droppers"]); dd = int(r["died"])
            who = [w.split(":") for w in r["who"].split(";")] if r["who"] else []
            if d["dumped"] == 0:
                continue
            if n == 1 and dd == 0 and d["dumped"] == 1:
                z = who[0][3]
                if d["lifted"] == 1:
                    single["lifted"] += 1; zone_beside[z]["lifted"] += 1
                elif d["lean"] == 1:
                    single["lean"] += 1; zone_beside[z]["lean"] += 1
                elif d["cand"] > 0:
                    c, s = d["cand"], d["by_spoil"]
                    kind = "none" if s == 0 else ("all" if s == c else "mixed")
                    single[kind] += 1; zone_beside[z][kind] += 1; cand_hist[c] += 1
                    if (kind == "mixed") != (d["discr"] == 1):
                        anomalies += 1
                else:
                    anomalies += 1
            else:
                shared_dumped += d["dumped"]; shared_discr += d["discr"]
        beside_total = T["dumped"] - T["lifted"] - T["lean"]  # includes dying carriers' pellets
        name = run.rstrip("/").split("/")[-1]
        print(f"\n== {name}: {T['dumped']} pellets put down; droppers seen {dropper_n}, carriers died holding {died}")
        print(f"   up the column (no cell beside to choose) {T['lifted']} ({100*T['lifted']/max(T['dumped'],1):.0f}%), lean drop {T['lean']}, "
              f"beside the ant or a dying carrier {beside_total} ({100*beside_total/max(T['dumped'],1):.0f}%)")
        print(f"   REAL CHOICE (some candidates beside spoil, some not): {T['discr']} = {100*T['discr']/max(T['dumped'],1):.2f}% of all drops, "
              f"{100*T['discr']/max(beside_total,1):.1f}% of drops beside the ant")
        print(f"   candidates: {T['cand']} cells over the beside drops (mean {T['cand']/max(beside_total,1):.2f}), "
              f"{T['by_spoil']} with spoil in reach ({100*T['by_spoil']/max(T['cand'],1):.1f}%)")
        sb = single["none"] + single["all"] + single["mixed"]
        print(f"   one-drop frames: beside {sb} (none beside spoil {single['none']}, all {single['all']}, mixed {single['mixed']}), "
              f"lifted {single['lifted']}, lean {single['lean']}; shared frames hold {shared_dumped} drops and {shared_discr} real choices; anomalies {anomalies}")
        print(f"   candidates per beside drop (one-drop frames): " + ", ".join(f"{c}:{n}" for c, n in sorted(cand_hist.items())))
        print("   by zone (one-drop frames): zone: beside none/all/mixed; lifted; lean")
        for z in sorted(zone_beside, key=lambda z: -sum(zone_beside[z].values())):
            c = zone_beside[z]
            print(f"     {z:10} {c['none']}/{c['all']}/{c['mixed']}; {c['lifted']}; {c['lean']}")
        # time profile
        prof = collections.defaultdict(lambda: [0, 0, 0])
        for r in load(run, 0, 10**9):
            w = int(r["frame"]) // 50000 * 50
            prof[w][0] += int(r["dumped"]); prof[w][1] += int(r["lifted"]); prof[w][2] += int(r["discr"])
        print("   by 50k (whole run): " + "  ".join(f"{w}k: {p[0]} put down, {p[1]} lifted, {p[2]} choice" for w, p in sorted(prof.items())))
        try:
            M = {int(r["frame"]): r for r in csv.DictReader(open(f"{run}/mound.csv"))}
            print("   mound above the old ground (spoil / soil / packedsoil): " + "  ".join(
                f"{f//1000}k {M[f]['spoil']}/{M[f]['soil']}/{M[f]['packedsoil']}" for f in (50000, 100000, 200000, 300000) if f in M))
        except FileNotFoundError:
            pass

if __name__ == "__main__":
    main()
