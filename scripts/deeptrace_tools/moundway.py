#!/usr/bin/env python3
"""moundway.py RUN... [--frames 100000,200000 | --every 10000] [--from F] [--to F] [--png OUT.png] [--csv OUT.csv]
-- the way into the nest through the mound, map by map (deep trace lane, 2026-10-06, for Scott's 19:29 "there is not a
clear entrance tunnel").

Reads deeptrace's `map_fNNNNNN.txt` (every 1,000 frames from 6k), `events.txt` (nest_x, ground_y) and `colony.csv`.
No libraries. For each map:

- **The mound's outline**: per column the highest soil or ground cell (not food: the provisions dropped over the heap
  float), then the median over 5 columns, so a lone spike of soil does not hide the air beside it. Open cells under the outline and above the old
  ground line are *inside* the mound; open cells above it are sky.
- **Open** is air, ants, crumbs and brood (ants walk through crumbs and brood). A step is to any of the 8 neighbours.
  The *clear* columns repeat the walk with ants as walls, so a tunnel packed with ants reads as shut.
- **The way in** is the shortest walk from any sky cell to the dug nest (open cells on or below the old ground line
  that join the door's shaft, the door included; a second hole into the nest counts as a way in). `way` is its length in steps; `straight` is the mound's thickness over the door (old ground line
  minus the outline at the door column); `bendy` = way / straight. `mouth` is where the shortest way leaves the sky,
  as columns from the door and rows above the old ground line. `width` is how many walks fit in side by side without
  sharing a cell, i.e. the narrowest neck between the sky and the door (max flow with one walker per cell); 0 when
  the way is shut, blank when no mound covers the door. A blank `way` means **no open way at all**, even counting
  ants as space: the door is sealed from the sky (checked by a plain flood fill on seed 4 at 50k and 200k).
- **Inside cells by what they lead to**: `onway` (at most 4 steps longer through it than the shortest way), `offway`
  (connected at both ends but more than 4 steps out of the way: dead ends and loops hanging off the door's network),
  `skyonly` (open to the sky, no way to the door), `nestonly`, `pocket` (shut off from both).
- **Ant cells** on the map in each class (`onway_ant_cells` etc.; an ant's body is two cells), and **ants** from
  `colony.csv` at the same frame, by zone: `mound_in` (in the mound under a roof), `mound_top`, `nest`.

`--png` draws each map with the ants hidden: soil tan, sky black, the shortest way white, other on-way cells light
blue, off-way grey-blue, sky-only dark green, pockets magenta, the dug nest dark. One row per run, one column per frame.
"""
import sys, os, csv, zlib, struct
from collections import deque, Counter

OPEN = set(".acb")
OPEN_CLEAR = set(".cb")
SOILY = set("s#")  # what the outline is made of: soil and ground, not the food heap or the floating provisions
SLACK = 4
N8 = [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)]
INF = 1 << 30


def founded(run):
    for line in open(f"{run}/events.txt"):
        if "FOUNDED" in line:
            kv = dict(t.split("=") for t in line.split() if "=" in t)
            return int(kv["nest_x"]), int(kv["ground_y"])
    return 256, 160


def load(p):
    L = open(p).read().split("\n")
    x0, y0, w, h = map(int, L[0].split())
    return x0, y0, w, h, [r.ljust(w, "#")[:w] for r in L[1:1 + h]]


def bfs(starts, ok, w, h):
    d = [INF] * (w * h)
    q = deque()
    for i in starts:
        d[i] = 0
        q.append(i)
    while q:
        i = q.popleft()
        x, y = i % w, i // w
        for dx, dy in N8:
            nx, ny = x + dx, y + dy
            if 0 <= nx < w and 0 <= ny < h:
                j = ny * w + nx
                if d[j] == INF and ok[j]:
                    d[j] = d[i] + 1
                    q.append(j)
    return d


