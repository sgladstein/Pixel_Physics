# Lane C — does `store_in_body` have two reachable ends?

## 2026-08-30 → coordinator

**Branch `claude/creature-lane-c-larder-census`.** Head SHA at the bottom.
Measured on `56b6b97` — this branch with `origin/main` merged in. **I merged
main and re-ran everything before publishing**, and it mattered: 53 commits
(441 lines of `plant.rs`, 716 of `worldgen/passes.rs`, nothing in
`creature.rs`) moved the paired median from +7 to +5 and the settled band
from 13 to 11. Every number below is post-merge.
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
  cells within 2 of the nest against **3** for the same world with no
  colony, nonzero on 15 of 18 seeds against 9 of 18. **Paired within each
  seed: +5 cells, 14 up / 3 down** — and at band 8 it is 11 up / 7 down,
  near a coin flip, so the effect lives exactly where a delivery lands (<=2
  by construction) and nowhere wider. Spread is enormous (0–43, empty on 3
  seeds), so read the distribution, not a seed: the one seed I first ran
  read 3 and would have understated it nearly 4x.
- **The material is sharper than the count.** Summed over 18 seeds, a
  colony's band holds `litter 53, leaf 25, moss 46, seed 78`; a colony-free
  one holds `litter 82` and nothing else. Litter is what *falls*; moss and
  seed do not arrive by falling. That is the cleanest evidence the pile is
  delivered rather than ambient, and it needed no statistics.
- **It does not accumulate.** 20,506 deliveries across 18 colonies against
  157,788 pickups and 156,434 drops — **87% of what an ant puts down it puts
  down away from the nest**. On the trajectory seed the standing count
  plateaus by frame 3,000 and never rises again.
- **It is a flow, not a store**, and this is the measurement I would keep if
  only one survived. Tracking the band as a *set of positions*: `resident` —
  positions occupied both at the first non-empty
  sample and now — **zero from frame 200 onward** (174 entries, 163 exits).
  The first pile forms by
  frame 100 and is gone by 200. A standing ten and ten-in-transit are
  identical to a count.
- **Persistence is not the blocker.** A hand-planted 40-cell pile in a
  colony-free world settles to 22–23 and holds for 18,000 frames, on every
  one of 18 seeds. The `litter`
  half rots into soil; the `leaf` half does not (`leaf.ron` has no
  `decays_into`). §5.3's own caveat about corpses keeping for ever is
  confirmed.
- **Peak worth is about half of one child.** Over 18 seeds the tight band
  peaks at 2,427 digestible = 1.30 `birth_cost`s — but the colony-free
  control peaks at 1,420 = 0.76 on ambient litter alone, so the
  colony-attributable part is **0.54 of a child**. Face value would have
  said 5.2: `diet_yield` at the ant's generalist gut keeps a quarter of a
  plant food's `food_value`.

### → Lane A, two things that are yours and not mine

1. **The re-pickup loop is in `act`'s ordering.** The eat/pick-up branch runs
   before the drop branch and is gated only on `carrying.is_none()`, so a
   sated ant beside its colony's own store picks a cell up. Nothing marks a
   cell as stored. Measured sink rather than assumed: the planted pile ends
   at 13 with a colony and 22 without, over a window in which `eats` never
   exceeded 69 and `deaths` was 0 — so the removals are pickups, not meals.
   `ant.ron`'s `nest_memory` comment already describes the visible form of
   it (*"arriving, picking food up and then milling on the spot"*).
2. **`eats` is 0 until about frame 10,500.** An ant starts at 900 and only
   swallows below `900 * hunger_fraction` = 450. Any creature harness whose
   budget is 6,000 frames is measuring a colony that has not begun to eat,
   which is worth knowing before a foraging or predation figure is quoted
   off one.

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
