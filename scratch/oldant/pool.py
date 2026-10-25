"""SCRATCH (old-ant thread, 2026-10-25): read a std_pool-style pool file (member 0 whole, the rest as differences from
it) into full genomes, and the mind genome's blocks (mind.rs's layout at GENOME_LEN 2797, then the dig head, then the
6-slot lay gate)."""
import sys
def read_pool(p):
    lines = [l for l in open(p).read().split('\n') if l and not l.startswith('#')]
    h = lines[0].split(); n = int(h[3])
    pool, ids = [], []
    for l in lines[1:]:
        if l.startswith('member'):
            pool.append(list(pool[0]) if pool else [0.0] * n); ids.append(l.split()[2]); continue
        i, v = l.split(); pool[-1][int(i)] = float(v)
    return h, pool, ids
BLOCKS = [('settings', 0, 23), ('scorer ctx', 23, 583), ('scorer feat', 583, 1303), ('scorer verb', 1303, 1447),
          ('scorer out', 1447, 1464), ('job', 1464, 1788), ('memory', 1788, 2340), ('emit', 2340, 2445),
          ('job hidden', 2445, 2797), ('dig head', 2797, 3279), ('lay gate', 3279, 3285)]
if __name__ == '__main__':
    h, pool, ids = read_pool(sys.argv[1])
    for name, a, b in BLOCKS:
        c = [sum(1 for v in g[a:b] if abs(v) >= 1e-6) for g in pool]
        print(f'{name:12} slots {b-a:5}  live median {sorted(c)[len(c)//2]:5}  min {min(c)} max {max(c)}')
    tot = [sum(1 for v in g[23:] if abs(v) >= 1e-6) for g in pool]
    print('total live (from slot 23)', sorted(tot)[len(tot)//2], min(tot), max(tot))
