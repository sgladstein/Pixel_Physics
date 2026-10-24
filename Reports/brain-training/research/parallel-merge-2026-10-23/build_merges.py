"""SCRATCH (parallel-merge thread, 2026-10-23): how Test 2 and Test 3's merged founders were made. Run from the
scratch folder holding gen.py, groups*.txt (xgroups.rs output: mind::crossover_group per slot) and the pools.
  Test 2: pools = colsel20 cs20light colonies c3 and c27, round 47 (project files brood/carry/), sorted by score.
  Test 3: pools = round-7 finals of rounds.sh islands J/F/D (e3-islands.jobs), survivors only, paired by garden seed.
Arms: MIX = 15 of each pool; XOVER = 30 children, each mind::crossover_group taken whole from one parent (train::cross
semantics); AVG = slot-wise mean of a pair; TASKSUM = hcs + (f - hcs) + (d - hcs), clamped to +-40."""
import random
def crossover(a, b, grp, rng):
    take = {}
    return [(b[s] if take.setdefault(grp[s], rng.random() < 0.5) else a[s]) for s in range(len(a))]
def average(a, b):
    return [(p + q) / 2 for p, q in zip(a, b)]
def task_sum(a, b, base):
    return [max(-40, min(40, p + q - z)) for p, q, z in zip(a, b, base)]
def lesson_union(base, lessons):
    """Test 1: the base with every lesson's changed slots written in. Refuses overlapping lessons."""
    out, seen = list(base), set()
    for g in lessons:
        d = {i for i in range(len(base)) if g[i] != base[i]}
        assert not (d & seen), f'lessons overlap on {len(d & seen)} slots'
        seen |= d
        for i in d:
            out[i] = g[i]
    return out
