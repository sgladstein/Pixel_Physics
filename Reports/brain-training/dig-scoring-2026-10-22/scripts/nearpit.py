"""Pit near the nest vs dents out in the field, from each run's final map (dig-scoring thread, 2026-10-22).
usage: nearpit.py RUN_DIR [RUN_DIR ...]   (RUN_DIR holds surface.txt and map_f*.txt)
nest.csv's pit counts every below-ground cell open straight to the sky anywhere in the bed, outside the mouth's +-4
columns. Under the rim refusal that number stayed at 46-111 while the picture showed no crater: it was one- and
two-cell surface dents 20-100 columns out. This splits it: near = within 20 columns of x=128 (mouth excluded), field
= the rest."""
import sys, glob
for run in sys.argv[1:]:
    surf = [int(l) for l in open(run + '/surface.txt')]
    m = sorted(glob.glob(run + '/map_f*.txt'))[-1]
    L = open(m).read().split('\n'); x0, y0, w, h = map(int, L[0].split())
    rows = [(r + ' ' * w)[:w] for r in L[1:1 + h]]
    near = field = 0
    for x in range(w):
        X = x + x0
        if abs(X - 128) <= 4:
            continue
        for y in range(h):
            if rows[y][x] not in '.af':
                break
            if y + y0 >= surf[X]:
                if abs(X - 128) <= 20: near += 1
                else: field += 1
    print(f'{run}\tnear {near}\tfield {field}')
