"""Brood/food separation in a nest snapshot (brood lane, 2026-10-24).
usage: separation.py RUN_DIR [RUN_DIR...]   reads nestmap_f*/occupants_f* (written by inrun under breeding every 25k)
Per snapshot: brood cells (egg/larva/pupa), stored food cells (provisions 'f' + crumbs in the nestmap, below the surface), and for brood the
share with food within 3 cells (Chebyshev) and the median distance to the nearest food. CONTROL: the same share for
every open nest cell below the surface within 12 columns of the door (where brood could have gone), so a share near
the control means placement ignores food; well below it means brood is kept apart."""
import sys, glob, re, statistics as S

def load(d, f):
    t = open(f'{d}/nestmap_f{f}.txt').read().split('\n')
    h = t[0].split(' '); x0, y0, leg = int(h[0]), int(h[1]), h[2].split(',')
    g = [r for r in t[1:] if r]
    o = [r for r in open(f'{d}/occupants_f{f}.txt').read().split('\n')[1:] if r]
    surf = [int(l) for l in open(f'{d}/surface.txt') if l.strip()]
    return x0, y0, leg, g, o, surf

def near(cells, food, r=3):
    fs = set(food)
    out = []
    for (x, y) in cells:
        best = None
        for d in range(0, 40):
            hit = any((x + dx, y + dy) in fs for dx in range(-d, d + 1) for dy in (-d, d)) or \
                  any((x + dx, y + dy) in fs for dy in range(-d, d + 1) for dx in (-d, d))
            if hit:
                best = d; break
        out.append(best if best is not None else 99)
    return out

print('run,frame,brood,food,brood_near3,ctrl_near3,brood_median_dist')
for d in sys.argv[1:]:
    for p in sorted(glob.glob(f'{d}/occupants_f*.txt')):
        f = re.search(r'_f(\d+)\.txt', p).group(1)
        x0, y0, leg, g, o, surf = load(d, f)
        brood, food, open_ = [], [], []
        for y in range(len(g)):
            for x in range(len(g[y])):
                ch, oc = g[y][x], o[y][x]
                m = leg[ord(ch) - 65] if ch != '?' else '?'
                if oc in 'elp':
                    brood.append((x, y))
                if (oc == 'f' or (m == 'crumbs' and oc not in 'elpa')) and y0 + y >= surf[x0 + x]:
                    food.append((x, y))  # stored food only: below the ground, so the field heaps do not count
                if m == 'empty' and y0 + y >= surf[x0 + x] and abs(x0 + x - 128) <= 12 and oc not in 'f':
                    open_.append((x, y))
        if not brood or not food:
            print(f'{d.rstrip("/").split("/")[-1]},{int(f)},{len(brood)},{len(food)},,,'); continue
        bd, cd = near(brood, food), near(open_, food)
        print(f'{d.rstrip("/").split("/")[-1]},{int(f)},{len(brood)},{len(food)},{sum(v <= 3 for v in bd)/len(bd):.2f},'
              f'{(sum(v <= 3 for v in cd)/len(cd)) if cd else float("nan"):.2f},{S.median(bd)}')
