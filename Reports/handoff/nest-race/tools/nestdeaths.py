"""nestdeaths.py RUN [LO HI] -- every ant that starved below the old ground line in [LO, HI), traced from its own
decision rows (Deep trace, 2026-10-07; the residual late nest deaths under WAY_FOOT + backfill).

Needs a deeptrace run with dig=1 hungry=1 (digrows.csv.gz, hungry.csv.gz, ledger.csv, stats.csv). Extracts the
dying ants' rows once with awk (RUN/nd_rows.csv, RUN/nd_hungry.csv) and then, per ant:
  spell     -- frames from the last decision at or above the grant (energy >= 1) to the death; 'never' if none
  rows by pull reason in the spell, split by what the jaws/crop held (0 nothing, 1 crop, 2 jaws)
  where     -- zone shares in the spell, deepest row, last head, steps from the door (way_d) at the end
  outcome   -- stepped / fell / idle shares, and whether it left the nest (any zone not 'nest') in the spell
  bites     -- bites in the spell (hungry.csv's bite column) and where
Prints a table per ant and the counts per bucket. Measures; the bucket names are descriptions of the rows.
"""
import sys, os, csv, gzip, subprocess, collections

RUN = sys.argv[1]
LO = int(sys.argv[2]) if len(sys.argv) > 2 else 200000
HI = int(sys.argv[3]) if len(sys.argv) > 3 else 300001

led = list(csv.DictReader(open(f'{RUN}/ledger.csv')))
dead = [r for r in led if r['died'] == '1' and r['cause'] == 'STARVED' and LO <= int(r['end']) < HI]
nest = [r for r in dead if r['zone_end'] == 'nest']
print(f'{RUN}: starved {len(dead)} in [{LO},{HI}), {len(nest)} with the head below the old ground line (zone nest)')
zc = collections.Counter(r['zone_end'] for r in dead)
print('  by zone at death:', dict(zc))
ids = {r['id'] for r in nest}
idf = f'{RUN}/nd_ids.txt'
open(idf, 'w').write('\n'.join(sorted(ids)) + '\n')

rows_f = f'{RUN}/nd_rows.csv'
hung_f = f'{RUN}/nd_hungry.csv'
tag = f'{RUN}/nd_tag.txt'
want_tag = f'{LO},{HI},{len(ids)}'
if not (os.path.exists(tag) and open(tag).read() == want_tag):
    for src, dst in ((f'{RUN}/digrows.csv.gz', rows_f), (f'{RUN}/hungry.csv.gz', hung_f)):
        cmd = f"zcat {src} | awk -F, 'NR==FNR{{ids[$1];next}} FNR==1 || ($2 in ids)' {idf} - > {dst}"
        subprocess.run(cmd, shell=True, check=True)
    open(tag, 'w').write(want_tag)

by = collections.defaultdict(list)
with open(rows_f) as fh:
    rd = csv.DictReader(fh)
    for r in rd:
        by[r['id']].append(r)
hb = collections.defaultdict(list)
with open(hung_f) as fh:
    rd = csv.DictReader(fh)
    for r in rd:
        hb[r['id']].append(r)

# store cells at each census frame (stats.csv nest_store_food), to read the store at the death
store = {}
for r in csv.DictReader(open(f'{RUN}/stats.csv')):
    if r.get('nest_store_food') not in (None, ''):
        store[int(r['frame'])] = int(float(r['nest_store_food']))


def store_at(f):
    k = (f // 1000) * 1000
    return store.get(k, store.get(k - 1000, -1))


out = []
for r in sorted(nest, key=lambda r: int(r['end'])):
    i = r['id']
    rows = by.get(i, [])
    if not rows:
        out.append(dict(id=i, end=int(r['end']), age=int(r['end']) - int(r['born']), n=0))
        continue
    end = int(r['end'])
    born = int(r['born'])
    # spell: rows after the last row with energy >= 1
    last_fed_idx = max((k for k, x in enumerate(rows) if float(x['energy']) >= 1.0), default=-1)
    spell = rows[last_fed_idx + 1:]
    spell_start = int(spell[0]['frame']) if spell else end
    e0 = float(spell[0]['energy']) if spell else float('nan')
    pc = collections.Counter((x['pull_why'], x['hold']) for x in spell)
    zc = collections.Counter(x['zone'] for x in spell)
    oc = collections.Counter(x['outcome'] for x in spell)
    left = any(x['zone'] != 'nest' for x in spell)
    hs = [x for x in hb.get(i, []) if int(x['frame']) >= spell_start]
    bites = [x['bite'] for x in hs if x['bite']]
    last = rows[-1]
    deep = max(int(x['hy']) for x in spell) if spell else int(last['hy'])
    wd = [int(x['way_d']) for x in spell[-50:] if x['way_d'] not in ('',)]
    a8 = [int(x['ants8']) for x in spell if x['ants8'] != '']
    shut = [x['shut_in'] for x in spell if x['shut_in'] != '']
    o = dict(id=i, end=end, age=end - born, born=born, n=len(spell), spell=end - spell_start if last_fed_idx >= 0 else None,
             e0=e0, fed_ever=last_fed_idx >= 0, pulls=pc, zones=zc, outc=oc, left=left, bites=bites, hx=int(last['hx']),
             hy=int(last['hy']), deep=deep, way_d_end=(wd[-1] if wd else None), a8=(sum(a8) / len(a8) if a8 else 0),
             shut=(shut.count('1') / len(shut) if shut else None), worker=last['worker'], store=store_at(end),
             hold_end=last['hold'], pull_end=last['pull_why'])
    out.append(o)

print()
print('id        end     age    born   spell  e0    rows  fell%  idle%  left bites hold_end pull_end            last(x,y) deep way_d ants8 shut% store top pulls (reason/hold: rows)')
for o in out:
    if o['n'] == 0:
        print(f"{o['id']:>8} {o['end']:>7} {o['age']:>6}  (no rows)")
        continue
    n = max(1, o['n'])
    top = ', '.join(f'{k[0]}/{k[1]}:{v}' for k, v in o['pulls'].most_common(4))
    shs = '' if o['shut'] is None else f"{100 * o['shut']:.0f}"
    print(f"{o['id']:>8} {o['end']:>7} {o['age']:>6} {o['born']:>7} {str(o['spell']):>6} {o['e0']:.2f} {o['n']:>6} {100*o['outc']['fell']/n:5.1f} {100*o['outc']['roll_failed_idle']/n:5.1f}  {int(o['left'])}  {len(o['bites']):>4}  {o['hold_end']:>3}  {o['pull_end']:<18} ({o['hx']},{o['hy']}) {o['deep']:>4} {str(o['way_d_end']):>5} {o['a8']:5.2f} {shs:>5} {o['store']:>4} {top}")

# pooled over the dying ants: rows in the final spell by pull reason and hold
print()
tot = collections.Counter()
for o in out:
    if o['n']:
        tot.update(o['pulls'])
T = sum(tot.values())
print(f'final-spell rows, all {len(out)} ants pooled ({T} rows), by pull reason / what was held (0 none, 1 crop, 2 jaws):')
for k, v in tot.most_common(14):
    print(f'  {k[0]:<20} {k[1]}  {v:>7}  {100*v/T:5.1f}%')
