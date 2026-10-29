"""SCRATCH (chambers lane, eat seed): one row per arm x world x round of poolrounds.sh runs -- alive at the end,
starved (all / underground: below the surface, as chambers/summ.py places a death by its last trace row), nest space and
passage share (roomcopy/score.py's `thin`: share of nest space outside any 5x5 open block), and home eats (homeeat
events). Run from the repo root. usage: table.py BASE:ARM[,ARM..] ...   (reads BASE/ARM_wW/rR)"""
import sys, os, csv, collections
sys.path.insert(0, 'scratch/inrun'); sys.path.insert(0, 'scratch/nest'); sys.path.insert(0, 'scratch/nest/roomcopy')
import roomshape
roomshape.OPEN_ONLY = False
def thin(d):
    space, _, _, _ = roomshape.nest_cells(d); S = set(space); K = 2; room = set()
    for (x, y) in S:
        if all((x + a, y + b) in S for a in range(-K, K + 1) for b in range(-K, K + 1)):
            room.update((x + a, y + b) for a in range(-K, K + 1) for b in range(-K, K + 1))
    return len(S), (len(S) - len(room)) / max(len(S), 1)
def row(d):
    surf = [int(l) for l in open(f'{d}/surface.txt') if l.strip()]
    last = {}
    for r in csv.DictReader(open(f'{d}/trace.csv')): last[int(r['id'])] = (int(r['x']), int(r['y']))
    st = und = he = 0
    for r in csv.reader(open(f'{d}/events.csv')):
        if not r[0].isdigit(): continue
        if r[2] == 'death' and int(float(r[5])) == 1:
            st += 1; p = last.get(int(r[1]))
            und += bool(p and p[1] > surf[p[0]])
        elif r[2] == 'homeeat': he += 1
    alive = int(list(csv.DictReader(open(f'{d}/nest.csv')))[-1]['alive'])
    n, th = thin(d)
    return dict(alive=alive, starved=st, under=und, space=n, thin=th, homeeat=he)
for spec in sys.argv[1:]:
    base, arms = spec.split(':')
    for a in arms.split(','):
        for w in (68, 73, 61, 64):
            for r in range(4):
                d = f'{base}/{a}_w{w}/r{r}'
                if not os.path.exists(f'{d}/nest.csv'):
                    print(f'{a:5s} w{w} r{r} --'); continue
                x = row(d)
                print(f'{a:5s} w{w} r{r} alive {x["alive"]:3d} starved {x["starved"]:4d} under {x["under"]:4d} space {x["space"]:4d} '
                      f'passage {100*x["thin"]:3.0f}% homeeat {x["homeeat"]:5d}', flush=True)
