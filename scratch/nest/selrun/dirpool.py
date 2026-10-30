"""SCRATCH (chambers selection run, 2026-10-29): a directory of .raw.txt founders written as one pool file (the format
of assets/minds/std_pool.txt), so colsel.py takes it as GENOME (a GENOME ending in pool.txt is founded as the pool,
each member once, no mutants). Member 0 whole, every later member as its differences from member 0, zeros included,
as scratch/inrun/mkpool.py writes it; the weights are copied as strings, so colsel's expand_pool writes the same files
back (checked by --check).

    python3 scratch/nest/selrun/dirpool.py DIR OUT_pool.txt "comment" ...     # members in file-name order
    python3 scratch/nest/selrun/dirpool.py --check OUT_pool.txt DIR            # expand it back and compare
"""
import glob, os, sys


def load(p):
    lines = open(p).read().split('\n')
    head = lines[0].split()
    assert head[0] == 'genome', (p, head)
    g = {}
    for l in lines[1:]:
        if l.strip():
            k, w = l.split()
            g[int(k)] = w
    return head, g


def expand(path):
    """colsel.expand_pool's reading, without its side effects: [(name, {i: w})] for each member."""
    lines = [l for l in open(path).read().split('\n') if l.strip() and not l.startswith('#')]
    head = lines[0].split()
    assert head[0] == 'pool' and head[2] == 'genome', head
    members = []
    for l in lines[1:]:
        if l.startswith('member '):
            members.append(dict(members[0]) if members else {})
            continue
        i, w = l.split()
        members[-1][int(i)] = w
    return head, [{i: w for i, w in m.items() if float(w) != 0.0} for m in members]


if sys.argv[1] == '--check':
    head, ms = expand(sys.argv[2])
    files = sorted(glob.glob(f'{sys.argv[3]}/*.raw.txt'))
    assert len(files) == len(ms) == int(head[1]), (len(files), len(ms), head)
    for f, m in zip(files, ms):
        h, g = load(f)
        assert h[1:] == head[3:5], (f, h, head)
        assert {i: w for i, w in g.items() if float(w) != 0.0} == m, f
    print(f'check ok: {len(ms)} members round-trip to {sys.argv[3]}')
    sys.exit(0)

src, out, notes = sys.argv[1], sys.argv[2], sys.argv[3:]
files = sorted(glob.glob(f'{src}/*.raw.txt'))
assert files, src
head0, g0 = load(files[0])
with open(out, 'w') as f:
    for n in notes:
        f.write(f'# {n}\n')
    f.write(f'pool {len(files)} genome {head0[1]} {head0[2]}\n')
    for k, p in enumerate(files):
        head, g = load(p)
        assert head[1:] == head0[1:], (p, head, head0)
        f.write(f'member {k} {os.path.basename(p)[:-len(".raw.txt")]}\n')
        if k == 0:
            f.writelines(f'{i} {w}\n' for i, w in sorted(g.items()))
        else:
            for i in sorted(set(g) | set(g0)):
                w = g.get(i, '0')
                if float(w) != float(g0.get(i, '0')) or (i in g0) != (i in g):
                    f.write(f'{i} {w}\n')
print(f'{out}: {len(files)} members from {src}')
