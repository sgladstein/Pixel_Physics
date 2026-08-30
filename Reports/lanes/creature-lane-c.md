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

### The pile is real, and I measured it anyway — the numbers matter for what comes next

- **It exists**, over 18 seeds at frame 18,000: median **11** free food
  cells within 2 of the nest against **1** for the same world with no
  colony, nonzero on 16 of 18 seeds against 11 of 18. **Paired within each
  seed: +9 cells, 15 up / 2 down** — the cleanest of the four trees. Spread
  is wide (0–32), so read the distribution, not a seed: the trajectory seed
  reads **0** at that frame.
- **The material is sharper than the count.** Summed over 18 seeds, a
  colony's band holds `litter 117, leaf 27, moss 18, seed 36, corpse 20`; a
  colony-free one holds `litter 95` and nothing else. The corpses are new
  since #142 — 144 deaths across 18 colonies where there was 1. Litter is what *falls*; moss and
  seed do not arrive by falling. That is the cleanest evidence the pile is
  delivered rather than ambient, and it needed no statistics.
- **It does not accumulate — and since #142 it is eaten to nothing.** On the
  trajectory seed the pile peaks at 11 cells at frame 1,600 (196 of 607
  deliveries made) and spends the back half of the run bouncing around zero
  while `eats` climbs 0 → 5 → 55 → 165 and the colony loses 12 of 52 ants.
  16,692 deliveries across 18 colonies against 138,115 pickups and 135,914
  drops — **88% of what an ant puts down it puts down away from the nest**.
- **It is a flow, not a store**, and this is the measurement I would keep if
  only one survived. Tracking the band as a *set of positions*: `resident` —
  positions occupied both at the first non-empty
  sample and now — **0 for almost the whole run, ending at 1**, while
  entries and exits track each other to within 4% (109 against 105). The
  first pile forms by
  frame 100 and is gone by 200. A standing ten and ten-in-transit are
  identical to a count.
- **The material keeps; the colony is the sink.** A hand-planted 40-cell
  pile in a colony-free world settles to 22–23 and holds for 18,000 frames
  on every one of 18 seeds. Put a colony on it and the paired difference is
  **−10 cells, down on 11 seeds of 18** (−6 on 13/18 before #142, −14 on
  15/18 after it — the sign is stable, the magnitude is not). The `litter`
  half rots into soil; the `leaf` half does not (`leaf.ron` has no
  `decays_into`). §5.3's own caveat about corpses keeping for ever is
  confirmed.
- **Peak worth is about one child.** Over 18 seeds the tight band peaks at
  2,440 digestible = **2.35** of your new `birth_cost` of 1,040 — but the
  colony-free control peaks at 1,307 = 1.26 on ambient litter alone, so the
  colony-attributable part is **1.09 children**. Note the direction: the
  pile got smaller and the priced figure went *up*, because #142 made a
  birth cheaper faster than the larder shrank. Face value would have said
  9.4: `diet_yield` at the ant's generalist gut keeps a quarter of a
  plant food's `food_value`.

### → Lane A, and this is the part that touches your 960

Your finding is that the body stamp — `body_energy × cells` = 960 of a 1,040
`birth_cost` — is what blocks a breeding colony, not the budget. **My answer
makes that sharper rather than moot, and in your favour.** A birth part-paid
from a nest store is one of the few routes by which that 960 stops having to
be saved up inside one animal's bank. The granary is not available as that
route today: `try_bud` charges `state.energy` and nothing reads the cells by
the nest.

And #142 has made the asymmetry visible in the source. You gave the
**replete** end a real heritable slot (`TRAIT_BIRTH_GRANT`, authored −0.2,
mutating every birth); the **granary** end still has no reader. Adding
`store_in_body` beside `birth_grant` would put two alleles in one genome
where one is connected to the simulation and the other is not — a harder
version of the bug to find later than an inert weight on its own.

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
   cell as stored. Measured sink rather than assumed: the planted pile ends
   at 13 with a colony and 22 without, over a window in which `eats` never
   exceeded 69 and `deaths` was 0 — so the removals are pickups, not meals.
   `ant.ron`'s `nest_memory` comment already describes the visible form of
   it (*"arriving, picking food up and then milling on the spot"*).
2. **#142 moved the frame at which a colony starts eating from ~10,500 to
   ~3,000, and gave it 134 deaths where there was 1.** At `start_energy: 200`
   the hunger threshold is 100, reached about three times sooner. Two
   consequences for anyone quoting a creature figure: a 6,000-frame harness
   now straddles the transition instead of sitting entirely before it, and
   **a short run and a long run no longer measure the same larder** — mine
   peaks at frame 1,600 and is empty by 15,000, where before it was flat.
   Any creature number taken on a fixed budget wants its frame named.

### The one I did walk into, and what it cost

My first summary printed a line headed *"paired, per-seed medians"* that
computed `med(colony) - med(no ants)` — **a difference of medians, which
discards the pairing the shared seed exists to provide.** It read +9 and
+19 on the pre-merge tree. Taken properly, within each seed, the same data
gave **+7 and +7** — an overstatement of about a third in both bands — and
the wide-band figure turned out to rest on 10 seeds of 18 rather than on a
population. Both versions are arithmetically correct; only one answers the
question, which is the shape `CLAUDE.md` warns about, and the heading is
what made it invisible. Fixing it cost a second 75-minute sweep, so the
probe now prints the per-seed rows as well and no one has to pay that twice.

### The trap I nearly walked into, for the record

The world-wide free-food count in the colony arm is 355–517 while the larder
holds 10. A census of every food cell in the world would have reported the
larder as fifty times its size, which is verbatim one of `CLAUDE.md`'s six
recorded instances (*a census counted every `Solid` in the world rather than
the platform under test*). `mode=control` tests it rather than intending it:
the same 40-cell pile planted at the far end of the world moves the world
column and leaves every band at 0.

The second one was quieter: face value 19,200 against 4,800 digestible for
the same forty cells. `food_value` is what a mouthful is worth to anybody,
`diet_yield` is what this gut extracts, and the ant's generalist gut keeps a
quarter of a plant food. Every worth figure I quote is the digestible one.

### Review card

Blind A/B posted to board `creatures`, id `20260830T014759506Z-618977`: the
colony's nest against a colony-free one at the same frame, daylight pinned,
asking which has taken 1,449 deliveries, with the counts in each item's
`meta`. Fire-and-forget — no verdict yet, and the finding above does not
depend on one, since the card asks whether the pile is *visible* and §0.1 is
about whether it is *spendable*.

### PR

I have the GitHub MCP tools, so the PR is opened from this lane:
**[#155](https://github.com/sgladstein/Pixel_Physics/pull/155)**, head
`06b7580` (this note's own commit lands on top of it). Coordinator owns the
merge.

Gates at the head: `cargo test --lib` 1089 passed / 0 failed / 54 ignored;
`cargo +1.98.0 clippy --all-targets -- -D warnings` clean; `docscheck`
clean; `mode=control`'s four assertions pass on every run.
