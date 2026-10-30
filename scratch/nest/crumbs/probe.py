"""SCRATCH (chambers crumbs lane, step 2): PickUp's best option score and job, by where the ant is and how fed, from a
gzipped PP_MIND_LOG (scratch/nest/trace.sh). Classes: field (at_nest 0), home-surface (at_nest 1, depth sense 0),
home-underground fed / hungry (depth > 0; hungry = energy sense < 1/3, the eat seed's bar). Prints the median and
quartiles of best[pickup], the PickUp job, and how often PickUp was chosen when offered.
usage: probe.py MIND.gz"""
import gzip, sys, collections
best = collections.defaultdict(list); job = collections.defaultdict(list); chose = collections.Counter(); n = collections.Counter()
with gzip.open(sys.argv[1], 'rt') as f:
    for l in f:
        if l[0] != 'M': continue
        p = l.split(',', 14)
        b = p[13].split()
        if len(b) < 5 or b[4].lower() in ('nan', '-inf', 'inf'): continue
        c = [float(x) for x in p[10].split()]
        e, at_nest, depth = c[0], c[3], c[18]
        k = 'field' if at_nest < 0.5 else ('home-surface' if depth <= 0 else ('home-under hungry' if e < 1 / 3 else 'home-under fed'))
        best[k].append(float(b[4])); j = p[12].split()[4]
        if j.lower() != 'nan': job[k].append(float(j))
        n[k] += 1; chose[k] += int(p[6]) == 4
q = lambda xs, f: sorted(xs)[int(f * (len(xs) - 1))] if xs else float('nan')
for k in sorted(n):
    print(f'{k:18} n {n[k]:7}  best q25/50/75 {q(best[k], .25):+.3f} {q(best[k], .5):+.3f} {q(best[k], .75):+.3f}  '
          f'job q50 {q(job[k], .5):+.3f}  picked {100 * chose[k] / n[k]:.1f}%')
