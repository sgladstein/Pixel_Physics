"""SCRATCH (chambers lane, eat seed): how each crop load a HUNGRY ant picks up at home ends, from a gzipped PP_MIND_LOG.
A load starts at a PickUp by an ant under half a tank (energy sense < 1/3) standing underground or at the nest, and ends
at the ant's first later decision where it is: a Drop with the crop held ("put back"), an Eat ("ate"), or the crop
empty with neither ("digested"). Also: the verb mix of every hungry decision with crop food held, and the energy those
ants reach 2,000 frames after the pick-up.
usage: eatloop.py MIND.gz RUNDIR"""
import gzip, sys, collections
V = ['walk', 'turn', 'rest', 'dig', 'pickup', 'drop', 'eat', 'feed', 'attack']
gz, d = sys.argv[1], sys.argv[2]
surf = [int(l) for l in open(f'{d}/surface.txt') if l.strip()]
open_load = {}          # id -> (frame, energy at pickup)
fate = collections.Counter(); mix = collections.Counter(); fed_mix = collections.Counter()
after = collections.defaultdict(list); watch = {}
n_rows = 0
with gzip.open(gz, 'rt') as f:
    for l in f:
        if l[0] != 'M': continue
        p = l.split(',', 11)
        fr, i, x, y, v = int(p[2]), int(p[3]), int(p[4]), int(p[5]), int(p[6])
        c = p[10].split(); e, crop, spoil, nest = float(c[0]), float(c[1]), float(c[2]), float(c[3])
        n_rows += 1
        if i in watch and fr >= watch[i][0] + 2000:
            after[watch[i][1]].append(e); del watch[i]
        if crop > 0:
            (mix if e < 1 / 3 else fed_mix)[V[v]] += 1
        if i in open_load:
            if crop > 0 and v == 5: fate['put back'] += 1; del open_load[i]
            elif crop > 0 and v == 6: fate['ate'] += 1; del open_load[i]; watch[i] = (fr, 'ate')
            elif crop == 0: fate['digested'] += 1; del open_load[i]; watch[i] = (fr, 'digested')
        if v == 4 and e < 1 / 3 and (y > surf[x] or nest > 0) and i not in open_load:
            open_load[i] = (fr, e)
tot = sum(fate.values())
print(f'{d}: {n_rows} decisions; hungry home pick-ups resolved {tot}: ' + ', '.join(f'{k} {v} ({100*v/max(1,tot):.0f}%)' for k, v in fate.most_common()))
h = sum(mix.values()); fd = sum(fed_mix.values())
print('  hungry, crop held, verb share:', ', '.join(f'{k} {100*v/h:.1f}%' for k, v in mix.most_common()), f'(n {h})')
print('  fed (>= half tank), crop held, verb share:', ', '.join(f'{k} {100*v/fd:.2f}%' for k, v in fed_mix.most_common()), f'(n {fd})')
for k, es in after.items():
    es.sort(); print(f'  energy sense 2000 frames after "{k}": median {es[len(es)//2]:.3f} (n {len(es)})')
