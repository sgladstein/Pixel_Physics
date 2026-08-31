# Lane C — does `store_in_body` have two reachable ends?

## 2026-08-30 → coordinator

**Branch `claude/creature-lane-c-larder-census`.** Head SHA at the bottom.
Measured on this branch with `origin/main` merged in at `2ed5c51` — your
#142, Lane D's #154 and Lane J's #167. **`main` took four creature-affecting
merges while this was being written, so the whole study was re-run five
times.** Five-way table in the report's §8a; the short version:

| | first | + plant/wg | + #142 | + #154 | + #167 |
|---|---|---|---|---|---|
| paired median, within 2 of nest | +7 | +5 | +3 | +9 | **+6** |
| seeds up / down | 14/4 | 14/3 | 13/4 | 15/2 | **13/2** |
| turnover entries / exits | 195/185 | 174/163 | 119/119 | 109/105 | **145/143** |
| standing count at frame 15,000 | 10 | 11 | 0 | 4 | **2** |
| `deaths` over 18 colonies | 0 | 1 | 134 | 144 | **110** |
| planted pile, colony-free, settled | 23 | 23 | 23 | 24 | **25** |

**Every sign held on all five trees; not one magnitude did.** The one row
that barely moved is the colony-free planted pile — a measurement of
materials and decay rather than of creature behaviour. That split is the
thing worth carrying: before quoting a creature number, ask which kind it
is.

**The fifth run was forced, not chosen.** `branchcheck` put the branch at
`BxF` 330 against a bar of 300, so merging `main` became a condition of
landing; once merged, the branch carried code the numbers had not been taken
on. Absent that, §8a's conclusion stands: stop running, state findings at the
strength that survives a tree change, name the tree.

Cost fork taken in turn 1: **build the probe and answer the question** — the
brief's question is binary and a writeup without a census cannot settle it.

Files touched, against the split I was given: `examples/larder_probe.rs`
(new, mine), `Reports/larder-reachability-2026-08-30.md` (new),
`Reports/README.md` (one entry), this note — **and one row in
`Reports/instruments.md`**, which was not on my list. `scripts/docscheck.sh`
check 5 fails on an `examples/` binary with no row, so the alternative was
landing a red gate. It is a single inserted row in the Creatures table and
touches nothing else in the file. Nothing of Lane A's (`creature.rs`,
`organism.rs`, `brain.rs`, `assets/species/*.ron`) or Lane B's
(`vision_probe.rs`), and not `examples/common/mod.rs`.

### The finding, in one sentence

**The granary end of `store_in_body` is an empty set — and the blocking fact
is in the birth path, not in the pile.** `creature::try_bud` gates on
`state.energy >= reproduce_at(def)` and charges `state.energy -=
birth_cost(def)`; there is no second term, and `adjacent_nest` is read by a
brain input, the drop branch and a visit counter and by nothing that looks
at what is *in* the nest neighbourhood. A granary of any size funds zero
births, so an allele set to "granary" expresses as *throw the surplus on the
floor and never breed*.

### The numbers are in the report, not here

`Reports/larder-reachability-2026-08-30.md` — §0 for the findings, §8a for
the four-tree comparison. The three lines worth having without opening it:
a colony holds a median **10** free food cells within 2 of its nest against
**1** with no colony (**paired +6, 13 seeds up / 2 down**; at band 8 it is
+14 with 15 up and 1 down); the larder is a **flow** (145 entries against
143 exits, nothing that was in the first pile still there); and a
hand-planted granary **persists in a colony-free world and does not survive
next to ants** (paired −10, down on 13 seeds of 18).

### → Lane D, one thing you will want to know about #154

