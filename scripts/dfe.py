#!/usr/bin/env python3
"""Distribution of fitness effects, from `selection_arena arm=mutantK` runs.

**The question this answers**, and it is the one the evolvability programme
exists for: are the mutations the engine actually makes things the world can
select on? Every arm in `Reports/plant-selection-teeth-2026-08-29.md` is a
handicap authored by hand, which shows the world *can* select and says nothing
about the variation mutation supplies.

**Why it is answerable when a per-mutation verdict is not.** That report's §3
measures the resolution floor: ~9.3 share-points of noise per seed, so a
single mutation's effect needs hundreds of worlds to pin down. But variance
decomposes:

    Var(observed across mutations) = Var(true effects) + Var(noise)

`Var(noise)` is measured directly by the identical-arms control, so the real
spread of fitness effects follows by subtraction even when no individual
mutation is resolvable. That is a statement about the *distribution*, which is
what "can selection act on this?" actually asks.

    Var(true) ~ 0  -> mutations are effectively neutral; selection has nothing
                      to sort, and the bottleneck is the genotype->phenotype
                      map rather than the environment.
    Var(true) > 0  -> there is heritable fitness variation and evolution can
                      proceed.

**Silent mutations are excluded, not counted as neutral.** A mutation that
changes the genome and not the plant sits at exactly the control's value and
would drag `Var(observed)` toward `Var(noise)` -- reading as "mutations are
neutral" when the truth is "this mutation never happened, phenotypically".
That is `plant-fate-operator-gate-2026-08-29.md`'s silent class, and pooling
it is the error that report was written to stop. They are reported as their
own count, which is itself a finding: the per-query fallback absorbs most
operators, so a large silent fraction is expected and is the thing the
fallback fork is about.

Usage:
    python3 scripts/dfe.py mutants.txt control.txt

Each file is `selection_arena` output; several runs may be concatenated.
"""
import re
import sys


def shares(path):
    """Every per-seed B-share (of cells) in a log, keyed by the arm it came from."""
    out = {}
    arm = None
    for line in open(path, encoding="utf-8"):
        m = re.match(r"selection_arena: species=\S+ arm=(\S+)", line)
        if m:
            arm = m.group(1)
            continue
        m = re.match(r"\s+\d+\s+[\d.]+%\s+([\d.]+)%", line)
        if m and arm:
            out.setdefault(arm, []).append(float(m.group(1)))
    return out


def var(xs):
    n = len(xs)
    if n < 2:
        return float("nan")
    m = sum(xs) / n
    return sum((x - m) ** 2 for x in xs) / (n - 1)


def main():
    if len(sys.argv) < 3:
        sys.exit(__doc__)
    mut, ctl = shares(sys.argv[1]), shares(sys.argv[2])
    ctl_all = [v for vs in ctl.values() for v in vs]
    if len(ctl_all) < 3:
        sys.exit("control file has under 3 seeds -- it sets the noise floor, so it cannot be skipped")
    v_noise = var(ctl_all)
    ctl_mean = sum(ctl_all) / len(ctl_all)

    # One number per mutation: its mean share over the seeds it was run on.
    per_mut, silent = {}, []
    for arm, vs in sorted(mut.items()):
        mean = sum(vs) / len(vs)
        # Silent iff every seed lands on the control's own value. Exact
        # equality, not a tolerance: a silent arm IS the control, bit for bit,
        # so anything that merely resembles it is a real (small) effect and
        # must stay in the distribution.
        if all(any(abs(v - c) < 1e-9 for c in ctl_all) for v in vs):
            silent.append(arm)
        else:
            per_mut[arm] = mean

    print(f"control: {len(ctl_all)} seeds, mean {ctl_mean:.2f}%, Var(noise) = {v_noise:.2f}")
    print(f"mutations drawn: {len(mut)}   silent (excluded): {len(silent)}   usable: {len(per_mut)}")
    if silent:
        print(f"  silent arms: {', '.join(silent[:12])}{' ...' if len(silent) > 12 else ''}")
        print("  (a silent arm changed the genome and not the plant -- the per-query fallback")
        print("   absorbing the operator. Counted, never pooled: pooling them would pull the")
        print("   spread toward the noise floor and read as 'mutations are neutral'.)")
    if len(per_mut) < 3:
        print("\nunder 3 usable mutations -- no distribution to speak of. Draw more arms.")
        return

    vals = sorted(per_mut.values())
    v_obs = var(vals)
    # Each mutation's mean is over `reps` seeds, so its own noise is reduced.
    reps = max(1, min(len(v) for v in mut.values()))
    v_true = v_obs - v_noise / reps
    n = len(vals)
    print(f"\nobserved spread across mutations: Var = {v_obs:.2f}  (sd {v_obs ** 0.5:.2f} points)")
    print(f"noise at {reps} seed(s) per mutation:      Var = {v_noise / reps:.2f}")
    print(f"  => Var(true fitness effects) = {v_true:+.2f}   sd = {abs(v_true) ** 0.5:.2f} points")
    print(f"\nmedian {vals[n // 2]:.1f}%   quartiles {vals[n // 4]:.1f}% .. {vals[3 * n // 4]:.1f}%"
          f"   range {vals[0]:.1f}% .. {vals[-1]:.1f}%")
    if v_true <= 0:
        print("\n  Var(true) <= 0: the spread across mutations is no wider than the harness's own")
        print("  noise, so NO heritable fitness variation is detectable. Read it as an upper")
        print("  bound, never as proof of neutrality -- with this many mutations and seeds the")
        print("  bound is loose, and it is quoted below rather than left implied.")
    else:
        print("\n  Var(true) > 0: mutations differ in fitness by more than noise accounts for.")
        print("  This is the precondition for selection to act on real variation.")
    # An upper bound is more honest than a null: say what would have been seen.
    print(f"  Detectable at this n: an effect sd above ~{(v_noise / reps / max(n - 1, 1)) ** 0.5 * 2:.1f} points.")


if __name__ == "__main__":
    main()
