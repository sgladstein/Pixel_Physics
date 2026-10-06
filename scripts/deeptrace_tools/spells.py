"""spells.py RUN... [--from F]: every ant's shut-in spells and how each ended (Deep trace's own count, 2026-10-06).

Written independently of Nest building's shutep.py, from the definition in nest-race/lane3/mound-out-2026-10-06.md:
a shut-in spell is an ant's run of consecutive 1k census samples (from 20k) in which its head is behind a sealed door,
in a closed pocket, or encased (starvewhere.py's space rule, imported from beside this file). A spell ends:
  out      -- the ant's next census sample is not shut in
  starved  -- the ant has no next sample and died of cause=STARVED (alone) before it
  other    -- the ant has no next sample and died of something else
  censored -- the run ended (300k) while the ant was still shut in
Prints per run: spells, each ending, the starved share, and the share split by whether the spell began holding a pellet;
the same starvers divided two other ways (per ant ever shut in, per 1k shut-in census samples). --csv DIR writes one row
per spell. Checked 2026-10-06: reproduces Nest building's shutep.py exactly on baseline ecca174e6 s1-8 (22,606 spells,
293 starved) and MOUND_OUT=dig s1-8 (13,915, 69), written from the definition without reading shutep.py.
"""
import os, sys, re, csv, collections
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import starvewhere as S

TRAPPED = S.TRAPPED


def spells(run, lo=20000):
    deaths = {}
    for L in open(run + '/events.txt'):
        if ' DIED ' in L:
            f = int(L.split()[0])
            i = re.search(r' id=(\d+)', L).group(1)
            cause = L.rstrip().split('cause=', 1)[1]
            deaths[i] = (f, cause)
    byframe = collections.defaultdict(list)
    for r in csv.DictReader(open(run + '/colony.csv')):
        f = int(r['frame'])
        if f >= lo:
            byframe[f].append(r)
    frames = sorted(byframe)
    last_frame = frames[-1]
    open_spell = {}          # id -> [start frame, last frame, began with pellet]
    seen_prev = set()        # ids sampled on the previous frame
    done = []                # (ending, began_with_pellet, length in samples)
    for f in frames:
        sp = S.spaces(f'{run}/map_f{f:06d}.txt')
        here = set()
        for r in byframe[f]:
            i = r['id']; here.add(i)
            shut = S.bucket(sp, int(r['hx']), int(r['hy'])) in TRAPPED
            if shut:
                if i in open_spell and open_spell[i][1] == f - 1000:
                    open_spell[i][1] = f
                else:
                    if i in open_spell:   # a gap in its samples should not happen; close it as out
                        s = open_spell.pop(i); done.append(('out', s[2], (s[1] - s[0]) // 1000 + 1, i, s[0], s[1]))
                    open_spell[i] = [f, f, r['spoil'] != '0']
            elif i in open_spell:
                s = open_spell.pop(i); done.append(('out', s[2], (s[1] - s[0]) // 1000 + 1, i, s[0], s[1]))
        # ants with an open spell that were not sampled on this frame died after their last sample
        for i in [i for i in open_spell if i not in here and open_spell[i][1] == f - 1000]:
            s = open_spell.pop(i)
            d = deaths.get(i)
            if d is None:
                end = 'vanished'
            elif s[1] < d[0] <= f:
                end = 'starved' if d[1] == 'STARVED' else 'other'
            else:
                end = 'death out of window'
            done.append((end, s[2], (s[1] - s[0]) // 1000 + 1, i, s[0], s[1]))
    for i, s in open_spell.items():
        d = deaths.get(i)
        if d is not None and s[1] < d[0]:
            done.append(('starved' if d[1] == 'STARVED' else 'other', s[2], (s[1] - s[0]) // 1000 + 1, i, s[0], s[1]))
        else:
            done.append(('censored', s[2], (s[1] - s[0]) // 1000 + 1, i, s[0], s[1]))
    return done


def main():
    a = sys.argv[1:]
    lo = int(a[a.index('--from') + 1]) if '--from' in a else 20000
    csvdir = a[a.index('--csv') + 1] if '--csv' in a else None
    runs = [x for j, x in enumerate(a) if not x.startswith('--') and (j == 0 or a[j - 1] not in ('--from', '--csv'))]
    tot = collections.Counter()
    for run in runs:
        d = spells(run, lo)
        c = collections.Counter(x[0] for x in d)
        name = '/'.join(run.rstrip('/').split('/')[-2:])
        if csvdir:
            import os
            os.makedirs(csvdir, exist_ok=True)
            with open(os.path.join(csvdir, name.replace('/', '-') + '.csv'), 'w') as fo:
                fo.write('id,start,last,samples,ending,pellet\n')
                for x in d:
                    fo.write(f'{x[3]},{x[4]},{x[5]},{x[2]},{x[0]},{int(x[1])}\n')
        pel = [x for x in d if x[1]]; emp = [x for x in d if not x[1]]
        sh = lambda xs: f"{100 * sum(x[0] == 'starved' for x in xs) / max(1, len(xs)):.1f}%"
        ants = {x[3] for x in d}; ants_st = {x[3] for x in d if x[0] == 'starved'}
        expo = sum(x[2] for x in d)
        print(f"{name}: spells {len(d)}, starved {c['starved']} ({sh(d)}), "
              f"out {c['out']}, other death {c['other']}, censored {c['censored']}"
              + ''.join(f", {k} {c[k]}" for k in ('vanished', 'death out of window') if c[k])
              + f"; began with a pellet {len(pel)} ({sh(pel)} starved), empty {len(emp)} ({sh(emp)} starved)"
              + f"; ants ever shut in {len(ants)} ({100 * len(ants_st) / max(1, len(ants)):.2f}% starved shut in)"
              + f"; shut-in samples {expo} ({1000 * c['starved'] / max(1, expo):.2f} starved per 1k)")
        tot['spells'] += len(d); tot['starved'] += c['starved']; tot['ants'] += len(ants); tot['ants_st'] += len(ants_st); tot['expo'] += expo
    if len(runs) > 1:
        print(f"TOTAL: spells {tot['spells']}, starved {tot['starved']} ({100 * tot['starved'] / max(1, tot['spells']):.2f}%); "
              f"ants ever shut in {tot['ants']} ({100 * tot['ants_st'] / max(1, tot['ants']):.2f}% starved); "
              f"shut-in samples {tot['expo']} ({1000 * tot['starved'] / max(1, tot['expo']):.2f} starved per 1k). "
              f"Pooled over runs: read the runs one by one when their spell counts differ much.")


if __name__ == '__main__':
    main()
