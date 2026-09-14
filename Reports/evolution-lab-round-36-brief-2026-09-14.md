# Round 36 brief — the half of an ant that is not in the creature pass

*Written by round 35's coordinator, 2026-09-14. Read
[`evolution-lab-round-35-2026-09-14.md`](evolution-lab-round-35-2026-09-14.md)
for what just landed and
[`lanes/evolution-lab-coordinator.md`](lanes/evolution-lab-coordinator.md)
for what binds. **Do not re-read the whole archive.***

---

## The one thing that matters

**Performance at high creature counts is the owner's #1 and has been for four
rounds.** Round 33 answered one half — parallelising the creature pass is worth
3–4% of the frame and ships off. Round 34 demolished the premise the other half
rested on: **there is no knee**, and round 32's cheap sub-knee regime is an
artifact of bed age and plants paying the ants' bill, reproducible on demand.

**What round 34 handed forward is a much better target, and it is measured
rather than inferred:**

> **About half of what an ant costs is not in the creature pass at all.** The
> creature phase accounts for **45%** of the frame's growth from 0 to 428 ants.
> The other **55%** is the CA sweep over the **29.4 cells per ant per frame**
> that an ant's movement leaves dirty.
> ([`evolution-lab-knee-2026-09-14.md`](evolution-lab-knee-2026-09-14.md) §4.)

**So the question for this round is: can an ant dirty fewer cells, or can the
sweep do less with the ones it dirties?** Those are two different jobs and
probably two lanes.

- **Fewer dirty cells.** 29.4 per ant per frame is a *measurement*, not a
  budget — nobody has looked at where they come from. An ant is a few cells; 29
  is several times its own body. Find out what the multiplier is made of before
  proposing anything: the body's own moves, the cells it wakes by being
  adjacent, the chunk it keeps awake, the pheromone it writes. **A census
  first. The fix, if any, comes after.**
- **A cheaper sweep over them.** This is where the perf line's handed-forward
  list already points
  ([`evolution-lab-frame-cost-2026-09-01.md`](evolution-lab-frame-cost-2026-09-01.md)
  §18.5): the ~21% in the kernel and rayon, the moisture pass, the pheromone
  `roundf` — which is **not** behaviour-free and needs a seed sweep, not an
  A/B.

**The instrument exists.** `examples/antcost.rs` was built for round 34 and it
already separates the phases. **Pin bed age and the plant bill** or it measures
those instead of the ant — that is exactly how round 32 produced a knee that
was not there.

**Two traps this repo has paid for, both live here.** A cost that *vanishes*
rather than shrinking usually means the work vanished, not that it got cheap —
find the quantity that says the subsystem still does its job. And **removing
work is not the same as removing cost**: a gate that skipped 91% of the field's
momentum passes made the frame *slower* in 7 of 8 paired runs, because the
arithmetic went away and the memory traffic only moved. **The phase a change is
made cheaper against is the whole frame**, measured paired and alternating.

## Task 2 — the economy rivalry now sits on, which is the half that was left

*Rewritten 2026-09-14: the switch shipped after this brief was first written.
`assets/species/ant.ron` authors `scent_spread: 2.0` (#423), chosen on an order
statistic over 12 seeds — cross-colony kills in **0 of 12** at spread 0, **9 of
12** at 1, **11 of 12** at 2. The task below is what lane C correctly declined
to do inside a one-line asset change.*

**A stranger is food now, and the constants that price the colony were all
calibrated on a bed where no ant was.** The commit names three: **the birth
bar, `colony_ants`, and the starvation balance.** *A correct mechanism at
inherited constants is a regression* — that is this repo's own rule and it is
pointing straight at these.

What the switch already moved, paired off(0) against shipped(2) over 12 seeds:
**deaths +99 median**, up on 11 of 12 and down on none; **starvation share −4.3
points**, down on 10 of 12, because killing displaces starving; **population
and births both unmoved, medians exactly 0.** So the bed absorbs it today —
which is the reason this is a re-derivation rather than an emergency, and also
the reason it is easy to leave undone.

The questions worth a lane:

- **Is a colony that can eat its neighbours meant to need the same birth bar?**
  Meat arriving from a rival is income the bar was never set against.
- **Starvation fell because killing replaced it.** Does that make the
  starvation balance right, or does it mean it was never doing the work?
- **`colony_ants` sets how many founders stand together.** At a live dial that
  is also how big a war party is.

**Gate any change on an order statistic over seeds, and keep an `off` arm** —
shipping on is not hardcoding, and the owner's standing direction is *"give me
the tools, data, access to the parameters that need to be tweaked and I do that
testing myself in the game."* **Six seeds is not a sweep.**

**Two traps this specific work has already sprung once, both recorded in
[`evolution-lab-round-35-2026-09-14.md`](evolution-lab-round-35-2026-09-14.md)
§8.** The `spread=` override *added* its offset to the scent an animal already
carried, so once a live default existed every arm measured `authored +
requested` and **`spread=0` was not an off arm**; it was caught by running the
authored value and the runtime override at one seed and requiring them
byte-identical, and they disagreed on one column **while every outcome column
matched**. And the founding draw **is not stable across engine changes** — one
seed's gap moved 0.907 → 2.170 at an unchanged setting, because the offsets
hash the colony *label* and which labels get claimed moved upstream. **Tune on
the threshold argument, never on a table of particular seeds.**

**And what ships is predation before it is a war.** A stranger is food to the
ordinary mouth; the `Attack` verb is a second, separate thing. Say which one a
number is about, every time.

## Task 3 — §Z23, which is a contained first job

**`nearest_foe` counts a plant as a foe**, so a fed colony vandalises its own
larder — and #417 *sharpened* it rather than closing it: a plant fails
`is_animal`, so `commits` is unconditionally true and **a plant is the one
target in the world struck with no assessment at all.**

**The repair is already designed and the obvious one is wrong.** Do not test
whether the target's species has a `creature` def — an animal cornered by
something it cannot digest must still be able to hit it. Gate **`cry_alarm`'s
two feeding call sites** on the victim being an animal, which leaves the target
rule intact. `Reports/open-bugs-handoff.md` §Z23 carries the reproduction.

## Standing facts that will cost you time otherwise

- **There is no delivery signal for a poke.** `fire_trigger` success,
  `last_run`, and the session's `updated_at` are all uninformative. **Check the
  branch head.** Put anything load-bearing in the repo as well as in the poke.
- **`bugindex.py --branches` before filing a bug, never `--check`** — two
  branches filed the same letter on one day in round 35 and both were green.
- **A new `dead-ends.md` entry needs a `screened.tsv` verdict**, or `docscheck`
  goes red and nothing else notices.
- **`cargo test --release` exceeds the 600 s Bash cap** — split `--lib` then
  `--test worldgen --test determinism`. **`cargo build --release` does not
  rebuild examples**; use `--examples` with `set -o pipefail`.
- **Timings in this container are untrustworthy across runs.** Pin
  `RAYON_NUM_THREADS`, or compare arms **inside one run** with the order
  swapped each round. Quote the ratio; the milliseconds do not transfer.
- **Post the picture, do not describe it.** `scripts/review.py`, and
  `review.py get <id>` is the only authoritative read.
