"""Spoil fall-back trace: where pellets are let go inside ('in'), and who picks them up again ('recut'), late in a round."""
import csv, glob, sys, collections, statistics as st
W0 = int(sys.argv[1]); dirs = sys.argv[2:]
surf_of = {}
tot = collections.Counter(); depth_in = []; dx_in = []; gap = []
for d in dirs:
    surf = [int(l) for l in open(d + '/surface.txt')]
    last_in = {}   # (x, y) -> (frame, dropper)
    for e in csv.DictReader(open(d + '/events.csv')):
        k = e['kind']; f = int(e['frame'])
        if k not in ('in', 'recut', 'out', 'dig'):
            continue
        x, y, i = int(e['x']), int(e['y']), int(e['id'])
        if k == 'in':
            last_in[(x, y)] = (f, i)
            if f >= W0:
                tot['in'] += 1; depth_in.append(y - surf[x]); dx_in.append(abs(x - 128))
        elif k == 'recut' and f >= W0:
            tot['recut'] += 1
            hit = None
            for r in range(3):
                for dy in range(-r, r + 1):
                    for ddx in range(-r, r + 1):
                        if (x + ddx, y + dy) in last_in and hit is None:
                            hit = last_in[(x + ddx, y + dy)]
            if hit is None:
                tot['recut of soil nobody let go (fell/slid)'] += 1
            else:
                tot['recut of a let-go pellet'] += 1
                tot['... by the same ant that let it go'] += hit[1] == i
                gap.append(f - hit[0])
        elif k == 'out' and f >= W0:
            tot['out'] += 1
print(f'from frame {W0}, {len(dirs)} colonies, per colony:')
for k, v in tot.items():
    print(f'  {k:45s} {v / len(dirs):7.0f}')
q = lambda v: f'median {st.median(v):.0f}, p25 {sorted(v)[len(v)//4]}, p75 {sorted(v)[3*len(v)//4]}'
print('  let go inside: rows below ground', q(depth_in), '| columns from door', q(dx_in))
print('  frames from let-go to pick-up again:', q(gap), f'| within 100 frames {sum(g <= 100 for g in gap)/len(gap):.0%}')
