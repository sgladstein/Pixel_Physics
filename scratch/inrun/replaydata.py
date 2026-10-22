"""SCRATCH (teacher lane): replay page data from an inrun film=N run: material grids (delta-coded), owners, graph series."""
import sys, csv, glob, re, json, gzip, struct, collections
run, out = sys.argv[1], sys.argv[2]
CW, Y0, Y1, FX0, FY0, STEP = 161, 16, 72, 48, 30, 50
names = [l.split(' ', 1)[1].strip() for l in open(run + '/film/materials.txt')]
mats = sorted(glob.glob(run + '/film/m*.bin'))
prev = None; frames = []
for p in mats:
    b = open(p, 'rb').read()
    g = struct.unpack('<%dH' % (len(b) // 2), b)
    g = g[Y0 * CW:Y1 * CW]
    if prev is None:
        frames.append({'full': list(g)})
    else:
        ch = [v for i in range(len(g)) if g[i] != prev[i] for v in (i, g[i])]
        frames.append({'d': ch})
    prev = g
owners = []
for p in sorted(glob.glob(run + '/film/o*.txt')):
    t = open(p).read().split()
    o = {}
    for k in range(0, len(t), 2):
        i = int(t[k]) - Y0 * CW
        if 0 <= i < (Y1 - Y0) * CW:
            o[i] = int(t[k + 1])
    owners.append(o)
# owner ids per frame as [idx, id] flat
own = [[v for i, d in sorted(o.items()) for v in (i, d)] for o in owners]
# energy of each ant at each 4k snapshot
snaps = {}
for p in glob.glob(run + '/ants_f*.txt'):
    fr = int(re.search(r'ants_f(\d+)', p).group(1))
    snaps[fr] = {l.split()[0]: round(float(l.split()[3]) / 200, 1) for l in open(p)}
# series per 1,000 frames
stock = list(csv.DictReader(open(run + '/stock.csv')))
F = [int(r['frame']) for r in stock]
ev = collections.defaultdict(collections.Counter)
for e in csv.DictReader(open(run + '/events.csv')):
    b = (int(e['frame']) - 1) // 1000 * 1000 + 1000
    k = e['kind']
    if k == 'death' and e['v'] == '1': k = 'starve'
    ev[b][k] += 1
births = collections.Counter(); culls = collections.Counter()
for r in csv.DictReader(open(run + '/births.csv')):
    b = (int(r['frame']) - 1) // 1000 * 1000 + 1000
    if r['kind'] in ('birth', 'refill', 'hall'): births[b] += 1
    elif r['kind'] in ('cull', 'retire'): culls[b] += 1
def cum(c):
    s = 0; o = []
    for f in F: s += c[f]; o.append(s)
    return o
nest = list(csv.DictReader(open(run + '/nest.csv')))
def step(col):
    o = []; v = None; k = 0
    for f in F:
        while k < len(nest) and int(nest[k]['frame']) <= f: v = float(nest[k][col]); k += 1
        o.append(v)
    return o
series = [
    ('alive', 'Ants alive', [int(r['alive']) for r in stock], True),
    ('tank', 'Mean tank (starting tanks)', [float(r['tank_mean']) for r in stock], True),
    ('tankmin', 'Lowest tank', [float(r['tank_min']) for r in stock], False),
    ('tankmax', 'Highest tank', [float(r['tank_max']) for r in stock], False),
    ('nestfood', 'Food stored in nest (cells)', [int(r['nest_food']) for r in stock], True),
    ('doorfood', 'Food at the door (cells)', [int(r['door_food']) for r in stock], False),
    ('fieldfood', 'Food in the heaps (cells)', [int(r['field_food']) for r in stock], False),
    ('crop', 'Food carried in crops (cells)', [int(r['crop_cells']) for r in stock], False),
    ('corpse', 'Corpses in the nest', [int(r['nest_corpse']) for r in stock], False),
    ('fieldsofar', 'Food bitten in the field so far (cells)', cum(collections.Counter({f: ev[f]['field'] for f in F})), True),
    ('delivsofar', 'Food delivered to the nest so far (cells)', cum(collections.Counter({f: ev[f]['deliver'] for f in F})), True),
    ('field', 'Field bites per 1,000 frames', [ev[f]['field'] for f in F], False),
    ('deliver', 'Deliveries per 1,000 frames', [ev[f]['deliver'] for f in F], False),
    ('dig', 'New cells dug per 1,000 frames', [ev[f]['dig'] for f in F], False),
    ('starve', 'Starvations per 1,000 frames', [ev[f]['starve'] for f in F], False),
    ('births', 'Births so far', cum(births), False),
    ('culls', 'Culls and retirements so far', cum(culls), False),
    ('nestscore', 'Nest score (every 25,000 frames)', step('nest_score'), False),
    ('pit', 'Open crater cells (every 25,000)', step('pit'), False),
]
data = {'names': names, 'cw': CW, 'rows': Y1 - Y0, 'x0': FX0, 'y0': FY0 + Y0, 'step': STEP, 'frames': frames, 'own': own,
        'snaps': {str(k): v for k, v in sorted(snaps.items())}, 'F': F,
        'series': [{'key': k, 'label': l, 'v': v, 'on': on} for k, l, v, on in series]}
raw = json.dumps(data, separators=(',', ':')).encode()
open(out, 'wb').write(raw)
print(len(raw), 'raw', len(gzip.compress(raw, 9)), 'gz', sum(len(f.get('d', [])) for f in frames) // 2, 'changes')
