"""zonedeaths.py RUN [LO HI ZONE] -- every ant that starved in ZONE (ledger zone_end: nest, mound, surface, heap) in
[LO, HI), traced from its own decision rows (Deep trace, 2026-10-07; seed 2's mound-tunnel deaths under NEST_STORE
`edible`). nestdeaths.py does the same for ZONE=nest only.

Needs a deeptrace run with dig=1 hungry=1 (digrows.csv.gz, hungry.csv.gz, ledger.csv, stats.csv). Extracts the dying
ants' rows once with awk (RUN/zd_<ZONE>_rows.csv, _hungry.csv), then per ant, over its last spell (the rows after its
last decision at or above the grant):
  where the spell began (zone, row), whether it was ever in the nest in the spell, highest and lowest row
  pulls and outcomes as shares of the spell's rows; the last row's pull, outcome and head
  bites in the spell (hungry.csv's bite column); store cells (stats nest_store_food) at the spell's start and at death
Prints a row per ant, then the buckets by where the spell began. Measures; the bucket names describe the rows.
"""
import sys, os, csv, subprocess, collections, statistics as st

RUN = sys.argv[1]
LO = int(sys.argv[2]) if len(sys.argv) > 2 else 200000
HI = int(sys.argv[3]) if len(sys.argv) > 3 else 300001
ZONE = sys.argv[4] if len(sys.argv) > 4 else 'mound'
GROUND = 160

led = list(csv.DictReader(open(f'{RUN}/ledger.csv')))
dead = [r for r in led if r['died'] == '1' and r['cause'] == 'STARVED' and LO <= int(r['end']) < HI]
pick = [r for r in dead if r['zone_end'] == ZONE]
print(f'{RUN}: starved {len(dead)} in [{LO},{HI}); by zone at death: {dict(collections.Counter(r["zone_end"] for r in dead))}')
print(f'  per 10k, {ZONE}:', dict(sorted(collections.Counter(int(r["end"]) // 10000 * 10 for r in pick).items())))
if not pick:
    sys.exit()
ids = {r['id'] for r in pick}
idf = f'{RUN}/zd_{ZONE}_ids.txt'
open(idf, 'w').write('\n'.join(sorted(ids)) + '\n')
rows_f, hung_f, tag = f'{RUN}/zd_{ZONE}_rows.csv', f'{RUN}/zd_{ZONE}_hungry.csv', f'{RUN}/zd_{ZONE}_tag.txt'
want_tag = f'{LO},{HI},{len(ids)}'
if not (os.path.exists(tag) and open(tag).read() == want_tag):
    for src, dst in ((f'{RUN}/digrows.csv.gz', rows_f), (f'{RUN}/hungry.csv.gz', hung_f)):
        cmd = f"zcat {src} | awk -F, 'NR==FNR{{ids[$1];next}} FNR==1 || ($2 in ids)' {idf} - > {dst}"
        subprocess.run(cmd, shell=True, check=True)
    open(tag, 'w').write(want_tag)

by = collections.defaultdict(list)
for r in csv.DictReader(open(rows_f)):
    if r.get('energy') not in (None, '') and r.get('hy') not in (None, ''):
        by[r['id']].append(r)
hb = collections.defaultdict(list)
for r in csv.DictReader(open(hung_f)):
    hb[r['id']].append(r)
store = {}
for r in csv.DictReader(open(f'{RUN}/stats.csv')):
    if r.get('nest_store_food') not in (None, ''):
        store[int(r['frame'])] = int(float(r['nest_store_food']))
sk = sorted(store)


def store_at(f):
    import bisect
    j = bisect.bisect_right(sk, f) - 1
    return store[sk[j]] if j >= 0 else -1


out = []
for r in sorted(pick, key=lambda r: int(r['end'])):
    i, end = r['id'], int(r['end'])
    rows = by.get(i, [])
    if not rows:
        out.append(dict(id=i, end=end, n=0))
        continue
    k_fed = max((k for k, x in enumerate(rows) if float(x['energy']) >= 1.0), default=-1)
    sp = rows[k_fed + 1:] or rows[-1:]
    s0 = int(sp[0]['frame'])
    n = len(sp)
    pc = collections.Counter(x['pull_why'] for x in sp)
    oc = collections.Counter(x['outcome'] for x in sp)
    zc = collections.Counter(x['zone'] for x in sp)
    hys = [int(x['hy']) for x in sp]
    bites = [x for x in hb.get(i, []) if int(x['frame']) >= s0 and x.get('bite')]
    out.append(dict(id=i, end=end, n=n, worker=sp[0].get('worker', ''), s0=s0, slen=end - s0,
                    z0=sp[0]['zone'], hy0=int(sp[0]['hy']), hx0=int(sp[0]['hx']), e0=float(sp[0]['energy']),
                    in_nest=zc.get('nest', 0), zc=zc, top=min(hys), bottom=max(hys), pc=pc, oc=oc,
                    last=sp[-1], bites=len(bites), st0=store_at(s0), st1=store_at(end), fed_before=k_fed >= 0))

print(f'  {len(out)} ants; per ant (spell = rows after the last row at or above the grant):')
print('  id     end     spell  worker start(zone,x,y,e)       nest%  top..bot  fell% idle% | last: x,y zone pull/outcome | bites | store cells start->death | top pulls')
for o in out:
    if not o['n']:
        print(f"  {o['id']:>6} {o['end']:>7}  no decision rows")
        continue
    L = o['last']
    tp = ', '.join(f'{k} {100 * v / o["n"]:.0f}%' for k, v in o['pc'].most_common(3))
    print(f"  {o['id']:>6} {o['end']:>7} {o['slen']:>6} {o['worker']:>3}  {o['z0']:>7},{o['hx0']},{o['hy0']},{o['e0']:.2f}"
          f"  {100 * o['in_nest'] / o['n']:5.1f}  {o['top']}..{o['bottom']}  {100 * o['oc'].get('fell', 0) / o['n']:4.1f}"
          f" {100 * sum(v for k, v in o['oc'].items() if 'idle' in k) / o['n']:4.1f} | {L['hx']},{L['hy']} {L['zone']}"
          f" {L['pull_why']}/{L['outcome']} | {o['bites']} | {o['st0']}->{o['st1']} | {tp}")

print('\n  buckets by where the last spell began:')
bk = collections.defaultdict(list)
for o in out:
    if o['n']:
        bk[o['z0'] + (' (below ground)' if o['hy0'] > GROUND else '')].append(o)
for b, L in sorted(bk.items(), key=lambda kv: -len(kv[1])):
    R = sum(o['n'] for o in L)
    pc = collections.Counter()
    oc = collections.Counter()
    for o in L:
        pc.update(o['pc'])
        oc.update(o['oc'])
    print(f'  {b}: {len(L)} ants, {R} rows; median spell {st.median(o["slen"] for o in L):.0f} fr;'
          f' ever in the nest in the spell {sum(1 for o in L if o["in_nest"])}; bites in spell {sum(o["bites"] for o in L)};'
          f' store under 8 at spell start {sum(1 for o in L if 0 <= o["st0"] < 8)}, at death {sum(1 for o in L if 0 <= o["st1"] < 8)}')
    print('     pulls:', ', '.join(f'{k} {100 * v / R:.1f}%' for k, v in pc.most_common(6)))
    print('     outcomes:', ', '.join(f'{k} {100 * v / R:.1f}%' for k, v in oc.most_common(5)))
    print('     last head rows:', sorted(int(o['last']['hy']) for o in L))
