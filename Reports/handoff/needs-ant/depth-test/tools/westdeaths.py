"""westdeaths.py RUN... [--from A] [--to B]: per run, the colony's peak and where its starvers died, against how many
ants stand far west at a time (redesign thread, depth test, 2026-10-06).

From events.txt (FOUNDED, DIED ... cause=STARVED), stats.csv (ants) and colony.csv (every ant every 1k). Where, by the
head's column against the door: nest (zone nest), door (within 6), west near (6-60 columns), west far (more than 60),
east (past 6, not at the food), food (within 12 of food_x). Window default 20k-300k. "After peak" = within 60k frames
after the frame of the most ants. "Far west at a time" = mean ants per census frame more than 60 columns west of the door
(any zone), over the window.
"""
import sys, re, csv, collections

args = sys.argv[1:]
A, B = 20_000, 300_000
for flag in ("--from", "--to"):
    if flag in args:
        i = args.index(flag)
        v = int(args[i + 1])
        del args[i:i + 2]
        A, B = (v, B) if flag == "--from" else (A, v)
COLS = ["nest", "door", "west near", "west far", "east", "food"]
print(f"window {A // 1000}-{B // 1000}k; starved by where they died: " + " / ".join(COLS))
for run in args:
    nest_x = food_x = None
    deaths = []
    for line in open(f"{run}/events.txt"):
        p = line.split()
        if len(p) > 2 and p[1] == "FOUNDED":
            kv = dict(t.split("=") for t in p[2:] if "=" in t)
            nest_x, food_x = int(kv["nest_x"]), int(kv["food_x"])
        elif len(p) > 2 and p[1] == "DIED" and "cause=STARVED" in line:
            f = int(p[0])
            if A <= f <= B:
                kv = dict(t.split("=", 1) for t in p[2:] if "=" in t)
                x = int(re.match(r"\((-?\d+),", kv["at"]).group(1))
                deaths.append((f, x, kv["zone"]))
    ants = {int(r["frame"]): int(r["ants"]) for r in csv.DictReader(open(f"{run}/stats.csv"))}
    win = {f: a for f, a in ants.items() if A <= f <= B}
    pf = max(win, key=lambda f: win[f])

    def where(x, zone):
        if zone == "nest":
            return "nest"
        if abs(x - food_x) <= 12:
            return "food"
        d = x - nest_x
        return "door" if abs(d) < 6 else ("east" if d > 0 else ("west near" if d >= -60 else "west far"))

    c = collections.Counter(where(x, z) for _, x, z in deaths)
    cp = collections.Counter(where(x, z) for f, x, z in deaths if pf <= f <= pf + 60_000)
    far = collections.Counter()
    frames = set()
    for r in csv.DictReader(open(f"{run}/colony.csv")):
        f = int(r["frame"])
        if A <= f <= B:
            frames.add(f)
            if int(r["hx"]) - nest_x < -60:
                far[f] += 1
    k = max(1, len(frames))
    farmean = sum(far.values()) / k
    farmax = max(far.values(), default=0)
    print(f"{run}: peak {win[pf]} ants at {pf // 1000}k (ants at {B // 1000}k: {ants.get(max(f for f in ants if f <= B))}); "
          f"starved {len(deaths)}: " + " / ".join(str(c[w]) for w in COLS)
          + f"; in the 60k after the peak {sum(cp.values())}: " + " / ".join(str(cp[w]) for w in COLS)
          + f"; ants far west at a time mean {farmean:.1f}, most {farmax}")
