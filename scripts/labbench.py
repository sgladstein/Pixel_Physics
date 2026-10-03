#!/usr/bin/env python3
"""The lab bench: did this change also hold in the game?

    python3 scripts/labbench.py BASE NEW                    # 12 seeds of the played bed, 120k frames
    python3 scripts/labbench.py origin/main .               # '.' = this working tree, uncommitted edits included
    python3 scripts/labbench.py main HEAD --seeds 1-24 --jobs 3 --frames 60000
    python3 scripts/labbench.py main HEAD --env PIXEL_PHYSICS_BROOD=off          # a knob on BOTH arms
    python3 scripts/labbench.py main main --new-env PIXEL_PHYSICS_BROOD=off      # a knob on ONE arm, one binary
    python3 scripts/labbench.py main HEAD --example digbox \\
        --args "hungry gap=90 food=400 refill=400 w=260 soil=60 frames={frames} seed={seed}"
    python3 scripts/labbench.py --selftest

**Where it sits** (owner, 2026-10-03): the simple beds -- the food box
(`digbox`), `trailfollow`, `labnest` -- stay where problems are found and
solved. This is the *last* check, that a fix which works there also holds in
the lab box the owner plays. It is not a replacement for them, and a lab
result is not a diagnosis: the lab has too many moving parts to say *why*.

What it does, in order, so nothing in the procedure is left to hand:

1. Resolves BASE and NEW to commits and builds each one's example binary in
   its own `git worktree` (`.` builds this working tree as it stands, and
   records `git status --short` beside it so a dirty arm says what it held).
   Each binary is **copied into the run directory** before anything runs, so
   a rebuild mid-batch cannot change what is being measured -- the handoff's
   standing pitfall.
2. Runs every seed on both arms, `--jobs` at a time (default 3 on a 4-core
   box, leaving one for the rest of the machine), each with
   `RAYON_NUM_THREADS=1`, **interleaved base/new** so a machine that slows
   halfway through slows both arms alike. Each run's working directory is its
   own arm's source tree, so `assets/lab_scenarios/*.ron` (read at run time,
   not compiled in) are the arm's own.
3. Names logs `base-<seed>.log` / `new-<seed>.log` -- exactly what
   `labpair.py` keys on -- and writes `manifest.txt` (commits, command line,
   env, per-run wall time).
4. For `labforage` beds, runs `scripts/labpair.py` and saves its table to
   `labpair.txt`. Other beds have no paired comparer yet; the logs are named
   the same way and the script says so.

**Resumable**: a log that already has its SUMMARY line is not re-run, so a
batch killed halfway is finished by running the same command again with the
same `--out`. A log without one (killed mid-run) is re-run from scratch.

Every number from this carries its commits: they are in `manifest.txt`, and
the table is meaningless without them. Paired by seed, medians, "higher on N
of M" and a sign test -- never a mean of a single run against a remembered
number.
"""
import argparse
import concurrent.futures
import filecmp
import os
import shlex
import shutil
import subprocess
import sys
import tempfile
import time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
LAB_ARGS = "scenario=played_bed frames={frames} seed={seed}"


def sh(cmd, cwd=ROOT, check=True, **kw):
    return subprocess.run(cmd, cwd=cwd, check=check, text=True, capture_output=True, **kw)


def parse_seeds(text):
    out = []
    for part in text.split(','):
        if '-' in part:
            a, b = part.split('-')
            out.extend(range(int(a), int(b) + 1))
        elif part:
            out.append(int(part))
    return sorted(set(out))


def finished(path):
    if not os.path.exists(path):
        return False
    with open(path, 'rb') as fh:
        return any(line.startswith(b'SUMMARY ') for line in fh)


