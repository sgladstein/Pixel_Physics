"""Write a colony's carry as a pool file (the format of assets/minds/std_pool.txt), for the ratchet (owner, 2026-10-24:
every batch starts from the latest best pool).

    python3 scratch/inrun/mkpool.py RUN/cK/rN OUT.txt "comment line" ...

Takes colsel's carry of the round directory (the top 30 of final/index.csv by score, as `carried()` reads it), writes
member 0 whole and every later member as its differences from member 0, zeros included, which is what colsel's
`expand_pool` and `lab::training::std_pool` read back."""
import csv
import sys

rdir, out, notes = sys.argv[1], sys.argv[2], sys.argv[3:]
r = list(csv.DictReader(open(f'{rdir}/final/index.csv')))
r.sort(key=lambda x: int(x['id']))
r.sort(key=lambda x: (-float(x['score']), int(x['id'])))


def load(i):
    lines = open(f'{rdir}/final/{i}.raw.txt').read().split('\n')
    head = lines[0].split()
    assert head[0] == 'genome', head
    g = {}
    for l in lines[1:]:
        if l.strip():
            k, w = l.split()
            g[int(k)] = w
    return head, g


# Distinct genomes only, best score first: a child copied with no mutation is a second ant with its parent's genome,
# and a pool member twice is a member wasted (farfood3b logsense c12 r23 had one such pair in its top 30; the lab's
# `the_first_founders_are_the_standard_pool` asserts the members differ).
seen, keep = set(), []
for x in r:
    key = tuple(sorted((i, float(w)) for i, w in load(x['id'])[1].items()))
    if key not in seen:
        seen.add(key)
        keep.append(x)
r = keep[:30]
head0, g0 = load(r[0]['id'])
with open(out, 'w') as f:
    for n in notes:
        f.write(f'# {n}\n')
    f.write(f'pool {len(r)} genome {head0[1]} {head0[2]}\n')
    for k, x in enumerate(r):
        head, g = load(x['id'])
        assert head[1:] == head0[1:], (head, head0)
        f.write(f'member {k} {x["id"]}\n')
        if k == 0:
            f.writelines(f'{i} {w}\n' for i, w in sorted(g.items()))
        else:
            for i in sorted(set(g) | set(g0)):
                w = g.get(i, '0')
                if float(w) != float(g0.get(i, '0')):
                    f.write(f'{i} {w}\n')
print(f'{out}: {len(r)} members')
