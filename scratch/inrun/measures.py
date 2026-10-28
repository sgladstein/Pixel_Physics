"""SCRATCH (selection lane, 2026-10-27): which colony measures REPEAT, so colony selection can rank on skill, not luck.

The selection review (Reports/brain-training/research/selection-review-2026-10-27.md) found colony survival repeats
from one world to the next at only 0.05-0.09, and starvation, the main goal's own measure, at 0.03 (clean1's k3,
selaudit.py). A case that does not repeat spends its vote on the world. This script computes candidate measures from
what every colsel round folder already keeps (stock.csv, brood.csv, larder.csv, nest.csv, births.csv -- colsel drops
only genomes and event logs), so it needs no new run, and scores each on two things:

  retest  Spearman of the measure across colonies kept twice (elite=1: the same carry, unmutated, in two fresh worlds
          r and r+1), averaged over round pairs, with a 90% bootstrap interval: how much the COLONY sets it.
  world   (with --pair A B, two arms run on the same seeds) Spearman between arms of the same colony index and round,
          which play the same world: how much the WORLD sets it. A good case has high retest and low world.

usage: measures.py ARM_DIR [ARM_DIR ...]        ARM_DIR: a colsel OUT (colsel.csv + c<k>/r<r>/ folders)
       measures.py --pair ARM_A ARM_B
       measures.py --csv ARM_DIR > m.csv         the per colony-round table, for anything else
"""
import csv, os, random, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from selaudit import load, spearman, ci, kept_twice  # noqa: E402


def rows(path):
    try:
        return list(csv.DictReader(open(path)))
    except OSError:
        return []


def measures(rdir, base):
    """Candidate measures of one colony-round. base: its colsel.csv row (home8, starved, alive, nest ...)."""
    m = {k: base[k] for k in ('home8', 'starved', 'alive', 'nest', 'scale', 'lab') if k in base}
    st = rows(f'{rdir}/stock.csv')
    br = rows(f'{rdir}/brood.csv')
    ld = rows(f'{rdir}/larder.csv')
    ns = rows(f'{rdir}/nest.csv')
    if not st:
        return None
    half = [r for r in st if int(r['frame']) > int(st[-1]['frame']) // 2]
    alive = [int(r['alive']) for r in st]
    m['alive_mean'] = sum(alive) / len(alive)
    m['alive_min2'] = min(int(r['alive']) for r in half) if half else 0
    tm = [float(r['tank_mean']) for r in half if int(r['alive']) > 0]
    m['tank_mean2'] = sum(tm) / len(tm) if tm else 0.0
    tmin = [float(r['tank_min']) for r in half if int(r['alive']) > 0]
    m['tank_min2'] = sum(tmin) / len(tmin) if tmin else 0.0
    hatched = int(br[-1]['hatched']) if br else 0
    m['hatched'] = hatched
    # starved per ant that lived in the round: a big colony starves more ants at the same skill
    lived = alive[0] + hatched
    m['starved_per_ant'] = base['starved'] / max(1, lived)
    m['starved_per_hatch'] = base['starved'] / max(1, hatched)
    m['larvae_starved'] = int(br[-1]['larvae_starved']) if br else 0
    m['ate_kj'] = float(br[-1]['ate_kj']) if br and 'ate_kj' in br[-1] else 0.0
    if ld:
        e = ld[-1]
        m['store_cells'] = int(e['prov_cells']) + int(e['crumb_cells']) + int(e['other_cells'])
        m['store_kj'] = float(e['prov_kj']) + float(e['crumb_kj']) + float(e['other_kj'])
        m['crumb_share'] = int(e['crumb_cells']) / max(1, m['store_cells'])
        sk = [float(r['prov_kj']) + float(r['crumb_kj']) + float(r['other_kj']) for r in ld if int(r['frame']) > int(ld[-1]['frame']) // 2]
        m['store_kj_mean2'] = sum(sk) / len(sk) if sk else 0.0
    if ns:
        m['food_home'] = int(ns[-1]['food_home'])
    hm = rows(f'{rdir}/home.csv')  # colsel.py since 2026-10-27: home8 in kJ (the cells' worth as put down)
    if hm and float(hm[0]['home8_kj']) > 0:
        m['home8_kj'] = float(hm[0]['home8_kj'])
    return m


def table(arm):
    d = load(f'{arm}/colsel.csv')
    out = {}
    for r in d:
        for c, base in d[r].items():
            m = measures(f'{arm}/c{c}/r{r}', base)
            if m:
                out.setdefault(r, {})[c] = m
    return d, out


def retest(arm):
    d, t = table(arm)
    keys = sorted({k for r in t for c in t[r] for k in t[r][c]})
    rt = {k: [] for k in keys}
    n = 0
    for r in sorted(d):
        if r - 1 not in d or r + 1 not in d or r not in t or r + 1 not in t:
            continue
        kept = [c for c in kept_twice(d, r) if c in t[r] and c in t[r + 1]]
        if len(kept) < 6:
            continue
        n += 1
        for k in keys:
            a = [t[r][c].get(k) for c in kept]
            b = [t[r + 1][c].get(k) for c in kept]
            if None not in a and None not in b:
                rt[k].append(spearman(a, b))
    print(f'\n=== {arm}: retest over {n} round pairs (kept-twice colonies), Spearman r -> r+1 [90% bootstrap]')
    rng = random.Random(1)
    for k in sorted(keys, key=lambda k: -(sum(x for x in rt[k] if x == x) / max(1, len(rt[k])))):
        print(f'   {k:18s} {ci(rt[k], rng)}')


def pair(a, b):
    _, ta = table(a)
    _, tb = table(b)
    keys = sorted({k for r in ta for c in ta[r] for k in ta[r][c]})
    w = {k: [] for k in keys}
    for r in sorted(set(ta) & set(tb)):
        cs = sorted(set(ta[r]) & set(tb[r]))
        if len(cs) < 6:
            continue
        for k in keys:
            x = [ta[r][c].get(k) for c in cs]
            y = [tb[r][c].get(k) for c in cs]
            if None not in x and None not in y:
                w[k].append(spearman(x, y))
    print(f'\n=== world effect: {a} vs {b} (same colony index and round = same world), Spearman [90% bootstrap]')
    rng = random.Random(1)
    for k in keys:
        print(f'   {k:18s} {ci(w[k], rng)}')


if __name__ == '__main__':
    a = sys.argv[1:]
    if a[:1] == ['--pair']:
        pair(a[1], a[2])
    elif a[:1] == ['--csv']:
        _, t = table(a[1])
        keys = sorted({k for r in t for c in t[r] for k in t[r][c]})
        w = csv.writer(sys.stdout)
        w.writerow(['round', 'colony'] + keys)
        for r in sorted(t):
            for c in sorted(t[r]):
                w.writerow([r, c] + [t[r][c].get(k, '') for k in keys])
    else:
        for arm in a:
            retest(arm)