def maxflow_width(inside_ok, src_adj, sink_adj, w, h, cap_limit=40):
    """Vertex-disjoint walks from the sky to the nest through the inside cells (unit capacity per cell)."""
    nodes = [i for i in range(w * h) if inside_ok[i]]
    if not nodes:
        return 0
    S, T = -1, -2
    # residual graph: node ids 2i (in), 2i+1 (out)
    adj, cap = {}, {}

    def add(u, v, c):
        adj.setdefault(u, []).append(v)
        adj.setdefault(v, []).append(u)
        cap[(u, v)] = cap.get((u, v), 0) + c
        cap.setdefault((v, u), 0)

    for i in nodes:
        add(2 * i, 2 * i + 1, 1)
        x, y = i % w, i // w
        for dx, dy in N8:
            nx, ny = x + dx, y + dy
            if 0 <= nx < w and 0 <= ny < h:
                j = ny * w + nx
                if inside_ok[j]:
                    add(2 * i + 1, 2 * j, 1)
        if src_adj[i]:
            add(S, 2 * i, 1)
        if sink_adj[i]:
            add(2 * i + 1, T, 1)
    flow = 0
    while flow < cap_limit:
        prev = {S: None}
        q = deque([S])
        while q and T not in prev:
            u = q.popleft()
            for v in adj.get(u, ()):
                if v not in prev and cap[(u, v)] > 0:
                    prev[v] = u
                    q.append(v)
        if T not in prev:
            break
        v = T
        while prev[v] is not None:
            u = prev[v]
            cap[(u, v)] -= 1
            cap[(v, u)] += 1
            v = u
        flow += 1
    return flow


