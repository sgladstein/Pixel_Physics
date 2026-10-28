"""SCRATCH (copy test, 2026-10-27): run arms in the game's lab boxes and collect `labsurvive`'s RESULT lines.
`exam.py`'s lab half, with the seeds chosen (the copy was labelled on seeds 1-6, so it is judged on others) and one
CSV for all arms.

usage: boxes.py OUT.csv ARM=FOUNDERS[|ENV] ... [boxes=a,b] [seeds=11-18] [after=60000] [P=4]
  FOUNDERS  as labsurvive takes it: old, std, or a genome list (path[@tag]:count,...)
  ENV       extra env for that arm, k=v;k=v; a third |-part adds labsurvive args, e.g. old||tax=0
Rows already in OUT.csv are kept and not rerun.
"""
import csv, os, subprocess, sys
from concurrent.futures import ThreadPoolExecutor

OUT = sys.argv[1]
opts = dict(a.split('=', 1) for a in sys.argv[2:] if a.split('=', 1)[0] in ('boxes', 'seeds', 'after', 'P'))
arms = [a.split('=', 1) for a in sys.argv[2:] if a.split('=', 1)[0] not in opts]
boxes = opts.get('boxes', 'two_larders,scattered_larder,feast_and_famine').split(',')
lo, hi = map(int, opts.get('seeds', '11-18').split('-'))
after = opts.get('after', '60000')
done = set()
rows = []
if os.path.exists(OUT):
    rows = list(csv.DictReader(open(OUT)))
    done = {(r['arm'], r['box'], r['seed']) for r in rows}


def run(job):
    arm, spec, box, sd = job
    founders, _, rest = spec.partition('|')
    env, _, args = rest.partition('|')
    extra = dict(kv.split('=', 1) for kv in env.split(';') if kv)
    out = subprocess.run(['target/release/examples/labsurvive', f'founders={founders}', f'scenario={box}', f'seed={sd}',
                          f'after={after}'] + args.split(), capture_output=True, text=True,
                         env={**os.environ, **extra, 'RAYON_NUM_THREADS': '1',
                              'PIXEL_PHYSICS_LAB_SCENARIOS': 'assets/lab_scenarios'}).stdout
    line = next((l for l in out.splitlines() if l.startswith('RESULT')), '')
    kv = dict(x.split('=', 1) for x in line.split()[1:])
    print(f'{arm} {box} {sd}: {line}', flush=True)
    return dict(arm=arm, box=box, seed=str(sd), **kv)


jobs = [(a, s, b, sd) for b in boxes for sd in range(lo, hi + 1) for a, s in arms if (a, b, str(sd)) not in done]
with ThreadPoolExecutor(int(opts.get('P', '4'))) as ex:
    for r in ex.map(run, jobs):
        rows.append(r)
        keys = list(dict.fromkeys(k for x in rows for k in x))
        with open(OUT, 'w') as f:
            w = csv.DictWriter(f, keys)
            w.writeheader()
            w.writerows(rows)
