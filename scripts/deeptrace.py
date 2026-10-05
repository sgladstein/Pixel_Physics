#!/usr/bin/env python3
"""Read `examples/deeptrace` output into per-ant lives and colony-wide budgets.

    python3 scripts/deeptrace.py OUT [--ants ID,ID,...] [--from F] [--to F] [--window W] [--stories N]

One pass over OUT/ticks.csv.gz (every row is one decision of one ant, or a
frame in which something happened to it). Writes beside the input:

  ants.csv         one line per ant seen: life span, worker flag, how it
                   died (events.txt), time budget by activity, what it did
                   (cuts, bites, pickups, deliveries, food handed on, soil put
                   down), how it walked, and outside steps toward / away from
                   the food (empty) or the nest (laden).
  windows.txt      the whole colony's time budget and acts per `--window`
                   frames (25,000): what the colony as a whole spends its
                   decisions on, and how that drifts.
  story_<id>.txt   for `--ants` (or the `--stories` longest-lived ants, plus
                   as many picked evenly by id): the life as episodes, one
                   line per stretch of one activity, with what changed in its
                   senses at the switch.

Activities, in world words: carry_soil, carry_food@<place>, empty@<place>
(digs are counted inside the empty stretch they happen in); places: nest
(under the old ground line), mound (in or on the spoil mound, 40 columns
either side of the nest), food (on/beside the heap), surface (anywhere else).
"""
import csv
import gzip
import sys
from collections import Counter, defaultdict

ZONE_GROUP = {"nest": "nest", "mound_in": "mound", "mound_top": "mound", "food": "food", "surface": "surface"}
WATCH = ["i_AtNest", "i_FoodAdjacent", "i_Energy", "i_Carrying", "i_CarryingFood", "i_Crowding", "i_PheroBAlong",
         "i_PheroAAlong", "i_HomeAligned", "i_KinNeed", "i_Stillness", "i_SurfaceCurvature", "i_LightHere"]


def num(v, d=0.0):
    try:
        return float(v)
    except (TypeError, ValueError):
        return d