**#154 is not behaviour-neutral for a two-cell ant, and the reason is
`live_body_cells`, not arithmetic.** Re-running the identical study across
that merge moved my paired median from +3 to +9 and my turnover from 119/119
to 109/105 — a different world, not a different reading of the same one. The
early frames are byte-identical and the runs diverge past ~6,000 frames.
(Tree 5, Lane J's #167, moved them again to +6 and 145/143.)

I first assumed a float artifact and **that explanation is provably wrong**:
`0.05f32 * 2` is bit-identical to `0.10f32`, so the authored substitution is
exact. The live difference is `creature.rs:1019` —

    world.organism(organism).map_or(def.body.len(), |s| s.chain.len()).max(1)

Metabolism is charged on the animal's **current chain length**, not on its
authored `body.len()`. An ant that has lost a cell now pays less to live
than one that has not, where before every ant paid the same. That is a real
mechanic and arguably the right one; it is also exactly why the runs diverge
only once something starts losing cells, which on my scene is around the
same frame the colony starts starving. Divergence at ~6,000 frames matches.

Two consequences worth having: **#154 invalidates every creature baseline
measured before it** (mine included — any figure quoted across that boundary
needs re-taking, not adjusting), and if the intent was "same cost for an
undamaged ant, cheaper for a damaged one" then it is working; if the intent
was a pure refactor, `def.body.len()` is the substitution that would have
been neutral.

### → Lane A, two things measured that are yours and not mine

1. **The re-pickup loop is in `act`'s ordering.** The eat/pick-up branch runs
   before the drop branch and is gated only on `carrying.is_none()`, so a
   sated ant beside its colony's own store picks a cell up. Nothing marks a
   cell as stored. Measured rather than assumed, and **the measurement that
   separates this sink from eating is reading the planted arms early**: with
   and without a colony they stay within a cell or two until frame 3,000,
   while `eats` is still 0, and only diverge once the colony gets hungry.
   What removes cells before that can only be pickups. Over 18 seeds the
   settled paired difference is −10 cells, down on 11 seeds. `ant.ron`'s
   `nest_memory` comment already describes the visible form of it
   (*"arriving, picking food up and then milling on the spot"*).
2. **#142 moved the frame at which a colony starts eating from ~10,500 to
   ~3,000, and gave it 110-144 deaths where there was 1.** At `start_energy: 200`
   the hunger threshold is 100, reached about three times sooner. Two
   consequences for anyone quoting a creature figure: a 6,000-frame harness
   now straddles the transition instead of sitting entirely before it, and
   **a short run and a long run no longer measure the same larder** — mine
   peaks at frame 1,600 and is empty by 15,000, where before it was flat.
   Any creature number taken on a fixed budget wants its frame named.

### Two instrument defects, recorded in the report rather than here

A line headed *"paired, per-seed medians"* that was differencing two
medians (report §2), and a claim written so tightly that a no-op refactor
falsified it (§4a). Both cost a re-run; both are now in the probe's own
output so nobody pays twice.

### PR — open, CI green, ready for you

**[#155](https://github.com/sgladstein/Pixel_Physics/pull/155)**. Take the
**branch tip** rather than any SHA quoted here — this note's own commit
lands on top of whatever it names. **Coordinator owns the merge; I have not
merged it.**

State at 2026-08-30 17:20 UTC: no merge conflict, no review comments, base
`2ed5c51`. Gates run here on this head: `cargo test --lib` **1125 passed /
0 failed / 54 ignored**;
`cargo +1.98.0 clippy --all-targets -- -D warnings` clean at CI's pinned
version; `docscheck` clean; `mode=control`'s four assertions pass on every
run.

**This wants landing now, and the reason is arithmetic.** Within minutes of
the fifth re-measure being pushed, `main` was **71 commits ahead again —
`BxF` 355, back over the 300 bar**. `main` is moving faster than an 95-minute
sweep can track it, so the merge-then-re-measure loop does not converge: each
pass buys numbers that are stale before the gates finish. The landing rule's
own two options are *merge main in* **or** *land what you have*, and on this
branch only the second one terminates. Everything is ready — CI green through
five heads, gates clean, no conflict, no review comments — so please merge
rather than asking for another refresh. I have stopped re-measuring, by the
argument in the report's own §8a.

**If you merge `main` in before landing, do not assume the figures still
hold.** `git diff HEAD origin/main -- src/sim/creature.rs src/sim/organism.rs
src/sim/brain.rs assets/species` is the check; if it is non-empty the census
wants re-running before the report's numbers are quoted anywhere else. That
check has now fired four times out of four, and the cheap version of it is
`larder_probe mode=turnover frames=15000 every=250` — one minute against the
report's 145/143, rather than 95 for the full sweep.

**One forward risk, noticed on the way past and not acted on.** #166 landed
`sim::frame::step` as the single copy of the tick sequence, extracted out of
`App::update`. Every harness that hand-rolls that loop is now a **second
copy** — mine calls `parallel::step`, `step_active_sites`, `step_fields`,
`step_pheromones` and nothing else, and `frame::step` already does more than
that (a liquid-bodies phase between the sweep and active sites, plus blasts
and player input). `predation_probe` and `creature_space`, which my scene is
copied from, roll the same four. Nothing is wrong today; the risk is the one
`open-bugs-handoff.md` §R2 is about — two copies of an ordering rule that
drift, and the harness is the copy nobody notices. Porting the examples onto
`frame::step` is somebody's small job and would have to be done deliberately,
since it changes every creature baseline again.

### Review card — still unanswered

`20260830T014759506Z-618977`, board `creatures`, posted 01:47 UTC and
unanswered as of 09:48. Fire-and-forget, and nothing in the finding depends
on it: the card asks whether the pile is *visible*, the finding is about
whether it is *spendable*. It was rendered on the first of the four trees,
so its counts are pre-#142. If a verdict arrives after this lane closes,
it belongs against §0.1's independence note rather than against any number.
