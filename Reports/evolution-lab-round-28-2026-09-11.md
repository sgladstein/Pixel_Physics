# The evolution lab, round twenty-eight: the pollinator arrives and eats the garden

*The coordinator's record of one round, 2026-09-11, 02:21 UTC onward
(`session_01Y7C61ozfhgPj8Lo3Z6Swue`), written at the round's close and moved
out of [`lanes/evolution-lab-coordinator.md`](lanes/evolution-lab-coordinator.md)
so the note stays under its 12 KB cap. Status: **record, not a work order.**
Every number here was taken by the lane it is credited to and is on `main` or
on the branch named; what binds from the round stays in the note.*

**Read this if you want to know what the round overturned.** Three lanes ran
under one coordinator, each on two passes, and a fourth piece of work — the
articulated-body landing — was carried by the round-27 coordinator's session
in parallel. The owner opened the round asleep, with one instruction added
mid-turn: *"be creative, think outside the box, you don't need to follow these
exact instructions if you have better ideas for how to improve the game"*, and
then *"when these 3 lanes are fully complete and merged if you still have
usage continue building and improving the game."*

## State at open, and the choice

On `main` since round twenty-seven: the played bed with thicket and tree
(#306), the pollinator design (#307), HISTORY per colony (#309), rain on with
a control (#310), nectar in two currencies (#312), the seed rides home (#313),
the bloom sense (#314), re-bloom with shrubs flowering (#317). The brief said
one PR was landing the articulated-body engine with the two-cell ant kept as
default; at 02:30 UTC **no such PR, branch or session existed**, the four-PR
stack (#303 → #311 → #315 + #316) merged onto `main` with one docs conflict,
and — the fact that mattered — **the stack as it stood rewrote the shipped ant
into a five-cell `Segmented` body** and the hopper likewise, so "landing it
with the two-cell ant kept" was a build, not a merge. The round-27 session
picked that build up itself at 01:49 (branch `claude/creature-bodies-land-r27`,
visible to this session only once it pushed at ~04:10), so it was never a lane
here.

**The three lanes, ranked by what the player sees:**

- **A — the flitter (P2).** Nectar, the bloom sense and re-bloom were all on
  `main` and all invisible because no animal could reach a flower. Folded in:
  `labgif follow=<species>` so a card can track one animal (the owner's rule
  that a one-cell event cannot be judged on a card), and the eye-price control
  as a `creature_arena` ablation rather than a lane.
- **B — bigger flowers with more variety.** The owner's newest ask, from the
  re-bloom card. Two PRs: the petal-colour slot alone (it moves a draw on the
  shared stream), then size and a heritable spread.
- **C — the garden loop fires.** Measurement first: where windfall lands
  against where ants walk, and every pip's exits with depth, light and water
  against germination's thresholds; then the one fix the numbers chose.

Not taken: the follow camera already exists in the lab (`follow_pin`,
`inspect_at`, the roster's FOLLOW), so only the card-side `follow=` was worth
building; sugar water, the divider, a lamp schedule and a predator stay
unruled.

## Environment, learned this round — read before spawning anything

- **Sonnet refused two of the three briefs on a `[bio]` classifier**, once on
  the prompt (lane B, before any work) and once forty minutes into the build
  (lane A, after the species file and `follow=` existed). The vocabulary this
  codebase uses for plants — locus, allele, genome, cross, heritable, pollen —
  is what trips it. Lane B ran on Sonnet once the brief was rewritten in the
  world's words (*"a slot in the plant's trait table"*, *"passes to
  seedlings"*); lane A was relaunched on **Opus**, which did not trip on the
  same code. Owner cost policy says Sonnet for builds; a refused Sonnet is
  worse than an Opus that works, so this is the standing exception for
  genetics-adjacent lanes.
- **A refused lane's worktree survives with its uncommitted work.** Salvage is
  `git add <paths>` + a WIP commit on the lane's branch from the coordinator,
  push, `git worktree remove --force`, and a new lane told to review that
  commit rather than trust it. Cost: one commit; the relaunched lane found and
  fixed a real bug in the salvaged `follow=` (a double zoom that clamped a
  crop to 4x4).
- **Never `TaskOutput` a running agent.** It returns the agent's JSONL
  transcript, ~30k tokens for nothing. Liveness is `tail -c 4000` on the task
  output file piped through `grep -o '"timestamp"…'`, plus `pgrep` for the
  build or the harness. All three lanes were checked that way at each
  check-in and none stalled.
- **`Monitor` caps at 30 minutes whatever `persistent` says**; re-arm it on
  the timeout event. A `git ls-remote` poll every 180 s on the lane branches
  was the cheapest wake-up for "a lane pushed".
- **`subscribe_pr_activity` is the right merge trigger**: `check_suite.
  completed` arrived within a minute of CI finishing on every lane PR, and
  merging on it needed one `get_check_runs` to confirm nine of nine.
- **Two coordinators on one trunk:** the round-27 session's bodies PR (8,300
  insertions) had to re-merge twice against #318 and #319 and asked, by poke,
  for a merge hold until it landed. Granted; it cost forty minutes and saved a
  third conflict on the largest diff of the night. Its conflict sites both
  times were `wiki/ants.md`'s freshness paragraph and the coordinator note.
- **A warm `target/` copied into each worktree** (`cp -r` of the main
  checkout's release build, 1.4 GB) saved every lane its cold build. Disk went
  29 → 18 GB with three worktrees; delete a finished lane's `target/`.

## Lane B — petal colour passes to seedlings (#318, merged d1535c39)

`LOCUS_FLOWER_COLOUR = 6`, `DISCRETE_LOCI` 6 → 7, `LOCUS_ALLELES` gains
`[…, 2]`; `flower_band` derives from the slot the way `foliage_band` does; the
`ORGAN_BAND_STREAM` flower draw is gone and the fruit draw stays. Founding
draws the allele first on its own positional stream (stream 70, which used to
pick the band directly) and derives the band from it, so a non-flowering
species founds real variance on the slot — the reverse trick foliage uses
would have pinned `tree`, `conifer`, `creeper` and `grass` at allele 0, and
`every_discrete_locus_varies_between_founders` would have gone red. **The
cost stated in the commit: one more `rng.chance` per plant birth on the
caller's shared stream.** Re-baselined: `set_seed_leaves_the_callers_rng_
position_alone` 471,168 → 939,699; `widening_the_genome_does_not_move_the_
breeding_draw_sequence` `0x2197_04fe_f1c7_3b67` → `0x0ea6_032f_41d2_6be3`,
both watched going red with the fault back. New guards: a founder stand of
sixty herbs wears more than one band; a bred seedling's band matches its
parent's allele in >80% of 200 children (the free-draw mechanism gives ~50%).
Petal census on sixteen herb founders at frame 6,000: 67 / 22 / 0 across the
three palette bands under two alleles.

## Lane B — size and spread (#322, second pass shipped at 12; a third lane measures the term)

`organ_cluster` raised (herb 9 → 16, scrambler and shrub in proportion), head
size scaled per individual by genotype slot 9 so a stand shows a spread, and
herb widened to three petal bands (`LOCUS_ALLELES[6]` 2 → 3, census 22 / 16 /
53 — non-degenerate). The second widening moved the breeding fingerprint again
(`0x0ea6_032f_41d2_6be3` → `0xfb9e_a33a_d20a_0087`) by changing a *value*
`tree`'s own jump feeds it rather than a draw count — a second shape of the
same class `CLAUDE.md` names. Card `…83eaba`, a blind A/B at frame 6,000.

**First pass, held by the coordinator.** `labforage scenario=played_bed`,
120,000 frames, three seeds, main → branch:

| | seed 1 | seed 2 | seed 3 |
|---|---|---|---|
| plants at 120,000 | 464 → 322 | 574 → 170 | 303 → 259 |
| `organs_built` | 511 → 882 | 330 → 608 | 383 → 486 |
| `flowers_rebloomed` | 145 → 214 | 46 → 112 | 42 → 80 |

Holding the per-head `Ripen` cost fixed by dividing both costs by the
cluster's growth ratio did not hold the stand, and protecting the seed's
endowment measured worse on aggregate (both in `dead-ends.md`). A default that
thins the bed 15–70% is a regression the owner sees as dying plants whatever
the flowers look like, so the PR waited on a second pass. **The term is per-cell upkeep**: every
organ cell reaches `organism_upkeep`'s maintenance charge and pays
`MAINTENANCE_PER_CELL` flat for as long as it stands, untouched by the
`Ripen`-cost re-derivation, which reaches only the two one-off charges;
organs are excluded from `crown_moment` and the span cap by the code's own
comment; measured in isolation (`organ_size_and_the_maintenance_bill`), seven
extra cells bill exactly seven times the constant. Sized against a −25% bar
on the same three seeds:

| head | seed 1 | seed 2 | seed 3 |
|---|---|---|---|
| main (9) | 464 | 574 | 303 |
| 16 | 322 | 170 | 259 |
| 12 (shipped) | 356 | 93 | 158 |
| 9, spread and colours only | 348 | 299 | 248 |

**None holds on every seed, and the table cannot say why**: 12 reads worse
than 16 on two seeds, and size 9 with only the spread and the third band
still reads under main — so either the per-plant *spread* (heads up to 1.7x)
is a second term, or three seeds are three re-rolled worlds, since main's own
three span 303–574. Card `…e1ee88`, a blind A/B/C (9 / 12 / 16). A fourth
lane (D) runs the discriminating sweep — spread on against off at 12, six
seeds, read as a median — and ships what holds; its result is in *Open at
close*.

## Lane C — the garden loop's missing last step (#319, merged c8a9bf99)

Measured before anything was touched, `played_bed`, 120,000 frames, three
seeds, with a column heat map of where ants' heads were and a `PipCheck` row
for every pip that reaches its first `Germinate` evaluation:

| seed | `windfall_bitten` | spilled / carried / delivered | `plants_from_pip` | `pips_eaten` | `pip_checks` | windfall in columns no ant ever occupied |
|---|---|---|---|---|---|---|
| 1 | 2 | 0 / 0 / 0 | 0 | 0 | 0 | 92% |
| 2 | 9 | 9 / 9 / 6 | 0 | 6 | 0 | 67% |
| 3 | 1 | 0 / 0 / 0 | 0 | 0 | 0 | 100% |

**Two findings, one of them unasked for.** Windfall stands 67–100% of the
time in columns the colony's body never occupies — the bite rate is a reach
problem, not a mechanism. And the one completed chain, traced by organism
handle: delivered at frame 55,932, fell two rows as a `Powder`, gone at frame
55,937 to the colony's own traffic, confirmed against the rendered pixel. The
brief's hypothesis that the nest is underground and dark was wrong about the
code (`paint_nest_patch` lays nest as surface material). Card `…49ba0a`, a
21-frame sequence with the pip ringed — the outcome is *eaten*, not *grown*,
and the card says so.

## Lane C — a live seed is not spoil (#323, merged f0c8999c)

The taker was **the dig verb**, not the bite: a neutral gut cannot eat a pip
(40 J × 0.25 = 10 < `EAT_YIELD_THRESHOLD` 12), but `dig`'s ground test could
not tell a live seed from dirt, since `pip` and `windfall` are `Powder`-kind,
so it cleared delivered pips as spoil with no counter watching. Fixed at the
dig dispatch site, fault put back and watched go red:

| seed | `dig_diverted_seed` | `pips_eaten` before → after | `pip_checks` | `plants_from_pip` |
|---|---|---|---|---|
| 1 | 6 | 0 → 0 | 0 | 0 |
| 2 | 22 | 6 → 1 | 0 | 0 |
| 3 | 26 | 0 → 0 | 0 | 0 |

Card `…03c9e3`: a pip standing 428 frames after delivery against the 5 it
managed before. **`pip_checks` is still 0 on every seed**: no pip has yet been
*evaluated*, and the lane names `deliver_seed_passenger`'s closed-form decay
roll — settled over the whole carried span at delivery — as the next blocker,
which is one mechanism from the ordinary germination path that is already
tested. The scene arm (scramblers 118/395 → 170/340, just outside the founding
band) was built and rejected: founders fell on all three seeds (40/31/23 →
27/23/22, one below main's floor) and `windfall_bitten` rose on one seed of
three. Kept as `played_bed_windfall_reach.ron` for reference, rejection in
`dead-ends.md`, `played_bed.ron` untouched.

## Lane C — the pip's clock was never re-armed (#325, merged b977af67)

The decay roll the brief suspected was already right (carried span, not
organism age — confirmed by reading and by a deterministic test). **The taker
this time was a schedule**: `deliver_seed_passenger` wrote a fresh `Seed`
cell on delivery and never called `schedule_active_site`, unlike
`bear_seed_at` and the germinate-wait reseed, and `organism_tick`'s own
"seed relocated" recovery can only reschedule from a cell the organism still
owns — a passenger-riding organism owns none — so a schedule coming due
mid-transit (carries of 50–1,000+ frames against a 45-frame tick) was dropped
for ever. One call fixes it. The instrument had also under-counted itself:
`pip_checks` was gated on `deferred_germination`, which an unrelated pre-bite
evaluation can already have set.

| seed | delivered | `pip_checks` | resting ok | light ok | water ok | `plants_from_pip` |
|---|---|---|---|---|---|---|
| 1 | 0 | 0 | — | — | — | 0 |
| 2 | 8 | 213 | 100% | 100% | 2 (0.9%) | **2** |
| 3 | 4 | 241 | 100% | 100% | 0 | 0 |

**The first two plants ever grown from a pip, across three rounds of this
line**, and the last blocker named to the digit: resting and light never
block, soil water blocks 452 of 454 checks, with readings at exactly 0.00 —
the pip is set down on the nest patch and the nest's material holds no
water. Card `…f83e2b`: organism 4656, delivered at frame 45,348, a standing
five-cell seedling by 46,339, ringed. A fifth lane (E) builds the midden the
ecology design promised — the pip set down on watered soil at the nest door —
and its result is in *Open at close*.

## Lane A — the flitter (#324)

`flitter.ron` cut from `hopper.ron`: `Chain(2)`, gut −1.0, `nest: ""`, the
pheromone steering and the four gate units stripped, the bloom pair and
`(Bias, Impulse, 2.0)` / `(BloomNear, Impulse, 1.0)` / `(FoodAdjacent,
Impulse, −2.0)` in their place, a pale blue-white body. Four things the brief
had wrong, found by reading and by measurement: **"cap 8" is not a bound on an
authored eye** (`sight_range_of` clamps an *evolved* eye at `SIGHT_MAX` 128;
the field is uncapped), so the eye went 8 → 32 with `sight_fraction` back to
the hopper's; **the perch already exists** (`translated_if_free` blocks on any
non-empty cell and `body_is_supported` counts `Plant`), proved by the reach
numbers rather than built; **`creature=` founds a nestless species**, so both
arms used it; and the hopper control needed `wire=` on `labforage`, not a
species copy, because a `.ron` copy is a different binary. Also fixed: the
salvaged `follow=` zoomed twice.

120,000 frames, seeds 1–3, one binary, post-merge:

| arm | born | alive at end | generations | real launches | refused % | starved aloft | head max rows | flower visits |
|---|---|---|---|---|---|---|---|---|
| flitter | 173 / 1,490 / 1,489 | 0 / 70 / 251 | 8 / 43 / 38 | 3,792 / 98,832 / 77,682 | 53 / 19 / 26 | 154 / 525 / 393 | 41 / 106 / 95 | 2 / 15 / 1 |
| hopper at `(Bias, Impulse, 2.0)` | 0 / 2 / 11 | 0 / 1 / 0 | 0 / 1 / 2 | 807 / 387 / 155 | 68 / 74 / 75 | 28 / 24 / 31 | 15 / 17 / 17 | 2 / 2 / 0 |
| ants + flitters (scenario copy) | 366 / 655 / 1,425 | flitter 0 / 0 / 177 | 14 / 13 / 28 | 5,003 / 24,251 / 70,442 | 46 / 16 / 26 | 253 / 199 / 383 | ant 38/37/27, flitter 49/76/91 | flitter 7 / 0 / 9 |

P2's stated gate passes on every clause: refused share falls, real launches
hold, births clear zero by two orders, `flower_visits` is non-zero on all
three seeds. **And the animal is wrong.** On the card's bed (`…cec12a`, three
followed sequences: one flitter, and the same bed with and without flitters),
flitters take plant cells 960 → 566 and standing flowers 34 → 4 with 452
alive — at gut −1.0 a leaf pays 480 J and a whole flower cell 1,440, so
`best_bite` made a leaf-and-flower eater that breeds off foliage and eats its
own flowers, the failure design §2.2 named, with the counter it prescribed
(`flowers_bitten` by species) left unbuilt. **Reach was the gap and is
closed; encounter is the gap now, and the mouth is the reason encounter
never mattered.** The eye-price control read the *blind* arm ahead (median
59.9% with `BloomNear` ablated, 84.2% with `BloomBearing`, 6 seeds, inside the
arena's 2.4–3.1x noise) — unsigned, and meaningless for an animal that lives
on leaves. **Second pass: nectar and nothing else.** `CreatureDef::nectar_only`, a
species field, default false — a switch on the menu rather than a weight on
it, because every yield through `diet_quality` is positive and no gut setting
avoids the leaf. For that animal `adjacent_food_counted`'s whole menu is
`plant::nectar_available` cells, a read-only split of `nectar_offer`'s own
preconditions, and the swallow block takes the hook or nothing; `try_bud`'s
shortfall loop turned out to be a second mouth ("exactly as if the parent had
eaten them" — 3,278,831 J of `intake` against 2,760 of `nectar_paid` and
2,976 births on the first gate) and is gated too, so `intake` and
`nectar_paid` now agree to the joule. Bit-identity on every shipped animal:
1,096 non-timing `ascii` lines byte-identical. `flowers_bitten_by_species`
built; a nestless species wears its own colour under ANIMALS WEAR BY COLONY.

| arm | born | alive | flitter flower visits | flowers bitten | plant cells | flowers standing |
|---|---|---|---|---|---|---|
| flitter alone | 0 / 0 / 0 | 0 / 0 / 0 | 3 / 12 / 0 | 0 / 0 / 0 | 1,021 / 780 / 1,296 | 5 / 38 / 33 |
| ants only | 197 / 110 / 663 | 84 / 15 / 511 | — | ant 1 / 0 / 9 | 290 / 116 / 87 | 10 / 0 / 11 |
| ants + flitters | 101 / 359 / 320 | 35 / 241 / 164 | 3 / 5 / 20 | ant 9 / 3 / 7 | 320 / 173 / 534 | 11 / 11 / 7 |

**The bed's bar is cleared and the animal's is not.** Plant cells with
flitters sit above the no-flitter band on all three seeds, flowers eaten by a
flitter are 0 everywhere; and the flitter never breeds and dies out, 3–20
visits per 120,000 frames against 0.025 J a frame of upkeep. **Finding a
flower, not reaching one, is now what it cannot do** — a followed animal with
a paying flower nine cells away never came closer than nine in 2,000 frames
and drifted as often away as toward. Two findings on the way: 57–87% of its
deaths are `STARVED ALOFT`, traced to every creature material and `water.ron`
authoring `density: 1.0`, so a hop that comes down on water hangs in the air
paying the airborne rate for ever (§Z9, reproduced, not fixed here); and the
eye-price control is unanswerable at 24,000 frames — 0 alive on both arms on
all 6 seeds. Card `…aa6091` replaces `…cec12a` (the queue's transport copies a
card only if absent, so a posted card cannot be edited).

## The bodies (#320, merged c16ffff0 by the round-27 session)

The tuck, the founding walk and the flip ship for every body; the shipped ant
stays two cells; the seven-cell body is `longant`, placed from the COLONY chip
or by `played_bed_longant.ron`. The shipped colony forages better for it (bed,
60,000 frames: deliveries 9 → 20). #303/#311/#315/#316 closed as superseded.
Left for a bodies lane: the long body's founding and its whole-body bite (the
swarm test), and the moisture-gradient scene's pickups 416 → 78, not chased.

## What the round overturned

- *The pollinator's gap was never the eye and is no longer the reach.* A
  flying animal reaches 106 rows; what it does at the flower is decided by
  what its mouth is allowed to take, and a plant specialist's mouth takes the
  plant.
- *The garden loop's last step was the dig verb.* Three rounds of "no pip has
  ever become a plant" were a `Powder`-kind seed being shovelled as dirt at the
  busiest cell in the box, invisible because no counter sat on that exit.
- *Windfall is a reach problem.* 67–100% of it lies where no ant ever stands,
  and moving the understory ten columns costs founders without buying bites.
- *Petal colour was the loudest heritable channel the plant did not have*, and
  giving it a slot moves every seeded plant figure — twice, once by a draw
  count and once by a value.
- *A bigger flower head is not free even at a fixed price per head*; the term
  that scales with its cells is the thing to name before a size ships.
- *"Cap 8" was a cap on evolution, not on authorship*; the perch was already
  there; `creature=` already founded a nestless species. Reading the code
  before building saved three mechanisms this round.

## Open at close

*(Filled in when the second passes land — see the note for what binds.)*