def main():
    out = sys.argv[1]
    opt = dict(zip(sys.argv[2::2], sys.argv[3::2]))
    want = set(opt["--ants"].split(",")) if "--ants" in opt else None
    lo, hi = int(opt.get("--from", 0)), int(opt.get("--to", 10**12))
    window = int(opt.get("--window", 25000))
    n_stories = int(opt.get("--stories", 4))

    food_x = nest_x = None
    died = {}
    for e in open(f"{out}/events.txt"):
        p = e.split()
        if len(p) > 1 and p[1] == "FOUNDED":
            kv = dict(t.split("=") for t in p[2:] if "=" in t)
            food_x, nest_x = int(kv["food_x"]), int(kv["nest_x"])
        elif len(p) > 1 and p[1] == "DIED":
            kv = dict(t.split("=", 1) for t in p[2:] if "=" in t)
            died[kv["id"]] = (int(p[0]), kv.get("cause", "?"), kv.get("zone", "?"))

    ants = defaultdict(lambda: {"first": None, "last": None, "worker": "", "budget": Counter(), "acts": Counter(),
                                "walk": Counter(), "steps": Counter(), "rows": 0})
    win = defaultdict(lambda: {"budget": Counter(), "acts": Counter(), "ants": set(), "steps": Counter()})
    # Stories need a second look at chosen ants; keep their rows' episodes as we go when chosen up front.
    eps = defaultdict(list)
    cur = {}
    prev = {}

    with gzip.open(f"{out}/ticks.csv.gz", "rt") as fh:
        rd = csv.reader(fh)
        h = next(rd)
        ix = {k: i for i, k in enumerate(h)}
        I = lambda name: ix[name]
        c_frame, c_id, c_worker, c_hx, c_hy, c_zone = I("frame"), I("id"), I("worker"), I("hx"), I("hy"), I("zone")
        c_spoil, c_crop, c_ddigs, c_dbites, c_ddel, c_dcrop = I("spoil"), I("crop_cells"), I("d_digs"), I("d_bites"), I("d_deliveries"), I("d_crop_cells")
        c_spoil_after, c_hungry, c_outcome, c_row, c_hxa, c_ahead, c_moved = I("spoil_after"), I("hungry_larvae"), I("outcome"), I("row"), I("hx_after"), I("ahead"), I("moved")
        c_energy, c_dig = I("energy_j"), I("o_Dig")
        watch_ix = [(k[2:], ix[k]) for k in WATCH if k in ix]
        for r in rd:
            fr = int(r[c_frame])
            if fr < lo or fr > hi:
                continue
            aid = r[c_id]
            a = ants[aid]
            if a["first"] is None:
                a["first"] = fr
                a["worker"] = r[c_worker]
            a["last"] = fr
            a["rows"] += 1
            wd = win[fr // window]
            wd["ants"].add(aid)
            z = ZONE_GROUP.get(r[c_zone], r[c_zone])
            if r[c_spoil] not in ("", "0"):
                act = "carry_soil"
            elif r[c_crop] not in ("", "0"):
                act = f"carry_food@{z}"
            else:
                act = f"empty@{z}"
            decided = r[c_row] == "1"
            if decided:
                a["budget"][act] += 1
                wd["budget"][act] += 1
                a["walk"][r[c_outcome]] += 1
            acts = Counter()
            if r[c_ddigs] not in ("", "0"):
                acts["cut"] += int(r[c_ddigs])
            if r[c_dbites] not in ("", "0"):
                acts["bite"] += int(r[c_dbites])
            if r[c_ddel] not in ("", "0"):
                acts["delivery"] += int(r[c_ddel])
            dc = int(r[c_dcrop]) if r[c_dcrop] else 0
            if dc > 0:
                acts["food_picked"] += dc
            elif dc < 0 and r[c_ddel] in ("", "0"):
                acts["food_out_other"] += -dc
                if r[c_hungry] not in ("", "0"):
                    acts["food_out_by_hungry_larva"] += -dc
            if r[c_spoil] not in ("", "0") and r[c_spoil_after] == "0":
                acts["soil_put_down"] += 1
            if decided and r[c_hungry] not in ("", "0"):
                acts["touching_hungry_larva"] += 1
                if r[c_crop] not in ("", "0"):
                    acts["touching_hungry_larva_with_food"] += 1
            if acts:
                a["acts"].update(acts)
                wd["acts"].update(acts)
            # Outside steps: empty toward the food, laden toward the nest.
            if r[c_outcome] == "stepped" and r[c_hxa] and food_x is not None and z in ("surface", "mound"):
                dx = int(r[c_hxa]) - int(r[c_hx])
                if dx:
                    if act.startswith("empty"):
                        k = "empty_toward_food" if (food_x - int(r[c_hx])) * dx > 0 else "empty_away_from_food"
                    elif act.startswith("carry_food"):
                        k = "laden_toward_nest" if (nest_x - int(r[c_hx])) * dx > 0 else "laden_away_from_nest"
                    else:
                        k = None
                    if k:
                        a["steps"][k] += 1
                        wd["steps"][k] += 1
            # Episodes for chosen ants.
            if want is not None and aid in want:
                c = cur.get(aid)
                if c is None or c["act"] != act:
                    if c is not None:
                        eps[aid].append(c)
                    why = []
                    p = prev.get(aid)
                    if p is not None:
                        for name, i in watch_ix:
                            d = num(r[i]) - num(p[i])
                            if abs(d) >= 0.2:
                                why.append(f"{name} {num(p[i]):.2f}->{num(r[i]):.2f}")
                    c = cur[aid] = {"act": act, "start": fr, "end": fr, "n": 0, "acts": Counter(), "moved": 0,
                                    "at": (r[c_hx], r[c_hy]), "energy": r[c_energy], "dig_p": r[c_dig], "why": why, "ahead": Counter()}
                c["end"] = fr
                if decided:
                    c["n"] += 1
                    c["moved"] += int(r[c_moved] or 0)
                    c["ahead"][r[c_ahead]] += 1
                c["acts"].update(acts)
                prev[aid] = r

    # ants.csv
    acts_keys = ["cut", "bite", "food_picked", "delivery", "food_out_other", "food_out_by_hungry_larva", "soil_put_down",
                 "touching_hungry_larva", "touching_hungry_larva_with_food"]
    bud_keys = sorted({k for a in ants.values() for k in a["budget"]})
    walk_keys = ["stepped", "roll_failed_idle", "roll_failed_tumbled", "blocked_tumbled", "fell", "swapped", "reversed", "crossing"]
    step_keys = ["empty_toward_food", "empty_away_from_food", "laden_toward_nest", "laden_away_from_nest"]
    with open(f"{out}/ants.csv", "w", newline="") as fh:
        w = csv.writer(fh)
        w.writerow(["id", "worker", "first", "last", "decisions", "died_frame", "cause", "died_zone"] + [f"t_{k}" for k in bud_keys]
                   + acts_keys + [f"w_{k}" for k in walk_keys] + step_keys)
        for aid, a in sorted(ants.items(), key=lambda kv: int(kv[0])):
            n = max(sum(a["budget"].values()), 1)
            d = died.get(aid, ("", "", ""))
            w.writerow([aid, a["worker"], a["first"], a["last"], sum(a["budget"].values()), *d]
                       + [round(a["budget"][k] / n, 3) for k in bud_keys] + [a["acts"][k] for k in acts_keys]
                       + [round(a["walk"][k] / n, 3) for k in walk_keys] + [a["steps"][k] for k in step_keys])

    # windows.txt
    lines = []
    for k in sorted(win):
        wd = win[k]
        n = max(sum(wd["budget"].values()), 1)
        s = wd["steps"]
        e = s["empty_toward_food"] + s["empty_away_from_food"]
        l = s["laden_toward_nest"] + s["laden_away_from_nest"]
        lines.append(f"frames {k*window}-{(k+1)*window-1}: {len(wd['ants'])} ants, {n} decisions")
        lines.append("  time: " + ", ".join(f"{a} {v/n:.1%}" for a, v in wd["budget"].most_common()))
        lines.append("  acts: " + ", ".join(f"{a} {v}" for a, v in sorted(wd["acts"].items())))
        lines.append(f"  outside steps: empty away from food {s['empty_away_from_food']/max(e,1):.0%} of {e}; laden away from nest {s['laden_away_from_nest']/max(l,1):.0%} of {l}")
    open(f"{out}/windows.txt", "w").write("\n".join(lines) + "\n")
    print("\n".join(lines))

    for aid, el in eps.items():
        if aid in cur:
            el.append(cur[aid])
        with open(f"{out}/story_{aid}.txt", "w") as st:
            d = died.get(aid)
            st.write(f"ant {aid}: frames {ants[aid]['first']}-{ants[aid]['last']}, worker {ants[aid]['worker']}, died {d}\n")
            for ep in el:
                ah = ",".join(f"{k} {v*100//max(ep['n'],1)}%" for k, v in ep["ahead"].most_common(2))
                did = " ".join(f"{k}={v}" for k, v in sorted(ep["acts"].items()) if not k.startswith("touching"))
                st.write(f"{ep['start']:>7}-{ep['end']:<7} {ep['act']:20s} {ep['n']:5d} dec, moved {ep['moved']*100//max(ep['n'],1):3d}%"
                         f" at {ep['at'][0]},{ep['at'][1]} E {ep['energy']:>7} dig_p {ep['dig_p']:>5} ahead {ah}"
                         f"{' | did ' + did if did else ''}{' | switch: ' + '; '.join(ep['why']) if ep['why'] else ''}\n")


if __name__ == "__main__":
    main()
