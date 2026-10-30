"""Colony ledger per 25k window for one inrun run (trace=250, PP_HUNGRYPROBE=1).
usage: ledger.py RUNDIR [RUNDIR...]
place: bowl = below datum within 12 cols of door; tunnel = below datum beyond 12 cols; door = above datum within 12;
field = surface beyond 12; wall = surface x<12 or x>436.
"""
import csv, sys, collections
NEST, WIN = 128, 25000
CAUSE = ['unk', 'starved', 'starvedfly', 'killed', 'culled', 'lostvital', 'felled', 'oldage']


def place(x, y, surf):
    s = surf[x] if 0 <= x < len(surf) else 0
    dx = abs(x - NEST)
    if y > s:
        return 'bowl' if dx <= 12 else 'tunnel'
    if x < 12 or x > 436:
        return 'wall'
    return 'door' if dx <= 12 else 'field'


def run(d):
    surf = [int(l) for l in open(f'{d}/surface.txt') if l.strip()]
    W = collections.defaultdict(collections.Counter)
    last = {}
    occ = collections.defaultdict(collections.Counter)
    trips = collections.defaultdict(list)  # window -> (duration, maxdist)
    out_since, out_max = {}, {}
    for r in csv.DictReader(open(f'{d}/trace.csv')):
        f, i, x, y = int(r['frame']), int(r['id']), int(r['x']), int(r['y'])
        p = place(x, y, surf)
        w = f // WIN
        occ[w][p] += 1
        last[i] = (f, x, y, float(r['tank']), p)
        far = abs(x - NEST) > 12
        if far and i not in out_since:
            out_since[i], out_max[i] = f, 0
        if i in out_since:
            out_max[i] = max(out_max[i], abs(x - NEST))
            if not far:
                trips[w].append((f - out_since.pop(i), out_max.pop(i)))
    for r in csv.reader(open(f'{d}/events.csv')):
        if not r[0].isdigit():
            continue
        f, k = int(r[0]), r[2]
        w = f // WIN
        if k == 'death':
            i = int(r[1])
            c = CAUSE[int(float(r[5]))] if int(float(r[5])) < len(CAUSE) else r[5]
            pl = last.get(i, (0, 0, 0, 0, '?'))[4]
            W[w][f'death_{c}'] += 1
            W[w][f'death_{c}@{pl}'] += 1
        elif k in ('deliver', 'field', 'homeeat', 'dig', 'churn', 'fdrop', 'out', 'in', 'recut'):
            W[w][k] += 1
        elif k == 'eat':
            x, y = int(r[3]), int(r[4])
            W[w]['eatkJ@' + place(x, y, surf)] += float(r[5]) / 1000
    br = {int(r['frame']): r for r in csv.DictReader(open(f'{d}/brood.csv'))}
    lar = {int(r['frame']): r for r in csv.DictReader(open(f'{d}/larder.csv'))}
    nest = {int(r['frame']): r for r in csv.DictReader(open(f'{d}/nest.csv'))}
    hung = collections.defaultdict(collections.Counter)
    try:
        for r in csv.DictReader(open(f'{d}/hungry.csv')):
            hung[int(r['frame']) // WIN][place(int(r['x']), int(r['y']), surf)] += 1
    except FileNotFoundError:
        pass
    print(f'== {d}')
    print(' win  alive  laid hatch lstarv blost hdeny | deliv field homeeat  dig | eat kJ bowl/tun/door/fld | larder kJ | trips n med_dur p90_dur med_far | occ% bowl tun door fld wall | hungry rows bowl tun door fld wall | deaths')
    prevb = None
    for w in range(0, 8):
        fe = (w + 1) * WIN
        b = br.get(fe) or br.get(max(k for k in br if k <= fe))
        pb = prevb or {k: '0' for k in b}
        dl = lambda k: int(b[k]) - int(pb[k])
        prevb = b
        n = nest.get(fe, {}).get('alive', '?')
        L = lar.get(fe) or lar[max(k for k in lar if k <= fe)]
        lk = float(L['prov_kj']) + float(L['crumb_kj'])
        t = sorted(trips[w])
        md = t[len(t) // 2][0] if t else 0
        p9 = t[int(.9 * len(t))][0] if t else 0
        mf = sorted(x[1] for x in t)[len(t) // 2] if t else 0
        o = occ[w]; on = sum(o.values()) or 1
        h = hung[w]
        c = W[w]
        dth = ' '.join(f'{k[6:]}={v}' for k, v in sorted(c.items()) if k.startswith('death_') and '@' in k)
        print(f' {w:3d} {n:>5} {dl("laid"):5d} {dl("hatched"):5d} {dl("larvae_starved"):6d} {dl("brood_lost"):5d} {dl("hatch_denied"):5d} |'
              f' {c["deliver"]:5d} {c["field"]:5d} {c["homeeat"]:6d} {c["dig"]:4d} |'
              f' {c["eatkJ@bowl"]:5.0f}/{c["eatkJ@tunnel"]:4.0f}/{c["eatkJ@door"]:4.0f}/{c["eatkJ@field"]:5.0f} | {lk:7.0f} |'
              f' {len(t):5d} {md:6d} {p9:6d} {mf:4d} | {100*o["bowl"]/on:3.0f} {100*o["tunnel"]:3.0f}'.replace(f'{100*o["tunnel"]:3.0f}', f'{100*o["tunnel"]/on:3.0f}') +
              f' {100*o["door"]/on:3.0f} {100*o["field"]/on:3.0f} {100*o["wall"]/on:3.0f} | {h["bowl"]:4d} {h["tunnel"]:4d} {h["door"]:4d} {h["field"]:4d} {h["wall"]:4d} | {dth}')


for d in sys.argv[1:]:
    run(d)
