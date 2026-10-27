"""SCRATCH: summary for judge.sh (teacher lane, 2026-10-21)."""
import sys, os, csv
O = sys.argv[1]
def ev(d):
    return list(csv.DictReader(open(d + '/events.csv'))) if os.path.exists(d + '/events.csv') else []
dep = {}; starved = 0
for arm in ('st', 'ctl'):
    n = 0
    for s in range(1, 9):
        for e in ev(f'{O}/{arm}_{s}'):
            f = int(e['frame'])
            if 20000 < f <= 26000:
                n += e['kind'] == 'hungryout'
                if arm == 'st' and e['kind'] == 'death' and e['v'] == '1':
                    starved += 1
    dep[arm] = n
alive20 = sum(sum(1 for _ in open(f'{O}/st_{s}/ants_f0020000.txt')) for s in range(1, 9) if os.path.exists(f'{O}/st_{s}/ants_f0020000.txt'))
surv = []; dl = []
for s in range(11, 15):
    p = f'{O}/clone_{s}/ants_f0200000.txt'
    surv.append(sum(1 for _ in open(p)) if os.path.exists(p) else 0)
    dl.append(sum(e['kind'] == 'deliver' for e in ev(f'{O}/clone_{s}')))
ext = 0; ok = 0; nests = []
for s in range(1, 9):
    d = f'{O}/sel_{s}'
    if os.path.exists(d + '/EXTINCT'):
        ext += 1
    elif os.path.exists(d + '/r2/ants_f0200000.txt') and sum(1 for _ in open(d + '/r2/ants_f0200000.txt')) >= 12:
        ok += 1
    if os.path.exists(d + '/r2/nest.csv'):
        last = list(csv.DictReader(open(d + '/r2/nest.csv')))
        if last: nests.append(round(float(last[-1]['nest_score']), 1))
print(f"{os.path.basename(O)}: hunger probe departures {dep['st']} hungry vs {dep['ctl']} fed, starved {starved} of {alive20} | "
      f"clone colonies survivors {surv} (mean {sum(surv)/4:.1f} of 24), deliveries {dl} | "
      f"in-run 3x200k: {ok} of 8 colonies healthy at end (12+ ants), {ext} extinct; final nest scores {nests}")