def analyse(run, f, nest_x, gy, colony=None):
    p = f"{run}/map_f{f:06d}.txt"
    if not os.path.exists(p):
        return None
    x0, y0, w, h, rows = load(p)
    G = gy - y0          # row index of the old ground line
    DX = nest_x - x0     # column index of the door
    cell = lambda x, y: rows[y][x]
    # outline: highest wall per column, median over 5
    top = []
    for x in range(w):
        t = h
        for y in range(h):
            if cell(x, y) in SOILY:
                t = y
                break
        top.append(t)
    tops = []
    for x in range(w):
        win = sorted(top[max(0, x - 2):x + 3])
        tops.append(win[len(win) // 2])
    sky = [False] * (w * h)
    inside = [False] * (w * h)
    below = [False] * (w * h)
    for y in range(h):
        for x in range(w):
            ch = cell(x, y)
            if ch not in OPEN:
                continue
            i = y * w + x
            if y >= G:
                below[i] = True
            elif y < tops[x]:
                sky[i] = True
            else:
                inside[i] = True
    # the dug nest: open cells on or below the old ground line joined to the door's shaft (a pit dug in the open
    # ground elsewhere is not a way in)
    seeds = [y * w + x for y in range(G, min(h, G + 4)) for x in range(max(0, DX - 3), min(w, DX + 4)) if below[y * w + x]]
    dn = bfs(seeds, below, w, h) if seeds else [0 if b else INF for b in below]
    nest = [below[i] and dn[i] < INF for i in range(w * h)]
    out = {"run": run, "frame": f}
    mound_cols = [x for x in range(w) if tops[x] < G]
    out["mound_w"] = len(mound_cols)
    out["mound_h"] = G - min(tops) if mound_cols else 0
    out["straight"] = max(0, G - tops[DX])
    soil = sum(1 for y in range(G) for x in range(w) if y >= tops[x] and cell(x, y) not in OPEN)
    n_in = sum(inside)
    out["mound_soil"] = soil
    out["inside_open"] = n_in
    out["open_share"] = round(n_in / max(1, n_in + soil), 3)
    res = {}
    for tag, opn in (("", OPEN), ("clear_", OPEN_CLEAR)):
        ok = [False] * (w * h)
        for i in range(w * h):
            if inside[i] or nest[i] or sky[i]:
                ok[i] = cell(i % w, i // w) in opn
        dS = bfs([i for i in range(w * h) if sky[i] and ok[i]], ok, w, h)
        dT = bfs([i for i in range(w * h) if nest[i] and ok[i]], ok, w, h)
        D = min((dS[i] for i in range(w * h) if nest[i] and ok[i]), default=INF)
        out[tag + "way"] = D if D < INF else ""
        res[tag] = (ok, dS, dT, D)
    ok, dS, dT, D = res[""]
    out["bendy"] = round(D / out["straight"], 2) if D < INF and out["straight"] > 0 else ""
    # the shortest way itself, traced back from the nest cell nearest the sky
    path = []
    if D < INF:
        end = min((i for i in range(w * h) if nest[i] and ok[i] and dS[i] == D), key=lambda i: abs(i % w - DX))
        cur = end
        while dS[cur] > 0:
            x, y = cur % w, cur // w
            path.append(cur)
            best = None
            for dx, dy in N8:
                nx, ny = x + dx, y + dy
                if 0 <= nx < w and 0 <= ny < h:
                    j = ny * w + nx
                    if ok[j] and dS[j] == dS[cur] - 1 and (best is None or abs(nx - DX) < abs(best % w - DX)):
                        best = j
            cur = best
        path.append(cur)
        mx, my = cur % w, cur // w
        out["mouth_dx"], out["mouth_up"] = mx - DX, G - my
        out["way_ants"] = sum(1 for i in path if cell(i % w, i // w) == "a")
    else:
        out["mouth_dx"] = out["mouth_up"] = out["way_ants"] = ""
    # classes of inside cells
    cls = Counter()
    kind = [""] * (w * h)
    for i in range(w * h):
        if not inside[i] or not ok[i]:
            continue
        a, b = dS[i] < INF, dT[i] < INF
        if a and b:
            k = "onway" if dS[i] + dT[i] - D <= SLACK else "offway"
        elif a:
            k = "skyonly"
        elif b:
            k = "nestonly"
        else:
            k = "pocket"
        cls[k] += 1
        kind[i] = k
    for k in ("onway", "offway", "skyonly", "nestonly", "pocket"):
        out[k] = cls[k]
    antin = Counter(kind[i] for i in range(w * h) if kind[i] and cell(i % w, i // w) == "a")
    out["onway_ant_cells"], out["offway_ant_cells"], out["pocket_ant_cells"] = antin["onway"], antin["offway"], antin["pocket"]
    # mouths: groups of inside open cells touching the sky; how many lead to the door
    mouth = [False] * (w * h)
    for i in range(w * h):
        if inside[i] and ok[i]:
            x, y = i % w, i // w
            if any(0 <= x + dx < w and 0 <= y + dy < h and sky[(y + dy) * w + x + dx] for dx, dy in N8):
                mouth[i] = True
    seen, groups, through = set(), 0, 0
    for i in range(w * h):
        if mouth[i] and i not in seen:
            groups += 1
            q, lead = [i], False
            seen.add(i)
            while q:
                j = q.pop()
                lead |= dT[j] < INF
                x, y = j % w, j // w
                for dx, dy in N8:
                    nx, ny = x + dx, y + dy
                    k2 = ny * w + nx
                    if 0 <= nx < w and 0 <= ny < h and mouth[k2] and k2 not in seen:
                        seen.add(k2)
                        q.append(k2)
            through += lead
    out["mouths"], out["mouths_to_door"] = groups, through
    # the narrowest neck, sky to nest, through inside cells (ants not walls)
    src = [inside[i] and ok[i] and mouth[i] for i in range(w * h)]
    snk = [False] * (w * h)
    for i in range(w * h):
        if inside[i] and ok[i]:
            x, y = i % w, i // w
            snk[i] = any(0 <= x + dx < w and 0 <= y + dy < h and nest[(y + dy) * w + x + dx] and ok[(y + dy) * w + x + dx] for dx, dy in N8)
    inside_ok = [inside[i] and ok[i] for i in range(w * h)]
    # no mound over the door (the door opens on the sky): the width is the door's, not the mound's, so leave it blank
    out["width"] = maxflow_width(inside_ok, src, snk, w, h) if not (D <= 1 and out["straight"] == 0) else ""
    # ants
    if colony is not None:
        z = colony.get(f, Counter())
        out["ants"] = sum(z.values())
        out["mound_in"], out["mound_top"], out["nest_ants"] = z["mound_in"], z["mound_top"], z["nest"]
    out["_draw"] = (w, h, rows, kind, set(path), nest, sky)
    return out


def read_colony(run):
    z = {}
    p = f"{run}/colony.csv"
    if not os.path.exists(p):
        return None
    with open(p) as fh:
        for r in csv.DictReader(fh):
            z.setdefault(int(r["frame"]), Counter())[r["zone"]] += 1
    return z


PAL = {"onway": (150, 200, 235), "offway": (70, 80, 110), "skyonly": (40, 90, 50), "nestonly": (90, 60, 30),
       "pocket": (200, 60, 200)}


def png(outpath, grid, S=4, X=None, Y=None):
    GAP = 6
    first = next(c for r in grid for c in r if c)
    w, h = first["_draw"][0], first["_draw"][1]
    xa, xb = X if X else (0, w - 1)
    ya, yb = Y if Y else (0, h - 1)
    cw, ch = (xb - xa + 1) * S, (yb - ya + 1) * S
    W = len(grid[0]) * (cw + GAP) + GAP
    H = len(grid) * (ch + GAP) + GAP
    img = [[(12, 12, 16)] * W for _ in range(H)]
    for ri, row in enumerate(grid):
        for ci, o in enumerate(row):
            ox, oy = GAP + ci * (cw + GAP), GAP + ri * (ch + GAP)
            if not o:
                continue
            w, h, rows, kind, way, nest, sky = o["_draw"]
            for y in range(ya, yb + 1):
                for x in range(xa, xb + 1):
                    i = y * w + x
                    c0 = rows[y][x]
                    if i in way:
                        c = (255, 255, 255)
                    elif kind[i]:
                        c = PAL[kind[i]]
                    elif nest[i]:
                        c = (40, 40, 52)
                    elif sky[i]:
                        c = (26, 26, 34)
                    elif c0 == "f":
                        c = (95, 158, 87)
                    elif c0 in "x":
                        c = (150, 70, 160)
                    elif c0 == "~":
                        c = (106, 160, 200)
                    elif c0 == "#":
                        c = (107, 79, 50)
                    else:
                        c = (200, 180, 138)
                    for yy in range(S):
                        line = img[oy + (y - ya) * S + yy]
                        for xx in range(S):
                            line[ox + (x - xa) * S + xx] = c
    raw = b"".join(b"\x00" + bytes(v for px in line for v in px) for line in img)
    chunk = lambda t, d: struct.pack(">I", len(d)) + t + d + struct.pack(">I", zlib.crc32(t + d) & 0xFFFFFFFF)
    open(outpath, "wb").write(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", W, H, 8, 2, 0, 0, 0))
                           + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))


COLS = ["run", "frame", "way", "straight", "bendy", "width", "clear_way", "way_ants", "mouth_dx", "mouth_up", "mouths",
        "mouths_to_door", "mound_h", "mound_w", "mound_soil", "inside_open", "open_share", "onway", "offway", "skyonly",
        "nestonly", "pocket", "onway_ant_cells", "offway_ant_cells", "pocket_ant_cells", "ants", "mound_in", "mound_top",
        "nest_ants"]


def main():
    a = sys.argv[1:]
    opt = {}
    for k in ("--frames", "--every", "--from", "--to", "--png", "--csv", "--crop"):
        if k in a:
            i = a.index(k)
            opt[k] = a[i + 1]
            del a[i:i + 2]
    runs = a
    if "--frames" in opt:
        frames = [int(x) for x in opt["--frames"].split(",")]
    else:
        ev = int(opt.get("--every", 10000))
        frames = list(range(int(opt.get("--from", ev)), int(opt.get("--to", 300000)) + 1, ev))
    grid, rows_out = [], []
    print("\t".join(COLS))
    for run in runs:
        nest_x, gy = founded(run)
        col = read_colony(run)
        row = []
        for f in frames:
            o = analyse(run, f, nest_x, gy, col)
            row.append(o)
            if o:
                print("\t".join(str(o.get(c, "")) for c in COLS), flush=True)
                rows_out.append({c: o.get(c, "") for c in COLS})
        grid.append(row)
    if "--csv" in opt:
        with open(opt["--csv"], "w", newline="") as fh:
            wr = csv.DictWriter(fh, COLS)
            wr.writeheader()
            wr.writerows(rows_out)
    if "--png" in opt:
        X = Y = None
        if "--crop" in opt:  # world coords x0,x1,y0,y1
            cx0, cx1, cy0, cy1 = map(int, opt["--crop"].split(","))
            first = next(c for r in grid for c in r if c)
            p = f"{first['run']}/map_f{first['frame']:06d}.txt"
            mx0, my0 = map(int, open(p).readline().split()[:2])
            X, Y = (cx0 - mx0, cx1 - mx0), (cy0 - my0, cy1 - my0)
        png(opt["--png"], grid, X=X, Y=Y)
        print("png", opt["--png"], file=sys.stderr)


if __name__ == "__main__":
    main()
