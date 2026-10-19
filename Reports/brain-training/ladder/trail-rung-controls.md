# Trail rung for the stage 2 brain: floors, clones, and one score change

Ladder research lane, 2026-10-17, first version ~14:30, floors measured ~15:05. For the brain lane, whose first stage 2 rung is trail following from a blank brain (its asks of 14:01-14:08; it named this note `trail-rung-stage2.md`). "Measured" means seen in a log here; "inferred" means reasoned, not run. The stage 2 brain does not exist yet, so the random walkers below are built on today's brain as stand-ins for a blank stage 2 brain picking at random.

**Corrected 2026-10-17 ~16:45, after the independent review (`review-stage2-docs.md`, findings 2-4, 22, 25).** Every number in the tables reproduced exactly on main `8e628480`, and the signed score and `right` share are now in PR 713. Three things changed:
- **The bar is "reach the follower", not "beat it".** PR 713's own stage 2 follower scores 0.860 as 6 clones on the hold-out, every ant the right way. Beating it by 12% needs about 0.96, near the ceiling for six clones in one lane; beating today's follower (0.26) is passed untrained. See section 3.
- **The rest trap.** On the signed worse side, rest scores 0.000 and every blind mover scores below it (-0.019 to -0.348 here; PR 713's blank -0.025, persist -0.016). Where that is the fitness directly (`challenge=trail`), moving less is the first hill from blank. Fold each pair by its mean and keep the worse side as a gate, and print the share of brains whose most-picked verb is Rest.
- **The trail's bare end is a cue of its own.** An ant that steps only where there is scent ahead, with no slope weight, scores 0.311 alone and 0.171 as 6 (the full follower: 0.443 and 0.260). Decide whether turning back at the end of the scent counts as passing; if not, add an edge-only control.

## Answer

- **Rest-only floor: 0.000**, alone and as 6. Measured: today's zero genome never moves (`P(move)` is the `Move` output clamped at 0), and it scores 0 on every layout.
- **Random walkers do not score 0 on today's score, and the worse-side fold does not save it.** On the worse side of each mirrored pair, measured on 8 hold-out layouts at 6,000 frames:
  - blind random walkers score 0.005-0.014 alone;
  - in a group of 6 they score 0.024-0.028 when they change direction often, and **0.171 when they keep their heading**;
  - a blind straight walker that never turns scores **0.263 alone and 0.197 as 6**, because the bed's far wall turns it round and sends it back along the trail.

  The hand-built gradient follower scores 0.443 alone and 0.261 as 6, so a blind walker gets up to three quarters of it.
- **Change the score to signed progress** (`along` without the clamp at 0, so time spent the wrong way counts against). Measured on the same runs:
  - every blind walker falls to a mean of 0.000 ± 0.004, with a negative worse side (corrected: that puts every blind mover below rest, which scores exactly 0; see the rest trap at the top);
  - the hand-built follower keeps exactly its score (0.570 and 0.570 alone; 0.319 and 0.318 as 6).

  It is one line in `Trail::run_ants` (the `clamp(0.0, 1.0)` in `progress` becomes `clamp(-1.0, 1.0)`).
- **Clones, not mixed.** Train and test as 6 clones, and print the lone-ant score beside it. Today's brain from zero trained among strangers reached 0.12-0.18 when tested as clones (corrected from 0.13-0.15 to match `cheats-seen.md` C7). Trained as clones from zero, it reached 0.35 by generation 4 and 0.82 by generation 40 (cheat C7). Mixed selection rewards an ant scored among strangers, then the champion is judged among copies of itself, which jam a one-lane floor.

## 1. The floors, measured

Probe: `probes/trail/laddertrailnull.rs`, on main `20948e67`, with main's `Trail` arena (stone floor, 256 wide, static channel-B trail, the rig's trail bans). 8 hold-out layouts (seeds 50000-50003, mirrored), 6,000 frames. Logs: `probes/trail/floors/`.

- **The random walkers** (`rw:p`) are blind run-and-tumble walkers built on the zero genome:
  - a step chance `p` each tick;
  - a new random heading on every tick it does not step;
  - footing strongly preferred;
  - no bonus for going straight.

  So `p` sets how long it keeps a heading: about 0.6 of steps continue the last direction at `p` = 0.2, and about 0.9 at 0.8.
