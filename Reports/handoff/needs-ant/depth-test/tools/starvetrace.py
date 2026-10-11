"""starvetrace.py RUN [--from A] [--to B] [--sample N]: who starved in the window, where, and in their last hunger which
pull they were on and what they sensed (redesign thread, depth test, 2026-10-06).

Needs a deeptrace `hungry=1` run: events.txt (DIED lines, FOUNDED), ledger.csv, colony.csv, hungry.csv.gz.
Ids are the full organism id (gen<<20 | slot) in every file; a slot is reused after a death, so an ant is (id, born =
frame - age).

Where they died, by the head's column against the door (nest_x) and the food (food_x, from FOUNDED):
  in the nest (zone nest) / at the food (within 12 columns of food_x) / east of the door / at the door (within 6) /
  west, near (6-60 columns) / west, far (more than 60 columns).
Last hunger = the ant's last unbroken run of hungry rows (gap <= 25 frames), as scripts/deeptrace_hunger.py spells.
Pull, scout state, trail and steps are counted over that spell's rows; 'scouting' = scout_w > 0, 'given up' =
scout_home 1. Trail: route = trail B under the picked heading, b6e/b6w = six cells east/west. Steps are rows that moved,
by the sign of hx_after - hx.
"""
import sys, csv, gzip, re, collections, statistics, subprocess, os

sys.path.insert(0, "/home/claude/Pixel_Physics/scripts")
import deeptrace_hunger as H  # noqa: E402

args = sys.argv[1:]
A, B, NS = 240_000, 300_000, 8
for flag in ("--from", "--to", "--sample"):
    if flag in args:
        i = args.index(flag)
        v = int(args[i + 1])
        del args[i:i + 2]
        A, B, NS = (v, B, NS) if flag == "--from" else ((A, v, NS) if flag == "--to" else (A, B, v))
run = args[0]
nest_x, food_x, ground_y = H.geo(run)


def where(x, zone):
    if zone == "nest":
        return "in the nest"
    if abs(x - food_x) <= 12:
        return "at the food"
    d = x - nest_x
    if abs(d) < 6:
        return "at the door"
    if d > 0:
        return "east of the door"
    return "west, near (6-60)" if d >= -60 else "west, far (>60)"


ORDER = ["in the nest", "at the door", "west, near (6-60)", "west, far (>60)", "east of the door", "at the food"]

dead = {}
for line in open(f"{run}/events.txt"):
    p = line.split()
    if len(p) < 3 or p[1] != "DIED" or "cause=STARVED" not in line:
        continue
    f = int(p[0])
    if not (A <= f <= B):
        continue
    kv = dict(t.split("=", 1) for t in p[2:] if "=" in t)
    x, y = map(int, re.match(r"\((-?\d+),(-?\d+)\)", kv["at"]).groups())
    k = (int(kv["id"]), f - int(kv["age"]))
    dead[k] = dict(frame=f, x=x, y=y, zone=kv["zone"], age=int(kv["age"]), where=where(x, kv["zone"]))
led = {(int(r["id"]), int(r["born"])): r for r in csv.DictReader(open(f"{run}/ledger.csv"))}
for k, d in dead.items():
    d["led"] = led.get(k, {})

print(f"== {run}: {len(dead)} starved {A // 1000}-{B // 1000}k (door x={nest_x}, food x={food_x}, food is "
      f"{'east' if food_x > nest_x else 'west'})")
byw = collections.Counter(d["where"] for d in dead.values())
print("  where they died: " + ", ".join(f"{w} {byw[w]}" for w in ORDER if byw[w]))
nb = collections.Counter((d["where"], d["led"].get("nb", "?")) for d in dead.values())
print("  of them nest-bound (ledger nb=1): " + ", ".join(f"{w} {nb[w, '1']}" for w in ORDER if byw[w]))

# colony.csv: positions over the last 40k frames of each starver
slots = collections.defaultdict(list)
for k in dead:
    slots[k[0]].append(k)
hist = collections.defaultdict(list)
for r in csv.DictReader(open(f"{run}/colony.csv")):
    f = int(r["frame"])
    if f < A - 60_000:
        continue
    s = int(r["id"])
    for k in slots.get(s, ()):
        if k[1] <= f <= dead[k]["frame"]:
            hist[k].append((f, int(r["hx"]), int(r["hy"]), r["zone"], int(r["crop_cells"]), r["spoil"], float(r["energy_j"]), r["worker"]))