def prepare_arm(arm, rev, example, out, target_dir, log):
    """Build `example` at `rev` and return (binary path, source dir, description)."""
    if rev == '.':
        src = ROOT
        head = sh(['git', 'rev-parse', 'HEAD']).stdout.strip()
        dirty = sh(['git', 'status', '--short', '--untracked-files=no']).stdout.strip()
        desc = f"{head} + working tree" + (f" (dirty:\n{dirty})" if dirty else " (clean)")
    else:
        sha = sh(['git', 'rev-parse', '--verify', f'{rev}^{{commit}}']).stdout.strip()
        src = os.path.join(out, f'src-{arm}')
        if os.path.isdir(src):
            got = sh(['git', 'rev-parse', 'HEAD'], cwd=src).stdout.strip()
            if got != sha:
                sys.exit(f"labbench: {src} holds {got}, not {sha} -- use a fresh --out")
        else:
            sh(['git', 'worktree', 'add', '--detach', src, sha])
        desc = f"{sha} ({rev})"
    log(f"{arm}: {desc}")
    log(f"{arm}: building --example {example} ...")
    # **One target dir per arm, never shared.** Shared, the second arm's build
    # was a no-op that pinned the FIRST arm's binary under the second's name:
    # cargo judges freshness by mtime, and a worktree checked out a moment ago
    # is newer than an edit made an hour ago. Caught on this script's own
    # first run (two byte-identical binaries for origin/main vs a dirty tree).
    env = dict(os.environ, CARGO_TARGET_DIR=os.path.join(target_dir, arm))
    t = time.time()
    r = subprocess.run(['cargo', 'build', '--release', '--locked', '--example', example],
                       cwd=src, env=env, text=True, capture_output=True)
    if r.returncode != 0:
        sys.stderr.write(r.stderr[-4000:])
        sys.exit(f"labbench: {arm} build failed at {desc}")
    built = os.path.join(target_dir, arm, 'release', 'examples', example)
    pinned = os.path.join(out, 'bin', f'{arm}-{example}')
    os.makedirs(os.path.dirname(pinned), exist_ok=True)
    shutil.copy2(built, pinned)
    log(f"{arm}: built in {time.time() - t:.0f}s, pinned at {pinned}")
    return pinned, src, desc


def run_one(job):
    arm, seed, binary, src, argv, env, path = job
    t = time.time()
    with open(path, 'w') as fh:
        r = subprocess.run([binary] + argv, cwd=src, env=env, stdout=fh, stderr=subprocess.STDOUT)
    return arm, seed, r.returncode, time.time() - t, path


def main(a):
    seeds = parse_seeds(a.seeds)
    out = os.path.abspath(a.out or os.path.join(ROOT, 'target', 'labbench', time.strftime('%Y%m%d-%H%M%S')))
    os.makedirs(out, exist_ok=True)
    target_dir = os.path.abspath(a.target_dir or os.path.join(ROOT, 'target', 'labbench-build'))
    manifest = open(os.path.join(out, 'manifest.txt'), 'a')

    def log(msg):
        print(msg, flush=True)
        manifest.write(msg + '\n')
        manifest.flush()

    template = a.args if a.args is not None else (LAB_ARGS if a.example == 'labforage' else None)
    if template is None:
        sys.exit("labbench: --args is required for a bed other than labforage ({seed} and {frames} are filled in)")
    if '{seed}' not in template:
        sys.exit("labbench: --args must contain {seed}, or every run is the same world")
    log(f"labbench {time.strftime('%Y-%m-%d %H:%M:%S')}: {' '.join(shlex.quote(x) for x in sys.argv)}")
    log(f"  out {out}")
    log(f"  bed {a.example} {template}  frames={a.frames} seeds={seeds} jobs={a.jobs}")

    def env_of(pairs):
        d = {}
        for p in pairs:
            k, _, v = p.partition('=')
            d[k] = v
        return d
    both = env_of(a.env)
    arms = {'base': (a.base, env_of(a.base_env)), 'new': (a.new, env_of(a.new_env))}
    built = {}
    for arm, (rev, extra) in arms.items():
        # Same commit on both arms (an env-only A/B): build once, pin twice.
        twin = next((b for b, r in built.items() if r[3] == rev and rev != '.'), None)
        if twin:
            pinned = os.path.join(out, 'bin', f'{arm}-{a.example}')
            shutil.copy2(built[twin][0], pinned)
            built[arm] = (pinned, built[twin][1], built[twin][2], rev)
            log(f"{arm}: same commit as {twin}, same binary")
        else:
            built[arm] = prepare_arm(arm, rev, a.example, out, target_dir, log) + (rev,)
        env = {**both, **extra}
        log(f"{arm}: env {' '.join(f'{k}={v}' for k, v in env.items()) or '(none beyond RAYON_NUM_THREADS=1)'}")
        with open(os.path.join(out, f'{arm}.commit'), 'w') as fh:
            fh.write(built[arm][2] + '\n')
    same_bin = filecmp.cmp(built['base'][0], built['new'][0], shallow=False)
    if same_bin and a.base != a.new:
        log("  WARNING: the two arms' binaries are byte-identical although their commits differ -- either the change "
            "does not reach this bed (docs only?) or a build was stale. The table below cannot show a difference.")
    elif same_bin:
        log("  arms share one binary: an env-only A/B")

    jobs = []
    for seed in seeds:
        for arm in ('base', 'new'):
            path = os.path.join(out, f'{arm}-{seed}.log')
            if finished(path):
                continue
            binary, src, _, _ = built[arm]
            argv = shlex.split(template.format(seed=seed, frames=a.frames))
            env = dict(os.environ, RAYON_NUM_THREADS='1', **both, **arms[arm][1])
            jobs.append((arm, seed, binary, src, argv, env, path))
    log(f"  {len(jobs)} run(s) to do, {2 * len(seeds) - len(jobs)} already finished")
    failed = []
    t0 = time.time()
    with concurrent.futures.ThreadPoolExecutor(max_workers=a.jobs) as pool:
        for i, (arm, seed, rc, secs, path) in enumerate(pool.map(run_one, jobs), 1):
            ok = rc == 0 and finished(path)
            log(f"  [{i}/{len(jobs)}] {arm} seed {seed}: {'ok' if ok else f'FAILED (exit {rc})'} in {secs:.0f}s")
            if not ok:
                failed.append((arm, seed))
    log(f"  runs done in {time.time() - t0:.0f}s" + (f"; FAILED {failed}" if failed else ""))

    for arm, (_, src, _, rev) in built.items():
        if rev != '.' and not a.keep_src and os.path.isdir(src):
            sh(['git', 'worktree', 'remove', '--force', src], check=False)

    if a.example != 'labforage':
        log(f"  no paired comparer for {a.example} yet; logs are {out}/base-<seed>.log and new-<seed>.log")
        return 1 if failed else 0
    r = sh([sys.executable, os.path.join(ROOT, 'scripts', 'labpair.py'), out, 'base', 'new'], check=False)
    table = r.stdout + r.stderr
    head = f"base {built['base'][2]}\nnew  {built['new'][2]}\n\n"
    with open(os.path.join(out, 'labpair.txt'), 'w') as fh:
        fh.write(head + table)
    print('\n' + head + table)
    return 1 if failed or r.returncode else 0