- **The one-way walker** (`rig-2026-10-17b/one-way-walker.txt`) always steps and never tumbles. It turns only when blocked.
- **The hand-built follower** (`rig-2026-10-17b/hand-gradient.txt`) moves when the scent ahead is stronger.

| Brain | Ants | `along` mean | `along` worse side | Signed mean | Signed worse side | Cells walked per ant |
|---|---|---|---|---|---|---|
| Rest-only (zero genome) | 1 | 0.000 | 0.000 | 0.000 | 0.000 | 0 |
| Rest-only (zero genome) | 6 | 0.000 | 0.000 | 0.000 | 0.000 | 0 |
| Random walker, p 0.2 | 1 | 0.040 | 0.006 | 0.000 | -0.068 | 184 |
| Random walker, p 0.2 | 6 | 0.034 | 0.024 | 0.000 | -0.019 | 150 |
| Random walker, p 0.5 | 1 | 0.091 | 0.005 | 0.000 | -0.171 | 509 |
| Random walker, p 0.5 | 6 | 0.073 | 0.028 | -0.001 | -0.092 | 401 |
| Random walker, p 0.8 | 1 | 0.188 | 0.014 | 0.000 | -0.348 | 806 |
| Random walker, p 0.8 | 6 | 0.210 | **0.171** | -0.004 | -0.080 | 709 |
| One-way walker | 1 | 0.292 | **0.263** | 0.000 | -0.061 | 906 |
| One-way walker | 6 | 0.288 | **0.197** | -0.002 | -0.186 | 623 |
| Hand-built follower | 1 | 0.570 | 0.443 | 0.570 | 0.443 | 837 |
| Hand-built follower | 6 | 0.319 | 0.261 | 0.318 | 0.260 | 219 |

At 12,000 frames the p 0.5 walker scores 0.007 alone and 0.054 as 6 on the worse side: a longer run gives a random walker more, not less (E3).

**Why the worse side does not catch them:**
- **Groups:** with 6 ants, some go each way on every layout. The p 0.8 walker sent 1-5 of 6 the right way.
- **Walls:** the bed's walls turn a straight walker round. A lone one-way walker ended on the trail on one layout of every mirrored pair, because whichever way it starts, it is walking back along the trail within about 850 frames.

This corrects `rungs.md`'s line that the one-way walker scores "about 0" on the worse side. That was inferred, and it is wrong: 0.197-0.263.

**The signed score's own check.** On the follower, which never goes the wrong way, the signed score matches the arena's `along` layout by layout (0.763, 0.490, 0.666, ...). So the start, direction and far end recovered by the probe are the arena's.

## 2. What the arena gives a brain to read (measured)

`probes/trail/laddertrailgrad.rs`, same layouts, log `probes/trail/gradient.log`.

| Quantity | Range over 8 layouts |
|---|---|
| Trail length | 96-138 columns |
| Scent at the near end, share of full scale | 0.08-0.24 |
| Scent at the far end | 0.41-0.64 |
| Difference between the cell east and the cell west of an ant (median along the trail) | **0.0052-0.0076** |
| The same, 3 cells either way | 0.016-0.023 |
| Difference between the two neighbours, relative to the scent where the ant stands | 0.014-0.026 |