# hungry.csv.gz rows of the starvers, prefiltered by awk on id:born
keyf = f"/tmp/claude-0/-home-claude-Pixel-Physics/c8162b93-2657-592f-b840-597b957aa530/scratchpad/depth/.keys-{os.getpid()}"
with open(keyf, "w") as fh:
    for k in dead:
        fh.write(f"{k[0]}:{k[1]}\n")
cmd = (f"zcat {run}/hungry.csv.gz | awk -F, 'NR==FNR{{w[$1]=1;next}} FNR==1{{print;next}} "
       f"$1>={A - 80_000} && (($2\":\"($1-$3)) in w)' {keyf} -")
proc = subprocess.run(["bash", "-c", "set -o pipefail; " + cmd], capture_output=True, text=True)
os.unlink(keyf)
rows = collections.defaultdict(list)
rd = csv.DictReader(proc.stdout.splitlines())
for r in rd:
    if r.get("bite") is None:
        continue
    f = int(r["frame"])
    rows[(int(r["id"]), f - int(r["age"]))].append(r)

# last hunger spell
for k, d in dead.items():
    rs = rows.get(k, [])
    sp = []
    for r in reversed(rs):
        if sp and int(sp[-1]["frame"]) - int(r["frame"]) > H.GAP:
            break
        sp.append(r)
    sp.reverse()
    d["spell"] = sp


def summarize(group, title):
    ks = [k for k in dead if dead[k]["where"] in group]
    if not ks:
        return
    print(f"\n  -- {title}: {len(ks)} ants")
    L = [dead[k]["led"] for k in ks]
    cnt = lambda p: sum(1 for x in L if x and p(x))
    print(f"     ledger: nest-bound {cnt(lambda x: x['nb'] == '1')}, ever foraged {cnt(lambda x: x['foraged'] == '1')}, "
          f"ever at the food {cnt(lambda x: int(x['last_heap']) > 0)}, ever delivered {cnt(lambda x: int(x['deliveries']) > 0)}; "
          f"age at death median {statistics.median(dead[k]['age'] for k in ks):.0f} frames")
    sps = [dead[k]["spell"] for k in ks if dead[k]["spell"]]
    if not sps:
        print("     no hunger rows")
        return
    lens = [int(s[-1]["frame"]) - int(s[0]["frame"]) for s in sps]
    x0 = [int(s[0]["hx"]) - nest_x for s in sps]
    z0 = collections.Counter(s[0]["zone"] for s in sps)
    e0 = [H.num(s[0]["e"]) for s in sps]
    print(f"     last hunger: {len(sps)} with rows; length median {statistics.median(lens):.0f} frames; began at column "
          f"(door = 0) median {statistics.median(x0):+.0f} (range {min(x0):+d} to {max(x0):+d}); began in "
          + ", ".join(f"{z} {n}" for z, n in z0.most_common()) + f"; energy then median {statistics.median(e0):.2f} of the grant")
    xmin = [min(int(r["hx"]) for r in s) - nest_x for s in sps]
    xmax = [max(int(r["hx"]) for r in s) - nest_x for s in sps]
    cross = sum(1 for a, b in zip(xmin, xmax) if a <= H.DOOR_HALF and b >= -H.DOOR_HALF)
    anch = collections.Counter()
    for s in sps:
        ax = [int(r["anchor_x"]) for r in s]
        anch["home point moved" if len(set((r["anchor_x"], r["anchor_y"]) for r in s)) > 1 else "home point fixed"] += 1
        anch["home point at the door (within 6)" if abs(ax[-1] - nest_x) < 6 else "home point away from the door"] += 1
    print(f"     during it: crossed the door's column {cross}; furthest east reached median {statistics.median(xmax):+.0f}; "
          + ", ".join(f"{a} {n}" for a, n in sorted(anch.items())))
    pulls, scout, trail, steps, outc, door, zone = (collections.Counter() for _ in range(7))
    for s in sps:
        for r in s:
            pulls[r["pull"]] += 1
            outc[r["outcome"]] += 1
            zone[r["zone"]] += 1
            if r["pull"] != "not scored":
                scout["given up" if r["scout_home"] == "1" else ("scouting" if H.num(r["scout_w"]) > 0 else "no scout")] += 1
                door[r["door"]] += 1
                rt, be, bw = H.num(r["route"]), H.num(r["b6e"]), H.num(r["b6w"])
                trail["on a trail (route > 0.5)" if rt > 0.5 else "off a trail"] += 1
                if be == be and bw == bw:
                    trail["trail stronger 6 east than 6 west" if be > bw else ("stronger west" if bw > be else "east = west")] += 1
            if r["moved"] == "1" and r["hx_after"] != "":
                dx = int(r["hx_after"]) - int(r["hx"])
                steps["east" if dx > 0 else ("west" if dx < 0 else "up/down")] += 1
    tot = lambda c: sum(c.values()) or 1
    pct = lambda c, n=8: ", ".join(f"{k} {100 * v / tot(c):.0f}%" for k, v in c.most_common(n))
    print(f"     decisions {tot(pulls)}: pull {pct(pulls)}")
    print(f"     zone {pct(zone)}")
    print(f"     scored decisions: scout {pct(scout)}; door read {pct(door, 5)}")
    print(f"     trail: " + ", ".join(f"{k} {100 * v / tot(collections.Counter({k: v for k, v in trail.items() if k.startswith(('on', 'off'))})):.0f}%" for k, v in trail.items() if k.startswith(("on", "off")))
          + "; " + ", ".join(f"{k} {100 * v / max(1, sum(v2 for k2, v2 in trail.items() if not k2.startswith(('on', 'off')))):.0f}%" for k, v in trail.items() if not k.startswith(("on", "off"))))
    print(f"     steps {tot(steps)}: {pct(steps)}; outcomes {pct(outc, 5)}")
    c = collections.Counter()
    for s in sps:
        for r in s:
            H.split(r, c)
    print("     empty steps with no other pull, by side of the home point (deeptrace_hunger's walk split):")
    H.print_split(c, "       ")


