"""SCRATCH (pheromones lane, 2026-10-28): **food carried home from the heap** -- deliveries whose trip began with a
pickup within `WINDOW` columns of the patch then current (`patches.csv`). The score the door loop cannot farm: inrun's
`deliver` (nest.csv `food_home`) also counts a delivered cell picked straight back up at the nest edge and put down
again (`evolvable-pheromones-2026-10-25.md` §9.13: 42% of PF4's deaf best's deliveries), but such a trip starts at the
door, not the heap. Each ant's pickups queue in order; each deliver / churn / field drop pops the oldest."""
import collections, csv

WINDOW = 15


def heaphome(d):
    heaps = [(int(r['frame']), int(r['x'])) for r in csv.DictReader(open(f'{d}/patches.csv'))]
    q = collections.defaultdict(collections.deque)
    n = 0
    with open(f'{d}/events.csv') as f:
        for r in csv.reader(f):
            if len(r) < 6 or r[0] == 'frame':
                continue
            k = r[2]
            if k == 'field':
                fr = int(r[0])
                hx = heaps[0][1]
                for at, x in heaps:
                    if at > fr:
                        break
                    hx = x
                q[r[1]].append(abs(int(r[3]) - hx) <= WINDOW)
            elif k in ('deliver', 'churn', 'fdrop'):
                t = q[r[1]].popleft() if q[r[1]] else False
                n += k == 'deliver' and t
    return n