- **The scent never changes during a run**, because the arena's frame does not step the scent planes. ~~So the gradient is the whole cue, apart from the trail's end 1-8 cells behind the group.~~ Corrected (measured by the reviewer): the trail's bare end is a large cue. Off the trail the scent is exactly 0, so an ant that steps only when there is scent ahead stops at the end, tumbles and walks back along the trail. Today's follower with its slope weight dropped (`Move ← PheroBFront` 20 kept) scores 0.311 alone and 0.171 as 6 on the signed worse side, 70% and 66% of the full follower. Slope only scores 0.094 and 0.039. With PR 713's `SCENT_GAIN` the edge reads 0.75-0.9 against about 0.15 for a one-cell slope, so the edge is the easier thing to find.
- **For the stage 2 brain (inferred from the design's pick rule).** Read as a share of full scale, the east option's "there minus here" is about +0.003 and the west option's about -0.003. With scores squashed, shifted and squared as the design's pick does, a 4:1 preference for the right way needs a squashed score of about 0.33. That means a weight of roughly 150 on that feature, or about 50 on "scent ahead" 3 cells out (about +0.01). The floor lowers the ratio further. The scorer's hidden layer can build the gain from two smaller weights in series (12 × 12), but mutation has to find both. Today's hand-built followers used weights of 20-40.
- **Recommendation:** give the scent-difference features a gain, or read them relative to the strength here, as real antennae compare. Then set the weight bounds with this number in hand. Done: PR 713 adds `SCENT_GAIN` = 32. Corrected detail: on a two-cell body, turning back is the `TurnAround` verb, not a pick of the west step, and with PR 713's pick floor (0.05) a 4:1 pick needs about 0.37, not 0.33. The order of magnitude stands.

## 3. The controls for the rung

| Control | How to build it | Should score (signed, worse side) |
|---|---|---|
| Rest-only | Rest preference at its maximum, everything else zero | 0 (measured on today's brain) |
| Blank brain | A uniform pick among legal options | Below 0: PR 713's blank mind reads -0.025, under rest's 0.000 (corrected from "about 0") |
| Straight walker | Never undoes its last step, reads no scent | At or below 0 (measured proxy: one-way walker -0.06 alone, -0.19 as 6) |
| Should pass | Hand-built in the new format: one weight on "scent ahead" for step options, with the gain from section 2 | PR 713's stage 2 follower: 0.860 as 6 clones, every ant the right way, before any training (cheat E2). Corrected: today's 0.44 / 0.26 is too low a bar for the new walk |
| Blind | The champion with only the scent features cut; self-senses and memory kept | At or below 0 (PR 713: follower blind -0.015). Cutting self-senses too would hide a heading-keeper |
| Edge-only (added) | The stage 2 follower with its scent-difference features cut and its "scent here or ahead" features kept | Today's equivalent scores 0.311 alone, 0.171 as 6. If edge-reading does not count as trail following, the champion must beat this; or bury the bare end under a long faint trail |

**Bar.** ~~The trained champion must beat the hand-built follower on the signed worse side by more than the noise bar (CV 0.12), as 6 clones.~~ Corrected: the trained champion must **reach** the stage 2 follower: signed worse side within the noise of its 0.860, and the `right` share at 0.8 or more, as 6 clones (PR 713's test asserts the same for the follower). Measure the noise bar on this arena from paired seeds; CV 0.12 came from another job, and a coefficient of variation means nothing near 0, where the controls sit. Today's trained brain reached 0.65-0.77 alone and 0.82 as clones on the clamped score, and on this arena the signed and clamped scores agree for an ant that never goes the wrong way.

**How to fold (added).** On the signed worse side rest beats every blind mover, so where that is the fitness, moving less is the first hill from blank. Inside a `Suite`, `f.max(0)` flattens everything below 0 instead, so there is no gradient until a mutant is positive on both sides. Recommended: fold each mirrored pair by its mean (blind walkers sit at about 0, rest at 0), keep the worse side as a gate and a second number, and print the share of brains whose most-picked verb is Rest in each early generation.

**Second numbers:**
- the lone-ant score;
- the share of ants that went the right way per layout (`end` past 0.1);
- cells walked, as the jam tell (a jammed crowd walks 28-60 cells a run; measured on today's walk, not the mind's).

## 4. What carries over unchanged

- The worse side of each mirrored pair (C4). It is still needed, but it is not sufficient on its own; see section 1.
- Clones in training and test (C7).
- EmitA and EmitB banned. In stage 2 the emits are brain outputs, so the ban must cover the new emit path too.
- Energy is 1e9 here, so hunger never varies. The trail champion learns nothing about when to stop; that comes with forage, on the scarce-food bed.
- Two horizons (E3): 6,000 and 12,000 frames.

## 5. Not checked

- **The blank stage 2 brain itself.** It needs the new act path. Its uniform pick has no heading at all, so it should sit nearest the p 0.2 walker, inferred from how the walkers move.
- **A forked trail** (`rungs.md`): leave it until a plain trail passes.

## Files

- `probes/trail/laddertrailnull.rs`: the floors probe. Genomes `zero`, `rw:<p>` or a file; `ants=`, `clones=`, `layouts=`, `frames=`. It prints clamped and signed scores, per layout and as the worse side. Not for merge.
- `probes/trail/floors/`: its logs and `jobs.txt`.
- `probes/trail/laddertrailgrad.rs` and `gradient.log`: the scent gradient.
