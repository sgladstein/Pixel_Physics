"""SCRATCH (nest lane, 2026-10-27): where a colony eats, from inrun's events.csv (eat rows carry the eater's head since
2026-10-27). home = within 12 columns of the door and at or below the datum (the nest bowl, as the hungry probe and the
home-eat crutch define it); door = within 12 columns above the datum; field = elsewhere. Also the larder at the end
(larder.csv: crumb cells and kJ) and how many starved.
usage: eatwhere.py RUNDIR [RUNDIR...] [--to FRAME]"""
import csv, sys

args = [a for a in sys.argv[1:] if not a.startswith('--')]
TO = int(sys.argv[sys.argv.index('--to') + 1]) if '--to' in sys.argv else 10 ** 9
args = [a for a in args if a != str(TO)]
NEST = 128
print('run                      eaten kJ   home%   door%  field%   crumbs  crumb kJ  starved  deliveries')
for d in args:
    surf = [int(l) for l in open(f'{d}/surface.txt') if l.strip()]
    kj = dict(home=0.0, door=0.0, field=0.0)
    starved = deliv = 0
    for l in open(f'{d}/events.csv'):
        p = l.split(',')
        if len(p) < 6 or not p[0].isdigit() or int(p[0]) > TO:
            continue
        if p[2] == 'eat':
            x, y, v = int(p[3]), int(p[4]), float(p[5])
            near = abs(x - NEST) <= 12
            kj['home' if near and y >= surf[x] else 'door' if near else 'field'] += v / 1000
        elif p[2] == 'death' and p[5].strip() == '1':
            starved += 1
        elif p[2] == 'deliver':
            deliv += 1
    lar = [r for r in csv.DictReader(open(f'{d}/larder.csv')) if int(r['frame']) <= TO][-1]
    t = sum(kj.values()) or 1
    print(f"{d.rstrip('/').split('/')[-1]:24s} {t:9.0f} {100 * kj['home'] / t:7.1f} {100 * kj['door'] / t:7.1f} {100 * kj['field'] / t:7.1f}"
          f"  {lar['crumb_cells']:>7s} {float(lar['crumb_kj']):9.0f} {starved:8d} {deliv:11d}")
