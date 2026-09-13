# Lane C — round 32

Two jobs: the spoil teleport (§Z19, **shipped**) and §Z13's *resting or
stuck* (**already answered on `main`; no code written**).

## 1. §Z19 — the spoil teleport

Full account and every number: `Reports/open-bugs-handoff.md` §Z19. What
belongs here rather than there:

**PR #221 is finished with.** Its instrument and its counters are on the
trunk; its *rule* is on the trunk in a strictly stronger form; its remaining
half — creatures walking through living root — was measured by that branch,
**left switched off by its own author**, and is not ported. That branch is
731 commits behind `main` and nothing is served by reviving it. **Close
#221.**

**The premise this lane was given was partly stale, and the stale part was
the interesting part.** The brief said the counters were unlanded; they were
landed, with #221's reasoning quoted, by round 31. What was *not* landed was
the instrument — and an instrument that will not build against `main` is
precisely what kept the work invisible. The lesson is not "check the brief":
it is that **a branch's counters can land while its harness does not, and the
counters then have nothing to print them.**

**The defect and the picture came apart, which no plan predicted.** #221 and
§Z18 both argued from the *standing* consequence — pellets left 50–99 rows up
a tree. §379's `needs_footing` removed that: measured here, tallest standing
pellet **+2/+4/+3/+2** with a tree against **+3/+5/+3/+4** without, i.e. gone.
The **travel** was untouched and still ran to 116 rows. Had this lane trusted
the standing census it inherited, it would have reported the bug fixed.

**#221's rule fixes the median and not the tail.** Its bound asks only *could
this animal have cut through what is in the way*, so a column of clear sky was
unbounded by it: median `lift_max` 78.5 → 9, and **max 116 → 102**. The tail
is the defect. Adding *could it have climbed* — open air passes only beside a
wall — takes max to **22**, p90 to **7**, for the same dig cost (3,222 digs
against 3,221). Both arms are shipped behind one switch so the next lane can
re-take them without a rebuild.

## 2. §Z13 — resting, not stuck. Answered before this round began.

**No code, and that is the result.** The brief asked for the owner's three
markers to be mapped to world cells and the animals there checked from the
inside. **Round 29 did exactly that and it is on `main`** —
`Reports/open-bugs-handoff.md` §Z13 and
`Reports/lanes/evolution-lab-longant-pile.md` §4 carry the crop
(`zoom=4`, `crop=160,120,224,56`), the world cells **(363,155)**, **(302,149)**,
**(244,154)**, and the per-animal table: full-length bodies, `moves` +0/+0/+1,
**`moves_blocked` +0 at all three**, `traffic_deferred` 0, `crossing`/`flight`/
`senescent` clear.

**They are resting. §Z13's *"look problem, not a walk bug"* stands.**

What this lane added, being the one thing §Z13's argument rested on without a
control: that table reads the **per-animal** `life.moves_blocked`, while
§Z13's reasoning is stated over the code paths that touch the **world**
counter. Those are two different fields and a reader is entitled to ask
whether they pair. **They do** — both sites in `step_chain` that increment
`world.creature_stats.moves_blocked` (`src/sim/creature.rs:7679` and `:7691`)
increment `state.life.moves_blocked` immediately after, so the per-animal
counter is not blind on either path and `+0` means *did not ask*, not *asked
and was not counted*.

**Nothing further should be built here without the owner's eye.** Round 31
lane E's three idle animations are already on his queue as one card; a fourth
is not wanted, and the brief said so.

## Standing notes for whoever is next

- **`--branches`, never `--check`.** #221's section was filed as §Z4, a letter
  already closed on `main`, so it is not in the register anyone reads. This
  round's §Z19 was taken from `bugindex.py --branches` over 69 refs.
- **The PR list is not the landed list.** `branchcheck --prs` showed #221's
  branch as *having a PR*, which reads as owned; it had been stalled ten days.
- **Pin `RAYON_NUM_THREADS` for any counter you will compare**, and change the
  rule with an env switch inside **one** binary. This lane produced and then
  discarded a whole 12-seed sweep because the example was rebuilt while it ran
  — the stale-binary trap `CLAUDE.md` records four times, arrived at a fifth
  way.
- **Whole-run arms on this bed are different worlds after the first
  divergence.** Read an order statistic over twelve seeds; per-seed pairs will
  show a byte-identical seed and a seed that moves backwards, and neither is a
  reading about the rule.

*Freshness: 2026-09-13.*
