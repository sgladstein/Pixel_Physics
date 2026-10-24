"""One picture sheet per in-run batch (dig-scoring thread, 2026-10-22): every arm's surviving colonies at the end of
their last round, cropped around the nest door, so a rung's number is read beside what it measured.

usage: python3 batchsheet.py BATCH_DIR OUT.png [--per-arm 8] [--arms a,b,c] [--nest 128]

Reads ARM_SEED/rN/map_f0200000.txt (the last round's final map; the Teacher lane's `pull` keeps it) and the last row
of that round's nest.csv for the caption. Colours: red = dug and open to the sky (the crater, rung 1's pit), blue = dug
under a roof, tan = spoil above the ground line, brown = untouched ground, green = food, black = ants. Extinct seeds are
skipped and counted in the row label. Captions: pit, roofed cells (nest.csv dug, which leaves out pit), rooms (niches + chambers), nest score,
door (share of 25k checks after 20k with the nest joined to the air), all from nest.csv.
"""
import csv, glob, os, re, sys
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import matplotlib.patches as mp

args = sys.argv[1:]
def opt(name, default):
    if name in args:
        i = args.index(name); v = args[i + 1]; del args[i:i + 2]; return v
    return default
PER = int(opt('--per-arm', '8'))
ARMS = opt('--arms', None)
NEST = int(opt('--nest', '128'))
if len(args) != 2:
    sys.exit(__doc__)
D, OUT = args[0].rstrip('/'), args[1]
C = dict(soil=(0.45, 0.34, 0.25), sky=(0.86, 0.90, 0.95), pit=(0.85, 0.22, 0.18), nest=(0.16, 0.42, 0.75),
         spoil=(0.78, 0.62, 0.40), ant=(0.1, 0.1, 0.1), food=(0.35, 0.65, 0.3))

def picture(run, surf):
    maps = sorted(glob.glob(run + '/map_f*.txt'))
    if not maps:
        return None
    L = open(maps[-1]).read().split('\n'); x0, y0, w, h = map(int, L[0].split())
    rows = [(r + ' ' * w)[:w] for r in L[1:1 + h]]
    img = [[C['sky']] * w for _ in range(h)]
    for x in range(w):
        g = surf[x + x0] - y0 if x + x0 < len(surf) else h
        sky = True
        for y in range(h):
            ch = rows[y][x]; below = y >= g
            if ch == '#':
                sky = False
                img[y][x] = C['soil'] if below else C['spoil']
            elif ch == 'f':
                sky = False; img[y][x] = C['food']
            elif ch == 'a':
                img[y][x] = C['ant']
            else:
                img[y][x] = (C['pit'] if sky else C['nest']) if below else C['sky']
    gx = NEST - x0; gy = surf[NEST] - y0
    return [r[max(0, gx - 32):gx + 32] for r in img[max(0, gy - 12):gy + 26]]

arms = sorted({re.sub(r'_\d+$', '', os.path.basename(p)) for p in glob.glob(D + '/*_*') if os.path.isdir(p)})
if ARMS:
    arms = [a for a in ARMS.split(',') if a in arms]
panels = {}
for a in arms:
    seeds = sorted((p for p in glob.glob(f'{D}/{a}_*') if os.path.isdir(p)), key=lambda p: int(p.rsplit('_', 1)[1]))
    ext = sum(os.path.exists(s + '/EXTINCT') for s in seeds)
    got = []
    for sd in seeds:
        if os.path.exists(sd + '/EXTINCT') or len(got) >= PER:
            continue
        rs = sorted(glob.glob(sd + '/r*'), key=lambda p: int(p.rsplit('r', 1)[1]))
        if not rs or not os.path.exists(rs[-1] + '/nest.csv') or not os.path.exists(rs[-1] + '/surface.txt'):
            continue
        surf = [int(l) for l in open(rs[-1] + '/surface.txt')]
        pic = picture(rs[-1], surf)
        if pic is None:
            continue
        nrows = list(csv.DictReader(open(rs[-1] + '/nest.csv')))
        e = nrows[-1]
        doors = [int(r['door_open']) for r in nrows if int(r['frame']) >= 20000 and int(r.get('door_open', -1)) >= 0]
        door = f"{100 * sum(doors) / len(doors):.0f}%" if doors else '?'
        cap = (f"{os.path.basename(sd)} {os.path.basename(rs[-1])}\npit {e['pit']}  roofed {e['dug']}  "
               f"rooms {int(e['niches']) + int(e['chambers'])}\nnest {float(e['nest_score']):.0f}  door {door}")
        got.append((pic, cap))
    panels[a] = (got, len(seeds), ext)

nr = max(1, len(arms)); nc = max(1, max((len(v[0]) for v in panels.values()), default=1))
fig, axs = plt.subplots(nr, nc, figsize=(2.3 * nc + 1.2, 2.5 * nr + 0.9), facecolor='#fcfcfb', squeeze=False)
for i, a in enumerate(arms):
    got, n, ext = panels[a]
    for j in range(nc):
        ax = axs[i][j]; ax.set_xticks([]); ax.set_yticks([])
        for sp in ax.spines.values():
            sp.set_visible(False)
        if j < len(got):
            ax.imshow(got[j][0], interpolation='nearest')
            ax.set_xlabel(got[j][1], fontsize=7, color='#333')
    axs[i][0].set_ylabel(f'{a}\n{n - ext} of {n} alive', fontsize=9)
fig.legend(handles=[mp.Patch(color=C[k], label=l) for k, l in (
    ('pit', 'dug, open to the sky (crater)'), ('nest', 'dug, under a roof'), ('spoil', 'spoil above ground'),
    ('soil', 'untouched ground'))], loc='lower center', ncol=4, frameon=False, fontsize=8)
fig.suptitle(f'{os.path.basename(D)}: each arm\'s surviving colonies at the end of their last round', x=0.01, ha='left', fontsize=11)
fig.tight_layout(rect=(0, 0.05, 1, 0.95))
fig.savefig(OUT, dpi=110)
print(OUT)
