"""E1 (review plan): where does the free arm's mutation load come from? For each of 6 evolved genomes make 12
standard mutants (the emergence runs' operator, 0.013:1.0:4) and split each into its channel part (alarm-read
links + alarm emit row) and its everything-else part; plus 12 gentle mutants (0.013:0.25:1). Writes arms.json."""
import json, os, subprocess, sys
sys.path.insert(0, '/tmp/claude-0/-home-user-Pixel-Physics/2ef7e65c-531a-5db4-866b-18b502ff6e79/scratchpad/review')
S = '/tmp/claude-0/-home-user-Pixel-Physics/2ef7e65c-531a-5db4-866b-18b502ff6e79/scratchpad'
E = f'{S}/e1'; BIN = f'{S}/nc/inrun2.bin'
import io, contextlib
with contextlib.redirect_stdout(io.StringIO()):
    sys.argv = ['slots.py']
    import slots
CH = slots.alarm_read() | set(range(slots.EMIT + 2 * slots.E_IN, slots.EMIT + 3 * slots.E_IN))
G = {'F1': f'{S}/nc/free_s1/g149/c00.raw.txt', 'F1b': f'{S}/nc/free_s1/g149/c01.raw.txt',
     'F2': f'{S}/nc/rep2/free.raw.txt', 'D1': f'{S}/nc/deaf_s1/g149/c00.raw.txt',
     'D2': f'{S}/nc/rep2/deaf.raw.txt', 'D2b': '/mnt/project-files/evolvable-pheromones/newchan/deaf_s2/g149/c01.raw.txt'}
GROUND = {'PIXEL_PHYSICS_ALARM_GROUND': '0.03:0.05'}
def load(p):
    L = open(p).read().split('\n'); return L[0], {int(l.split()[0]): l.split()[1] for l in L[1:] if l.strip()}
def save(p, h, g):
    open(p, 'w').write(h + '\n' + ''.join(f'{i} {w}\n' for i, w in sorted(g.items()) if float(w) != 0.0))
arms = {}; nch = {}
for name, path in G.items():
    d = f'{E}/{name}'; os.makedirs(d, exist_ok=True)
    env = {**GROUND} if name.startswith('F') else {**GROUND, 'PP_SCENT_DEAF': 'alarm'}
    h, par = load(path); save(f'{d}/parent.raw.txt', h, par)
    arms[f'{name}_parent'] = {'founders': f'{d}/parent.raw.txt:30', 'env': env}
    for k in range(12):
        for kind, mut in (('std', '0.013:1.0:4'), ('gentle', '0.013:0.25:1')):
            dest = f'{d}/{kind}{k:02}.raw.txt'
            pm, sg, ev = mut.split(':')
            subprocess.run([BIN, f'founders={path}:1', f'out={E}/_mut', f'mutate_to={dest}:{pm}:{sg}:{ev}:{7000 + 100 * k + (kind == "gentle")}'],
                           capture_output=True, env={**os.environ, 'PP_MIND_ALARM': 'train'}, check=True)
            arms[f'{name}_{kind}{k:02}'] = {'founders': f'{dest}:30', 'env': env}
        _, m = load(f'{d}/std{k:02}.raw.txt')
        diff = {i for i in set(par) | set(m) if par.get(i, '0') != m.get(i, '0')}
        nch.setdefault(name, []).append((len(diff & CH), len(diff - CH)))
        for part, keep in (('chan', diff & CH), ('other', diff - CH)):
            g = dict(par)
            for i in keep:
                if i in m: g[i] = m[i]
                else: g.pop(i, None)
            save(f'{d}/{part}{k:02}.raw.txt', h, g)
            arms[f'{name}_{part}{k:02}'] = {'founders': f'{d}/{part}{k:02}.raw.txt:30', 'env': env}
json.dump(arms, open(f'{E}/arms.json', 'w'))
for n, v in nch.items(): print(n, 'changed slots (chan, other) per std mutant:', v)
print(len(arms), 'arms')
