"""scorecard.py's room rule (open below ground, >=7 of 3x3 open, 4-connected, 30+), with each room's bounding box and narrowest width."""
import sys
OPEN = set('.abfcx')  # anything but soil/ground
def comps(path, gy):
    L = open(path).read().split('\n'); x0, y0, wd, ht = map(int, L[0].split()); rows = [l.ljust(wd, '#')[:wd] for l in L[1:1 + ht]]
    op = [[rows[y][x] in OPEN and y0 + y > gy for x in range(wd)] for y in range(ht)]
    room = {(x, y) for y in range(1, ht - 1) for x in range(1, wd - 1) if op[y][x] and sum(op[y + j][x + i] for j in (-1, 0, 1) for i in (-1, 0, 1)) >= 7}
    seen, out = set(), []
    for p in room:
        if p in seen: continue
        st, cc = [p], []; seen.add(p)
        while st:
            cx, cy = st.pop(); cc.append((cx, cy))
            for q in ((cx + 1, cy), (cx - 1, cy), (cx, cy + 1), (cx, cy - 1)):
                if q in room and q not in seen: seen.add(q); st.append(q)
        if len(cc) >= 30:
            xs = [c[0] + x0 for c in cc]; ys = [c[1] + y0 for c in cc]
            rw = {}
            for c in cc: rw[c[1]] = rw.get(c[1], 0) + 1
            out.append((len(cc), min(xs), max(xs), min(ys) - gy, max(ys) - gy, sorted(rw.values())[len(rw) // 2]))
    return sorted(out, reverse=True)
B = sys.argv[1]
for s in (1, 2, 3, 4):
    for f in (100000, 200000, 300000):
        cs = comps(f'{B}/s{s}/map_f{f}.txt', 160)
        print(f's{s} {f//1000}k: ' + '; '.join(f'{n} cells x{a}-{b} depth {t}-{u} medrow {m}' for n, a, b, t, u, m in cs))
