"""bandfill.py RUN... [--csv DIR] [--from F] [--to T] [--show F1-F2] -- how full each part of the dug nest is, map by map
(deep trace lane, 2026-10-06 evening, for the owner's question: how can the room be packed solid when almost no ant-time
is deep?).

On every map (`map_f*.txt`, 1k apart) every cell below the old ground line that is not ground (chambers.py's room:
ground is `s # = r`) is booked to the scorecard's bands: door knot (within 4 columns of the door, top 10 rows), top
(the rest of the top 10 rows), shaft (the door column deeper than 10 rows), deep (off the door column, deeper than
10 rows); and, when the owner's chamber rule (chambers.py) finds 2+ chambers, the cells every chamber but the biggest
claims are booked again as "2nd chamber". Each band's cells split into free (`.`), ant (`a`), brood (`b`) and other
(food, crumbs, corpses, water); beside them, the ants whose head is in the band (colony.csv, same frame). Fill is the
share of the band's room that is not free; free per ant is free cells over heads.

Prints the mean of each over --from..--to (100-300k by default) per band, and every map in --show (e.g. 80000-94000)
band by band. --csv DIR writes one row per map per band.
"""
import sys, os, csv, collections

# chambers.py from beside this file (deep-trace/tools/ or the repo's scripts/deeptrace_tools/), else the shared folder
sys.path[:0] = [os.path.dirname(os.path.abspath(__file__)), "/mnt/project-files/deep-trace/tools"]
import chambers as CH  # noqa: E402

BANDS = ["knot", "top", "shaft", "deep"]


def band(x, d, nx):
    door = abs(x - nx) <= 4
    if d <= 10:
        return "knot" if door else "top"
    return "shaft" if door else "deep"


def heads(run):
    out = collections.defaultdict(list)
    with open(f"{run}/colony.csv") as fh:
        for r in csv.DictReader(fh):
            if r["zone"] == "nest":
                out[int(r["frame"])].append((int(r["hx"]), int(r["hy"])))
    return out


def per_map(path, nx, gy, hs):
    mask, w, h, x0, top, rows = CH.room_mask(path, gy)
    found, _, _, _, claim = CH.chambers(mask, w, h)
    big = max(range(len(found)), key=lambda k: found[k]["cells"]) if found else -1
    y0 = int(open(path).readline().split()[1])
    c = {b: collections.Counter() for b in BANDS + ["2nd chamber"]}
    for j in range(h):
        y = top + j
        row = rows[y - y0]
        for i in range(w):
            if not mask[j][i]:
                continue
            ch = row[i]
            kind = "free" if ch == "." else "ant" if ch == "a" else "brood" if ch == "b" else "other"
            b = band(x0 + i, j + 1, nx)
            c[b]["room"] += 1
            c[b][kind] += 1
            if len(found) > 1 and claim[j][i] >= 0 and claim[j][i] != big:
                c["2nd chamber"]["room"] += 1
                c["2nd chamber"][kind] += 1
    for (hx, hy) in hs:
        d = hy - gy
        if d <= 0:
            continue
        c[band(hx, d, nx)]["heads"] += 1
        i, j = hx - x0, hy - top
        if len(found) > 1 and 0 <= j < h and 0 <= i < w and claim[j][i] >= 0 and claim[j][i] != big:
            c["2nd chamber"]["heads"] += 1
    return c, len(found)


def main():
    a = sys.argv[1:]
    opt = lambda k, d: a[a.index(k) + 1] if k in a else d
    lo, hi = int(opt("--from", 100000)), int(opt("--to", 300000))
    show = opt("--show", None)
    csvdir = opt("--csv", None)
    runs = [x for j, x in enumerate(a) if not x.startswith("--") and (j == 0 or a[j - 1] not in ("--from", "--to", "--show", "--csv"))]
    for run in runs:
        nx, gy = CH.founded(run)
        hd = heads(run)
        name = run.rstrip("/").split("/")[-1]
        maps = sorted(int(f[5:-4]) for f in os.listdir(run) if f.startswith("map_f") and f.endswith(".txt"))
        rows = []
        for f in maps:
            c, n = per_map(f"{run}/map_f{f:06d}.txt", nx, gy, hd.get(f, []))
            rows.append((f, c, n))
        if csvdir:
            os.makedirs(csvdir, exist_ok=True)
            with open(f"{csvdir}/bandfill-{name}.csv", "w") as fh:
                fh.write("frame,chambers,band,room,free,ant_cells,brood,other,heads\n")
                for f, c, n in rows:
                    for b in BANDS + ["2nd chamber"]:
                        k = c[b]
                        fh.write(f"{f},{n},{b},{k['room']},{k['free']},{k['ant']},{k['brood']},{k['other']},{k['heads']}\n")
        win = [(f, c, n) for f, c, n in rows if lo <= f <= hi]
        print(f"{run}: {len(win)} maps {lo // 1000}-{hi // 1000}k, means per map")
        print(f"  {'band':12} {'room':>6} {'free':>6} {'ants':>6} {'brood':>6} {'other':>6} {'heads':>6} {'fill':>6} {'free/head':>9}")
        for b in BANDS + ["2nd chamber"]:
            m = len(win) or 1
            tot = collections.Counter()
            for _, c, _ in win:
                tot.update(c[b])
            r = tot["room"] / m
            hh = tot["heads"] / m
            extra = f"   (on {sum(1 for _, _, n in win if n > 1)} maps)" if b == "2nd chamber" else ""
            print(f"  {b:12} {r:>6.0f} {tot['free'] / m:>6.0f} {tot['ant'] / m:>6.0f} {tot['brood'] / m:>6.0f} {tot['other'] / m:>6.0f} {hh:>6.1f} "
                  f"{(1 - tot['free'] / tot['room']) if tot['room'] else float('nan'):>6.0%} {(tot['free'] / tot['heads']) if tot['heads'] else float('nan'):>9.1f}{extra}")
        if show:
            s0, s1 = map(int, show.split("-"))
            print(f"  maps {s0 // 1000}-{s1 // 1000}k: per band room/free/brood/heads")
            for f, c, n in rows:
                if s0 <= f <= s1:
                    print(f"    {f // 1000:>4}k " + "  ".join(f"{b} {c[b]['room']}/{c[b]['free']}/{c[b]['brood']}/{c[b]['heads']}" for b in BANDS)
                          + (f"  2nd {c['2nd chamber']['room']}/{c['2nd chamber']['free']}/{c['2nd chamber']['brood']}/{c['2nd chamber']['heads']}" if n > 1 else ""))


if __name__ == "__main__":
    main()
