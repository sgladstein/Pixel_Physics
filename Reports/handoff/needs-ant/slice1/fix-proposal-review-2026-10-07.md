# Review of the slice 1 fix proposal (Nest building, 2026-10-07 03:1x)

Reviewing `fix-proposal-2026-10-07.md` and `results.md` (Redesign, 02:50-02:51). This is a code-read of
`claude/project-thread-ns0j6p` at 8298e59f (`src/sim/creature/needs.rs`) plus my own earlier traces. Nothing
was run.

## Verdict
**Agree, with two must-fix changes before building and three checks on the evidence.** Fixes 1-3 each answer
a cause the results trace. As written, Fix 4 and Fix 1's clearance cut use a trigger that never fires in this
walk. Together they would leave ants shut in a mound pocket with no way to dig out.

## Must fix before building

### 1. "A step the ant tried and was refused" never happens in this walk (Fix 4, and Fix 1 option a)
- **Code-read:** the walk's step draws only from `usable_headings` (`needs.rs` ~1153, 1331), as the shipped
  chooser does. A cell holding 4 ants, soil, or no footing is never offered, so a chosen step is not refused.
- **Measured** on the shipped chooser (my door-column traces, `nest-race/lane3/door-column/dieoff/`):
  - 0 refused steps in about 147k shaft decisions;
  - 0% in the seed 4 crush room, where 47% of decisions offered nothing toward the pull.
  - Findings trap, same day: blocked can't be read from the blocked outcome.
- **Fix 4 as written** turns escape's stall count from "no progress on the way out" (today: `walk_after`,
  `out_score`) into a count that never rises. Escape would never fire.
  - With Fix 1 removing every other dig urge, an ant sealed in a mound pocket or below ground could not cut
    out at all.
  - The no-veto guard's sealed-pocket rows should go red. If they stay green, the guard is blind.
- **Instead:** keep the progress measure. Just don't count decisions where the ant rolled to stay. Pass
  `walk_after` whether the step roll was won, not only `moved`. A won roll with no usable heading, or a step
  that didn't bring the out score down, is a real stall. A chosen stay is not.
- **Fix 1(a)'s trigger needs the same change.** It should fire after k decisions where the step roll was won
  and no usable heading brought the ant nearer its target, and the cell toward the target is soil. That last
  part is a cell-kind read (my `cellkind.patch` does this for the trace).

### 2. Fix 1(a) must be scoped to the door, or it brings Fault 1 back
- Results step 5: carriers "circle on top" of the mound.
- With an unscoped clearance cut, every circling carrier whose step home is soil cuts the mound's walls above
  the ground line. Those are exactly the cuts Fault 1 measured (1,197 against 683).
- **Scope it** to cells on the nest's way in, or the founding cut's columns, within a few cells of the door.
- Count clearance cuts by place, and gate them on "share of cuts above the ground line by non-diggers".

**Answer to Q1:** (a), with both changes. (b) depends on a stimulus that has never yet been seen to act
(dead-ends 1878, C2 inconclusive), so it can't be the only way in through a shut door.

## Check the evidence before relying on it

### 3. Which sample do steps 5-8 come from?
- `results.md` says the first walk trace sampled one ant in N by id, so every traced ant was a nest worker.
- The hash sampling landed in 8298e59f, at 02:56, after `results.md` (02:50).
- **Say which trace each of steps 5-8 used, for both games.**
  - Shipped also makes every fourth id a nest worker, so its 30 traced ants may be nest workers too.
  - If step 6's "empty ants outside stepped 12% against 49%" compares nest workers, it is not evidence about
    foragers, and Fault 2 rests on it.
- Steps 5 and 8 also cite census counts (all ants), which stand.

### 4. Step 7's "brain's own step chance is close to zero"
- If this was read from `deeptrace` `o_` columns before 8298e59f, it is wrong; your own note says 85% of
  them disagreed.
- The measured outcome (empty ants below the egg bar step 6-16%) stands either way.
- Re-read the brain's chance with the fixed readout before citing it as the cause.

### 5. Deep time on a near-zero base
- Scott's reporting rule says to give deep time as N of M.
- Fed ants deep and staying are 1.7-5.1 a seed against 0.2-0.3 shipped. "4.6-9.2x" is true, but it is a
  handful of ants in colonies that then collapse.
- Ant-time share also rises when the colony shrinks (the denominator). Quote the 100-150k window, before the
  crash, beside the 100-300k figure, so the gate isn't passed by the collapse.

## Each fix against its cause

**Fix 1 (only diggers dig).**
- Answers steps 1-2 (measured): 98% of the extra cuts came from drives that are not the dig job.
- Two links in the chain to the collapse are still inferred:
  - holders choke the mound (measured: 128-247 holders);
  - the door shuts (inferred, 57-90 maps against 5-23).
- That door is cheap to trace now, with no new run: `deep-trace/tools/doorseal.py` on seed 1's maps over
  87-93k, plus your dig record, names the sealing cell and who put it there.
- Under Scott's rule that should come before building. If it isn't soil from non-diggers, Fix 1 won't open
  the door.
- **Risk:** the new crowding stimulus is a new mechanism inside a fix round. Build it as its own named part so
  a failure can be pinned on it, and check "next cut at the face" and rooms first, as planned.

**Fix 2 (foragers walk, and give up outside).**
- Answers step 8 (measured, by census and code-read: the job never ends).
- Ending the job sends more ants home through mound tunnels full of soil holders. So run it with Fix 1, not
  ahead of it.
- Home cues over the mound are the false roads (findings, Deep trace 10-05). Your "where are they 2,000
  frames later" check is the right one; also count how many reach the dug nest.
- **Q2:** keep 0.5 as a flat floor while the job holds. A pace that falls with the stimulus is one more
  untraced constant, and the job's end already stops them.

**Fix 3 (purpose rule).**
- Nest race already built this as `meal` in d49d6cc9 (`meal_held`, keyed on `OrganismState::lunch`), not as
  part B (`giveup`).
- Its line is "under its grant". Yours is "past the hunger onset". **Pick one, and call `meal_held` from the
  walk** rather than writing a second helper.
- *(03:1x: the proposal's 03:00 note gives the helper to Nest race, with the walk calling it. Agreed. Settle
  the threshold with Nest race before it lands.)*

**Fix 4 (escape counts only real stalls).**
- Right goal, wrong trigger: see must-fix 1.

## The "not in this round" note on soil drops
Nest race's `sky` as built has a 2-cell clearance. That was my number, and on today's mounds it is too strict:
- only 1-13 of the 24 columns the ring draws have a legal sky cell at 2, against 8-22 at 1;
- carriers that reach an illegal one hold the pellet on the mound's top.

See `nest-race/sky-diff-coderead-2026-10-07.md`. The walk inherits whichever version lands, so let it
inherit only after that is fixed. Until then, results under `sky` will show holds moved, not removed.

## The runs (Q3)
Run all four fixes together first, on 4 seeds. Run the single-fix arms only if that fails the gate, to
attribute the failure before the kill rule is applied. That saves 8 runs if it passes. Either way, keep
`forage_trips` out of every check (step 8's trap). Use trip deliveries and visits to the food.
