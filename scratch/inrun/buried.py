# How stored food got sealed: for each walled-in nest food pile on a map, when food was last put down on those cells,
# and what was let go (spoil pellets, 'in') or cut ('dig', 'recut') within 2 cells of the pile after that.
import csv, sys, collections
D, F = sys.argv[1], int(sys.argv[2]); NEST = 128
surf = [int(l) for l in open(D + '/surface.txt')]
L = open(f"{D}/map_f{F:07d}.txt").read().split('\n'); x0, y0, W, H = map(int, L[0].split()); g = L[1:1 + H]
cells = {(i, j) for j, row in enumerate(g) for i, c in enumerate(row) if c == 'f' and abs(x0 + i - NEST) <= 12 and y0 + j >= surf[x0 + i]}
seen, piles = set(), []
for c0 in cells:
    if c0 in seen: continue
    comp, st, op = [], [c0], False; seen.add(c0)
    while st:
        i, j = st.pop(); comp.append((x0 + i, y0 + j))
        for ii, jj in ((i+1, j), (i-1, j), (i, j+1), (i, j-1)):
            if 0 <= jj < H and 0 <= ii < len(g[jj]):
                if g[jj][ii] in '.a': op = True
                if (ii, jj) in cells and (ii, jj) not in seen: seen.add((ii, jj)); st.append((ii, jj))
    piles.append((op, comp))
ev = [(int(r['frame']), r['kind'], int(r['x']), int(r['y'])) for r in csv.DictReader(open(D + '/events.csv')) if int(r['frame']) <= F]
for op, comp in piles:
    if op: continue
    S = set(comp)
    near = lambda x, y: any(abs(x - a) <= 2 and abs(y - b) <= 2 for a, b in S)
    put = [f for f, k, x, y in ev if k in ('deliver', 'churn') and (x, y) in S]
    t0 = max(put) if put else 0
    after = collections.Counter(k for f, k, x, y in ev if f > t0 and k in ('in', 'dig', 'recut', 'out') and near(x, y))
    print(f"sealed pile of {len(comp)} at x {min(a for a,b in comp)}-{max(a for a,b in comp)}, y {min(b for a,b in comp)}-{max(b for a,b in comp)}: food last put down at frame {t0}; after that within 2 cells: {dict(after)}")
print(f"piles: {sum(1 for o, c in piles if o)} open, {sum(1 for o, c in piles if not o)} sealed")
