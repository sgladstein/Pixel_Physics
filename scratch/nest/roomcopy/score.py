import sys, os, csv, json
sys.path.insert(0, 'scratch/inrun'); sys.path.insert(0, 'scratch/nest')
import roomshape
from width import width
roomshape.OPEN_ONLY = False
def thin(d):
    space, _, _, _ = roomshape.nest_cells(d); S = set(space); K = 2; room = set()
    for (x, y) in S:
        if all((x + a, y + b) in S for a in range(-K, K + 1) for b in range(-K, K + 1)):
            room.update((x + a, y + b) for a in range(-K, K + 1) for b in range(-K, K + 1))
    return len(S), (len(S) - len(room)) / max(len(S), 1)
base = sys.argv[1]; R = {}
for a in sys.argv[2].split(','):
    for s in range(61, 69):
        for r in range(12):
            d = f'{base}/{a}_{s}/r{r}'
            if not os.path.exists(d + '/.done'): continue
            n, th = thin(d); w = width(d)
            nest = list(csv.DictReader(open(d + '/nest.csv')))
            ev = [l.split(',') for l in open(d + '/events.csv')][1:]
            R[f'{a}_{s}_{r}'] = dict(space=n, thin=round(th, 3), width=w['width'], open=w['open'], alive=int(nest[-1]['alive']),
                                     starved=sum(1 for e in ev if e[2] == 'death' and e[5] == '1'))
json.dump(R, open(base + '/result.json', 'w'))