def selftest():
    """The parts that decide WHAT runs, without building anything."""
    bad = []
    if parse_seeds('1-3,7,2') != [1, 2, 3, 7]:
        bad.append(f"seed parse: {parse_seeds('1-3,7,2')}")
    d = tempfile.mkdtemp()
    p = os.path.join(d, 'x.log')
    with open(p, 'w') as fh:
        fh.write("labforage: frames=9\nSUMMARY p_move_hist n=1\n")
    if not finished(p):
        bad.append("a log with a SUMMARY line must count as finished")
    with open(p, 'w') as fh:
        fh.write("labforage: frames=9\n  900  5  3\n")
    if finished(p):
        bad.append("a log cut off before SUMMARY must be re-run")
    if '{seed}' in LAB_ARGS.format(seed=4, frames=9) or 'seed=4' not in LAB_ARGS.format(seed=4, frames=9):
        bad.append("the default lab args must carry the seed")
    for m in bad:
        print("SELFTEST FAIL:", m)
    print("labbench selftest:", "FAILED" if bad else "all checks passed")
    return 1 if bad else 0


if __name__ == '__main__':
    if sys.argv[1:] == ['--selftest']:
        sys.exit(selftest())
    ap = argparse.ArgumentParser(description=__doc__.split('\n\n')[0], formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('base', help="commit, branch or '.' (this working tree)")
    ap.add_argument('new', help="commit, branch or '.' (this working tree)")
    ap.add_argument('--seeds', default='1-12', help="e.g. 1-12 or 1,4,9 (default 1-12)")
    ap.add_argument('--frames', type=int, default=120000)
    ap.add_argument('--jobs', type=int, default=3, help="runs at once (default 3 on a 4-core box)")
    ap.add_argument('--example', default='labforage', help="which examples/ bed (default labforage)")
    ap.add_argument('--args', default=None, help="the bed's arguments; {seed} and {frames} are filled in")
    ap.add_argument('--env', action='append', default=[], help="K=V on both arms (repeatable)")
    ap.add_argument('--base-env', action='append', default=[], help="K=V on the base arm only")
    ap.add_argument('--new-env', action='append', default=[], help="K=V on the new arm only")
    ap.add_argument('--out', default=None, help="run directory (default target/labbench/<time>); reuse it to resume")
    ap.add_argument('--target-dir', default=None, help="parent of the per-arm cargo target dirs (default target/labbench-build)")
    ap.add_argument('--keep-src', action='store_true', help="keep the arms' worktrees after the runs")
    sys.exit(main(ap.parse_args()))
