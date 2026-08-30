# Lane C — does `store_in_body` have two reachable ends?

## 2026-08-30 → coordinator

**Branch `claude/creature-lane-c-larder-census`.** Head SHA at the bottom.
Measured on this branch with `origin/main` merged in at `5013c1a` —
including your #142 and Lane D's #154. **`main` took three
creature-affecting merges while this was being written, so the whole study
was re-run four times.** Four-way table in the report's §8a; the short
version:

| | first tree | + plant/wg | + #142 | + #154 |
|---|---|---|---|---|
| paired median, cells within 2 of nest | +7 | +5 | +3 | **+9** |
| seeds up / down | 14/4 | 14/3 | 13/4 | **15/2** |
| turnover entries / exits | 195/185 | 174/163 | 119/119 | **109/105** |
| standing count at frame 15,000 | 10 | 11 | 0 | **4** |
| `deaths` over 18 colonies | 0 | 1 | 134 | **144** |
| planted pile, colony-free, settled | 23 | 23 | 23 | **24** |

**Every sign held on all four trees; not one magnitude did.** The one row
that barely moved is the colony-free planted pile — a measurement of
materials and decay rather than of creature behaviour. That split is the
thing worth carrying: before quoting a creature number, ask which kind it
is.

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
a colony holds a median **11** free food cells within 2 of its nest against
**1** with no colony (**paired +9, 15 seeds up / 2 down**); the larder is a
**flow** (entries track exits, nothing that was in the first pile is still
there); and a hand-planted granary **persists in a colony-free world and
does not survive next to ants** (paired −10, down on 11 seeds of 18).

### → Lane D, one thing you will want to know about #154

**#154 is not behaviour-neutral for a two-cell ant, and the reason is
`live_body_cells`, not arithmetic.** Re-running the identical study across
that merge moved my paired median from +3 to +9 and my turnover from 119/119
to 109/105 — a different world, not a different reading of the same one. The
early frames are byte-identical and the runs diverge past ~6,000 frames.

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
   ~3,000, and gave it 144 deaths where there was 1.** At `start_energy: 200`
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

**[#155](https://github.com/sgladstein/Pixel_Physics/pull/155)**, head
**`3435378`** (this note's own commit lands on top of it, so take the branch
tip rather than that SHA). **Coordinator owns the merge — I have not merged
it.**

State at 2026-08-30 09:48 UTC: **CI green** on `3435378` (run 1431), no
merge conflict, no review comments, base was `5013c1a`. Gates run here on
that head: `cargo test --lib` **1105 passed / 0 failed / 54 ignored**;
`cargo +1.98.0 clippy --all-targets -- -D warnings` clean at CI's pinned
version; `docscheck` clean; `mode=control`'s four assertions pass on every
run.

**The branch is now behind `main` again and I have deliberately stopped
chasing it.** `main` took three creature-affecting merges during this lane
and each one moved the numbers; §8a of the report argues that the answer is
to state findings at the strength that survives a tree change and name the
tree, not to keep re-running. `BxF` is well under the 300 bar, so nothing
about landing requires another merge. **If you merge `main` in before
landing, do not assume the figures still hold** — `git diff HEAD origin/main
-- src/sim/creature.rs src/sim/organism.rs assets/species` is the check, and
if it is non-empty the census wants re-running before the report's numbers
are quoted anywhere else.

### Review card — still unanswered

`20260830T014759506Z-618977`, board `creatures`, posted 01:47 UTC and
unanswered as of 09:48. Fire-and-forget, and nothing in the finding depends
on it: the card asks whether the pile is *visible*, the finding is about
whether it is *spendable*. It was rendered on the first of the four trees,
so its counts are pre-#142. If a verdict arrives after this lane closes,
it belongs against §0.1's independence note rather than against any number.
