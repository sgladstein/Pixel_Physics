"""Write a predator pool directory (`*.raw.txt`, as predtrain's `rN/pool` and the predator lanes' pools) as a lab pool
file: the format of assets/minds/spider_pool.txt, every member whole, zeros left out, read by
`lab::training::predator_minds` (nest raider thread, 2026-10-25).

    python3 scratch/inrun/mkpredpool.py POOL_DIR OUT.txt "comment line" ...
"""
import os
import sys

pdir, out, notes = sys.argv[1], sys.argv[2], sys.argv[3:]
files = sorted(f for f in os.listdir(pdir) if f.endswith('.raw.txt'))
assert files, f'{pdir}: no .raw.txt'
members, length = [], None
for f in files:
    lines = [l for l in open(os.path.join(pdir, f)).read().split('\n') if l.strip()]
    head = lines[0].split()
    assert head[0] == 'genome', head
    length = length or ' '.join(head[1:])
    assert ' '.join(head[1:]) == length, (f, head)
    members.append([l for l in lines[1:] if float(l.split()[1]) != 0.0])
with open(out, 'w') as o:
    for n in notes:
        o.write(f'# {n}\n')
    o.write(f'pool {len(members)} genome {length}\n')
    for k, m in enumerate(members):
        o.write(f'member {k}\n')
        o.writelines(f'{l}\n' for l in m)
print(f'{out}: {len(members)} members, genome {length}')
