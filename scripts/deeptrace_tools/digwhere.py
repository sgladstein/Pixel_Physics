"""digwhere.py RUN... -- where the ground changed between consecutive maps (1000 frames apart), 100-300k.
Below the old ground line: cells that went ground->open (dug) and open->ground (filled), by the scorecard's depth bands
(knot = within 4 cols of the door and 10 rows of the surface; top = rest of the top 10 rows; shaft = door column deeper;
deep = off the door column, deeper). Above it (the mound): soil->open and open->soil. Net per 1000-frame step, so a cell
dug and refilled inside one step is missed: these are lower bounds on gross digging. Prints the digs counter beside it."""
import sys, csv
GROUND = set("s#=r")
def founded(run):
    for L in open(f"{run}/events.txt"):
        if " FOUNDED " in L:
            kv = dict(t.split("=", 1) for t in L.split() if "=" in t)
            return int(kv["nest_x"]), int(kv["ground_y"])
def load(p):
    L = open(p).read().split("\n")
    x0, y0, w, h = map(int, L[0].split())
    return x0, y0, w, h, L[1:1 + h]
def band(x, y, nx, gy):
    d = y - gy; door = abs(x - nx) <= 4
    if d <= 10: return "knot" if door else "top"
    return "shaft" if door else "deep"
def run(r, a=100000, b=300000):
    nx, gy = founded(r)
    dug = dict(knot=0, top=0, shaft=0, deep=0); fill = dict(dug)
    mdug = mfill = 0
    prev = None
    for f in range(a, b + 1, 1000):
        x0, y0, w, h, rows = load(f"{r}/map_f{f:06d}.txt")
        if prev is not None:
            for j in range(h):
                y = y0 + j
                for i in range(w):
                    p, c = prev[j][i], rows[j][i]
                    pg, cg = p in GROUND, c in GROUND
                    if pg == cg: continue
                    if y > gy:
                        bd = band(x0 + i, y, nx, gy)
                        if pg: dug[bd] += 1
                        else: fill[bd] += 1
                    else:
                        if pg: mdug += 1
                        else: mfill += 1
        prev = rows
    S = {int(row["frame"]): row for row in csv.DictReader(open(f"{r}/stats.csv"))}
    digs = int(S[b]["digs"]) - int(S[a]["digs"])
    mlet = int(S[b]["mound_digs_let"]) - int(S[a]["mound_digs_let"])
    return dug, fill, mdug, mfill, digs, mlet
if __name__ == "__main__":
    print(f"{'run':10} {'digs':>6} {'mound_let':>9} | below ground dug (knot/top/shaft/deep = all; deeper) | filled (all; deeper) | mound soil dug / built")
    for r in sys.argv[1:]:
        dug, fill, md, mf, digs, mlet = run(r)
        name = r.rstrip("/").split("/")[-1]
        D = sum(dug.values()); F = sum(fill.values())
        print(f"{name:10} {digs:6} {mlet:9} | {dug['knot']}/{dug['top']}/{dug['shaft']}/{dug['deep']} = {D}; {dug['shaft']+dug['deep']}"
              f" | {F}; {fill['shaft']+fill['deep']} | {md} / {mf}")
