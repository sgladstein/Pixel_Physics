# The evolution lab, round thirty-two: the brief

**Handed over by round 31's coordinator, 2026-09-13.** Round 31's record is
[`evolution-lab-round-31-2026-09-13.md`](evolution-lab-round-31-2026-09-13.md).
**Read that record's *"nobody can reproduce the owner's bed"* section before
anything else here** — it is why this brief is ordered the way it is, and it
invalidates the premise of at least three open register sections.

**A `lab-coordinator` skill now exists** (`.claude/skills/lab-coordinator/SKILL.md`,
landed 2026-09-13 by another session). Read it for the spawn/poke/close
mechanics; this brief carries only what is specific to round 32.

## Task 1 — build a bed that looks like the owner's game

**Everything else in this brief is downstream of this, and most of the open
register is untrustworthy until it exists.**

Round 31 established, by measurement and then by the owner's own eye, that
**no bed this project can currently generate resembles the one he plays.**

| | ants |
|---|---|
| the owner's ordinary late game | **1000+ long ants** |
| `played_bed`, shipped settings, 200,000 frames | median **111**, max **408** |
| the largest single run in a 75-run sweep | **1,067**, once, `life_half_life: 80000` seed 8 |

His verdict on the most developed nest round 31 could render — `played_bed`
seed 3, 150,000 frames, 74 ants — was *"None of this reads as an ant hill
though it just looks like herbs growing in dirt."*

**The input you will have that round 31 did not: a real chronicle file.**
#374 gave the chronicle player actions, autosave, and nine load/cost columns
(wall clock, awake chunks, active sites, achieved against requested ticks,
`sim_debt`, speed multiple, display rate, draws skipped). The owner has agreed
to play a session to the point where it hurts and hand the file over. **Ask
the coordinator for it; do not post a card asking him** (see the standing
rulings below).

From that file, build a scenario that reaches his scale, and **prove it does**
before anything is measured on it. Then re-open, on that bed and not before:
§Z18 (floating debris), §Z13 (resting reads as stuck), and the stripped-ground
complaint in task 3.

## Task 2 — the late-game performance deep-dive

**The owner asked for this directly and it is the reason task 1 exists.** He
reports the game struggling with 1000+ long ants.

**Do not start it on a 400-ant bed.** With task 1's bed and his chronicle:

- read `sim_debt`, achieved-against-requested and `draws_skipped` out of his
  own session first — that says whether he is sim-bound or render-bound, and
  it is the fork everything else hangs on;
- **read `speed_multiple` beside every one of those** — the same achieved rate
  means opposite things at 1X and at the top of the ladder;
- the perf line's handed-forward list is the **~21% in the kernel and rayon**,
  then the moisture pass, then the pheromone `roundf` (which is *not*
  behaviour-free);
- per-phase timing belongs in `scale_probe phases=`, headless, **never in the
  live loop** — round 31 deliberately kept stopwatches out of `Lab::tick`.

**Model: Opus, not Fable.** See the rulings below — the difficulty here is
measurement discipline, not reasoning depth, and this repo's history of
timing numbers that were correct and about the wrong thing is long.

## Task 3 — land recovery behind a colony

The owner's playtest report, 2026-09-13: ants find a herb patch, feed, breed,
dig, **eat it clean**, and move on; the abandoned nest never regrows, though a
hand-planted herb there grows fine.

**Eating a patch clean and moving on is foraging, not a bug. The bug is that
the land does not recover behind them** — and that is a plant problem.
`Reports/plant-reseeding-2026-09-03.md` already measured four causes ahead of
dispersal, which is only a 1.4x effect:

- the germination gate opens on **two materials in the whole set** (three now
  — `spoil` declares `water_capacity: 1000`, checked);
- the grow lamps leave **32-column dead bands**;
- the colony is a **seed predator**, cutting the stand **2.8x**;
- and the largest single sink is seeds stuck **on the parent plant** — 183 of
  332 standing seeds, because a seed does not fall through branches while a
  windfall does, an inconsistency that was never designed.

**Treat these as one problem.** Fixing dispersal alone buys 1.4x against a
2.8x predation loss.

## Task 4 — §Z13, and why three idle animations all failed

Round 31 shipped `PIXEL_PHYSICS_IDLE_ANIM=head|antennae|shuffle`, all
default-off, and the owner's verdict was *"The creatures that I think look
stuck are stuck in all of them. Although this is a very short gif to have to
judge this on."* Lane E was left checking three explanations: the animation
never fired on the marked animals; it fired and 502 px of 163,840 is too faint
to read; or they are genuinely stuck and §Z13's "look problem, not a walk bug"
is wrong. **Read where that got to before building a fourth candidate.**

**And fix the card, not just the mechanism.** A resting ant's idle streaks run
33–83 stops of 900 frames. A short gif can show neither an animation cycling
nor an animal that has not moved in 50,000 frames.

## Standing rulings this round paid for

- **The review queue is for visual evaluations only.** Owner, 2026-09-13:
  *"General questions or requests should be sent to the coordinating agent to
  tell me."* A lane wanting a file, an answer or a decision routes it through
  the coordinator.
- **Fable 5.1 is `$10`/`$50` per MTok against Opus 5's `$5`/`$25`** — twice
  Opus, the most expensive tier, not the cheapest. Round 31's brief said the
  opposite; it was retracted in #372. `create_session` takes no effort
  parameter, so for a lane **the model is the whole dial**.
- **A conservation failure is a question about the ruler before it is a
  question about the engine.** Adding a material silently changes every census
  that enumerates materials by name — including ones inside tests.
- **The dangerous merge is the conflict-free one, and CI cannot see it.** Two
  PRs measured against different trunks are a combination nothing tested.
  Round 31 merged three lane branches into a scratch branch and ran the suite
  there; it found a red test no CI had reported.
