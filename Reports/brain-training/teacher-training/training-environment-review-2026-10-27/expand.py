import os, sys
path, out = sys.argv[1], sys.argv[2]
lines = [l for l in open(path).read().split('\n') if l.strip() and not l.startswith('#')]
head = lines[0].split()
members = []
for l in lines[1:]:
    if l.startswith('member '):
        members.append(dict(members[0]) if members else {}); continue
    i, w = l.split(); members[-1][int(i)] = w
os.makedirs(out, exist_ok=True)
res = []
for k, m in enumerate(members):
    fp = f'{out}/p{k:02}.raw.txt'
    with open(fp, 'w') as f:
        f.write(f'genome {head[3]} {head[4]}\n')
        f.writelines(f'{i} {w}\n' for i, w in sorted(m.items()) if float(w) != 0.0)
    res.append(f'{fp}@p{k:02}:1')
print(','.join(res))
