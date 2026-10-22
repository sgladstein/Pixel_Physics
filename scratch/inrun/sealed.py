# Stored food the colony can still reach: nest food cells (within 12 columns of the nest, below the datum) with an open
# or ant-held cell beside them, against those walled in, from the map files of each round.
import glob, sys, collections
NEST = 128
tot = collections.Counter()
for m in sorted(glob.glob(sys.argv[1] + '/*/r*/map_f0200000.txt')):
    d = m.rsplit('/', 1)[0]
    surf = [int(l) for l in open(d + '/surface.txt')]
    L = open(m).read().split('\n')
    x0, y0, W, H = map(int, L[0].split()); g = L[1:1 + H]
    reach = sealed = 0
    cells = {(i, j) for j, row in enumerate(g) for i, c in enumerate(row) if c == 'f' and abs(x0 + i - NEST) <= 12 and y0 + j >= surf[x0 + i]}
    seen = set()
    for c0 in cells:
        if c0 in seen: continue
        comp, st, open_ = [], [c0], False
        seen.add(c0)
        while st:  # a pile counts as reachable if any of its cells touches open ground or an ant
            i, j = st.pop(); comp.append((i, j))
            for ii, jj in ((i+1, j), (i-1, j), (i, j+1), (i, j-1)):
                if 0 <= jj < H and 0 <= ii < len(g[jj]):
                    if g[jj][ii] in '.a': open_ = True
                    if (ii, jj) in cells and (ii, jj) not in seen: seen.add((ii, jj)); st.append((ii, jj))
        if open_: reach += len(comp)
        else: sealed += len(comp)
    arm = d.split('/')[-2].split('_')[0]
    tot[(arm, 'reach')] += reach; tot[(arm, 'sealed')] += sealed
for arm in sorted({k[0] for k in tot}):
    r, s = tot[(arm, 'reach')], tot[(arm, 'sealed')]
    print(f"{arm:6s} nest food cells at round ends: touching open {r}, walled in {s} ({100*s/max(1,r+s):.0f}% sealed)")
