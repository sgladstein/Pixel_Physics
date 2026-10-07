"""spellfunnel.py RUN [LO HI] -- the lean spells that began below the old ground line, ant by ant, and how each ended
(Deep trace, 2026-10-07). Reads RUN/hspells.csv (hspells.awk over digrows), RUN/ledger.csv, RUN/bites.csv
(frame,id,x,y: every bite an ant under its grant took, from hungry.csv), RUN/stats.csv.

A spell starts at the first decision under half the grant and ends at the next one at or above the grant (fed), at the
ant's death (died: the ledger's cause, the death within 500 frames of the spell's last row), or at the run's end
(censored). Funnel per spell, then the same split per ant, then the died spells against the fed spells that never left
the nest, on what the rows say they were doing.
"""
import sys, csv, collections, statistics as st

RUN = sys.argv[1]
LO = int(sys.argv[2]) if len(sys.argv) > 2 else 200000
HI = int(sys.argv[3]) if len(sys.argv) > 3 else 300001
GROUND = 160

led = {r['id']: r for r in csv.DictReader(open(f'{RUN}/ledger.csv'))}
bites = collections.defaultdict(list)
for r in csv.DictReader(open(f'{RUN}/bites.csv')):
    bites[r['id']].append((int(r['frame']), int(r['x']), int(r['y'])))
store = {}
for r in csv.DictReader(open(f'{RUN}/stats.csv')):
    if r.get('nest_store_food') not in (None, ''):
        store[int(r['frame'])] = int(float(r['nest_store_food']))

sp = []
for r in csv.DictReader(open(f'{RUN}/hspells.csv')):
    s0 = int(r['start'])
    if not (LO <= s0 < HI):
        continue
    for k in ('start', 'end', 'last_row', 'rows', 'rows_nest', 'rows_out', 'fell', 'idle', 'stepped', 'p_store', 'p_hout',
              'p_soil', 'p_none', 'p_ns', 'p_other', 'h0', 'h1', 'h2', 'maxdepth', 'door_rows', 'a8sum', 'start_hy',
              'start_hx', 'start_age', 'last_hx', 'last_hy'):
        r[k] = int(float(r[k]))
    if r['how'] == 'open':
        L = led.get(r['id'])
        if L and L['died'] == '1' and int(L['end']) - r['last_row'] <= 500:
            r['how'] = 'died:' + L['cause']
            r['end'] = int(L['end'])
        else:
            r['how'] = 'censored'
    b = [x for x in bites.get(r['id'], []) if r['start'] <= x[0] <= r['end']]
    r['bites_in'] = sum(1 for x in b if x[2] > GROUND)
    r['bites_out'] = len(b) - r['bites_in']
    sp.append(r)

nest = [r for r in sp if r['start_zone'] == 'nest']
print(f'{RUN}: lean spells starting {LO}-{HI}: {len(sp)}; starting below the old ground line: {len(nest)}')


def funnel(rows, label):
    c = collections.Counter()
    for r in rows:
        left = 'left nest' if r['rows_out'] > 0 else 'stayed in'
        how = r['how'] if not r['how'].startswith('died') else ('starved' if r['how'] == 'died:STARVED' else 'died other')
        c[(left, how)] += 1
    n = len(rows)
    print(f'  {label}: {n} spells')
    for left in ('stayed in', 'left nest'):
        tot = sum(v for k, v in c.items() if k[0] == left)
        parts = ', '.join(f'{h} {c[(left, h)]}' for h in ('fed', 'starved', 'died other', 'censored') if c[(left, h)])
        print(f'    {left:<10} {tot:>5} ({100 * tot / max(1, n):4.1f}%): {parts}')


funnel(nest, 'per spell')
# per ant: its first such spell's ending is not the point; count ants by worst ending
ants = collections.defaultdict(list)
for r in nest:
    ants[r['id']].append(r)
ac = collections.Counter()
for i, rs in ants.items():
    hows = {x['how'] for x in rs}
    ac['starved' if 'died:STARVED' in hows else ('died other' if any(h.startswith('died') for h in hows) else ('censored' if 'censored' in hows else 'fed every time'))] += 1
print(f'  per ant ({len(ants)} ants with such a spell):', dict(ac))


def desc(rows, label):
    if not rows:
        print(f'  {label}: none')
        return
    n = len(rows)
    R = sum(r['rows'] for r in rows)

    def med(k):
        return st.median(r[k] for r in rows)

    def share(k):
        return 100 * sum(r[k] for r in rows) / max(1, R)
    newb = sum(1 for r in rows if r['start_age'] < 3000)
    wk = sum(1 for r in rows if r['worker'] == '1')
    print(f'  {label}: n={n}, rows={R}; median length {st.median(r["end"] - r["start"] for r in rows):.0f} fr; '
          f'start depth median {med("start_hy")}, deepest median {med("maxdepth")}; newborn (age<3k at start) {newb}; worker {wk}')
    print(f'      rows: store pull {share("p_store"):.1f}%, hungry out {share("p_hout"):.1f}%, soil way {share("p_soil"):.1f}%, '
          f'none {share("p_none"):.1f}%, not scored {share("p_ns"):.1f}%, other {share("p_other"):.1f}%')
    print(f'      rows: fell {share("fell"):.1f}%, idle {share("idle"):.1f}%, stepped {share("stepped"):.1f}%; jaws held {share("h2"):.1f}%, '
          f'crop held {share("h1"):.1f}%; in door columns {share("door_rows"):.1f}%; ants8 mean {sum(r["a8sum"] for r in rows) / max(1, R):.2f}')
    print(f'      bites in the spell: in nest {sum(r["bites_in"] for r in rows)} (spells with one: {sum(1 for r in rows if r["bites_in"])}), '
          f'outside {sum(r["bites_out"] for r in rows)} (spells with one: {sum(1 for r in rows if r["bites_out"])})')


print()
stayed_fed = [r for r in nest if r['rows_out'] == 0 and r['how'] == 'fed']
left_fed = [r for r in nest if r['rows_out'] > 0 and r['how'] == 'fed']
starved = [r for r in nest if r['how'] == 'died:STARVED']
desc(stayed_fed, 'FED, never left the nest')
desc(left_fed, 'FED, left the nest in the spell')
desc(starved, 'STARVED')
desc([r for r in starved if r['rows_out'] == 0], 'STARVED, never left the nest')
desc([r for r in starved if r['rows_out'] > 0], 'STARVED, left the nest in the spell')

# store size when the spells ended
def store_at(f):
    k = (f // 1000) * 1000
    return store.get(k, store.get(k - 1000, -1))

for lab, rows in (('fed, stayed in', stayed_fed), ('starved', starved)):
    v = [store_at(r['end']) for r in rows]
    if v:
        print(f'  store cells at the end of the spell, {lab}: median {st.median(v)}, zero on {sum(1 for x in v if x == 0)} of {len(v)}, under 8 on {sum(1 for x in v if x < 8)}')