summarize({"west, far (>60)", "west, near (6-60)"}, "died west of the door")
summarize({"east of the door", "at the food", "at the door"}, "died at or east of the door")
summarize({"in the nest"}, "died in the nest")

# sample stories: every k-th west death
west = sorted((k for k in dead if dead[k]["where"].startswith("west")), key=lambda k: dead[k]["frame"])
step = max(1, len(west) // NS)
print(f"\n  -- sample of {min(NS, len(west))} west deaths (every {step}th by death frame); columns relative to the door")
for k in west[::step][:NS]:
    d, L, sp, h = dead[k], dead[k]["led"], dead[k]["spell"], hist.get(k, [])
    print(f"   ant {k[0]} born {k[1]}, starved {d['frame']} at column {d['x'] - nest_x:+d} ({d['zone']}), age {d['age']}; "
          f"nest-bound {L.get('nb')}, foraged {L.get('foraged')}, deliveries {L.get('deliveries')}, decisions surface/mound/nest/heap "
          f"{L.get('surface')}/{L.get('mound')}/{L.get('nest')}/{L.get('heap')}, last fed frame {L.get('last_fed')}")
    pts = [p for p in h if p[0] in {d["frame"] - d["frame"] % 1000 - j * 5000 for j in range(0, 9)}]
    print("     census (frame: column, zone, crop, energy J): " + "; ".join(f"{p[0] // 1000}k: {p[1] - nest_x:+d} {p[3]} c{p[4]} {p[6]:.0f}" for p in pts))
    if sp:
        pl = collections.Counter(r["pull"] for r in sp)
        sc = collections.Counter("given up" if r["scout_home"] == "1" else ("scouting" if H.num(r["scout_w"]) > 0 else "none") for r in sp if r["pull"] != "not scored")
        st = collections.Counter(("E" if int(r["hx_after"]) > int(r["hx"]) else ("W" if int(r["hx_after"]) < int(r["hx"]) else "v")) for r in sp if r["moved"] == "1" and r["hx_after"] != "")
        print(f"     last hunger {sp[0]['frame']}-{sp[-1]['frame']}: began column {int(sp[0]['hx']) - nest_x:+d} {sp[0]['zone']} e={sp[0]['e']}, "
              f"home point {int(sp[0]['anchor_x']) - nest_x:+d} -> {int(sp[-1]['anchor_x']) - nest_x:+d}; pulls {dict(pl.most_common(4))}; scout {dict(sc)}; steps {dict(st)}")
