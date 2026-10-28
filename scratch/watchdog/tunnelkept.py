#!/usr/bin/env python3
"""SCRATCH (watchdog, 2026-10-28): the Nest lane's tunnel pay against what is left at the round's end.

For every cut inrun logged as `tunnel` (paid as a passage) and every other deep `dig` (depth >= 3), classify the cell
in the round's last nestmap: open nest (roofed or narrow-mouthed open air joined to the door), filled nest (crumbs,
food, bodies in the nest), crater, back to ground (soil/spoil/packed: refilled), other. A passage bonus that is gamed
pays cuts that do not stay open nest, or pays cuts at the cone (small door distance) rather than away from it.
usage: tunnelkept.py RUNDIR...   (inrun round folders: events.csv + nestmap.txt + surface.txt)
"""
import collections, csv, os, statistics as st, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import nestsets  # noqa: E402


def classify(d):
    s = nestsets.sets(d)
    x0, y0, grid, legend, nest, seen, e = s['x0'], s['y0'], s['grid'], s['legend'], s['nest'], s['seen'], s['e']
    ground = {chr(65 + k) for k, m in enumerate(legend) if m in ('soil', 'packedsoil', 'spoil', 'stone')}
    H, W = len(grid), len(grid[0])

    def kind(x, y):
        dx, dy = x - x0, y - y0
        if not (0 <= dx < W and 0 <= dy < H):
            return 'off-map'
        c = grid[dy][dx]
        if c in ground:
            return 'ground'
        if (dx, dy) in nest:
            return 'open nest' if c == e else 'filled nest'
        if (dx, dy) in seen:
            return 'crater'
        return 'cut off'
    out = {'tunnel': collections.Counter(), 'deep dig': collections.Counter()}
    dist = {'tunnel': [], 'deep dig': []}
    for r in csv.reader(open(f'{d}/events.csv')):
        if len(r) < 6 or r[0] == 'frame':
            continue
        if r[2] == 'tunnel' or (r[2] == 'dig' and int(r[5]) >= 3):
            k = 'tunnel' if r[2] == 'tunnel' else 'deep dig'
            x, y = int(r[3]), int(r[4])
            out[k][kind(x, y)] += 1
            dist[k].append(abs(x - 128))
    return out, dist


if __name__ == '__main__':
    tot = {'tunnel': collections.Counter(), 'deep dig': collections.Counter()}
    alld = {'tunnel': [], 'deep dig': []}
    for d in sys.argv[1:]:
        if not os.path.exists(f'{d}/nestmap.txt'):
            continue
        o, ds = classify(d)
        for k in o:
            tot[k].update(o[k]); alld[k] += ds[k]
        n = sum(o['tunnel'].values())
        if n:
            print(d, 'tunnel', dict(o['tunnel']), 'median door dist', st.median(ds['tunnel']))
    for k in tot:
        n = sum(tot[k].values())
        if n:
            print(f'ALL {k}: n={n}', {a: f'{b / n:.0%}' for a, b in tot[k].most_common()},
                  f'door dist median {st.median(alld[k])}')
