import sys, statistics as st
# usage: summ.py BASE ARM [ARM...]  -- per arm: alive, deaths, field bites, home deliveries (|x-128|<=8), digs; paired sign counts vs BASE
def one(run):
    a = int([l for l in open(run + '.log') if l.startswith('frame  200000')][0].split()[3])
    d = dict(alive=a, died=0, field=0, home=0, dig=0)
    for l in open(run + '/events.csv'):
        p = l.split(',')
        if len(p) < 4: continue
        k = p[2]
        if k == 'death': d['died'] += 1
        elif k == 'field': d['field'] += 1
        elif k == 'dig': d['dig'] += 1
        elif k == 'deliver':
            try:
                if abs(int(p[3]) - 128) <= 8: d['home'] += 1
            except ValueError: pass
    return d
seeds = range(1, 33)
arms = sys.argv[1:]
R = {a: {s: one(f'/tmp/claude-0/tj/{a}-{s}') for s in seeds} for a in arms}
base = arms[0]
for a in arms:
    line = f'{a:14s}'
    for k in ('alive', 'died', 'field', 'home', 'dig'):
        v = [R[a][s][k] for s in seeds]
        line += f' | {k} {st.mean(v):7.1f}'
        if a != base:
            up = sum(R[a][s][k] > R[base][s][k] for s in seeds); dn = sum(R[a][s][k] < R[base][s][k] for s in seeds)
            line += f' ({up}/{dn})'
    print(line)
