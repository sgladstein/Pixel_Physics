"""SCRATCH (chambers crumbs lane, 2026-10-30): rank the crumb sources by cells standing in the nest at the end of each
round (standing state, not event rate), pooled over the run folders given, split by where the crumb lies.
Source of the last put-down on each standing crumb (crumbsrc.py's join):
  fed re-handle      load picked up at home by an ant over half a tank, put down again
  hungry re-handle   the same under half a tank
  forager delivery   load picked up in the field, carried home (by its own put-down; ate = it Ate from the crop since)
plus, for re-handles, how far the crumb moved (pick-up cell of the load's first cell to landing cell, Manhattan).
usage: rank.py RUNDIR:LOG [RUNDIR:LOG ...]
"""
import collections, os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import crumbsrc

tot = collections.Counter(); mv = collections.Counter(); n = 0; joined = 0
for a in sys.argv[1:]:
    d, logp = a.split(':')
    c = crumbsrc.classify(d, logp)
    n += c['crumbs']
    for r in c['rows']:
        joined += 1
        if r['first_home']:
            s = 'fed re-handle' if r['energy'] >= 0.5 else 'hungry re-handle'
            m = abs(r['x'] - r['first'][0]) + abs(r['ly'] - r['first'][1])
            mv[(r['_place'], 'moved <5' if m < 5 else 'moved 5-14' if m < 15 else 'moved 15+')] += 1
        else:
            s = 'forager delivery' + (' (ate some)' if r['eats'] else '')
        tot[(s, r['_place'])] += 1
        tot[(s, 'all')] += 1
        tot[('all', r['_place'])] += 1
print(f'{len(sys.argv) - 1} runs; standing crumbs {n}, joined {joined} ({100 * joined / max(1, n):.0f}%)')
places = ['neck', 'passage', 'room', 'all']
print(f'{"source":28}' + ''.join(f'{p:>16}' for p in places))
for s in sorted({k[0] for k in tot if k[0] != 'all'}, key=lambda s: -tot[(s, 'all')]):
    print(f'{s:28}' + ''.join(f'{tot[(s, p)]:7} ({100 * tot[(s, p)] / max(1, tot[("all", p)] if p != "all" else joined):3.0f}%)' for p in places))
print(f'{"all joined":28}' + ''.join(f'{(tot[("all", p)] if p != "all" else joined):16}' for p in places))
print('re-handles by distance moved:', ', '.join(f'{k[0]}/{k[1]} {v}' for k, v in sorted(mv.items())))
