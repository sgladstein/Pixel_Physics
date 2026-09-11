# The evolution lab, round twenty-seven: the flower gets a customer it cannot yet reach

*The coordinator's record of one round, 2026-09-10 to 2026-09-11
(`session_01NGdywxc1ACg3L5scK7xBTc`), moved out of
[`lanes/evolution-lab-coordinator.md`](lanes/evolution-lab-coordinator.md) at
the round's close because the note had reached 20 KB against its 12 KB cap.
Status: **record, not a work order.** Every number here was taken by the lane
it is credited to and is on `main` or on the branch named; what binds from
the round stays in the note.*

**Read this if you want to know what the round overturned.** Eight lanes ran
under one coordinator in about nine hours; six landed on `main`, two are
stacked on the bodies branch waiting on the owner. The design of record for
the pollinator is
[`evolution-lab-pollinator-design-2026-09-10.md`](evolution-lab-pollinator-design-2026-09-10.md)
(PR #307, this round); for the bodies, `creature-articulated-body-2026-09-09.md`
§13, on branch `claude/creature-mobility-r27` (PR #311) and not on `main`.

**State at open.** Round twenty-six's cards came back all at once (synced
15:31 UTC) and the round opened on them at 16:00. The verdicts: the thicket
goes in the default played bed and so does a tree (*"a mix is best"*); rain
ships on with the control kept; the HISTORY page wants a summary per colony
as its default; **the animals carry pollen**, and *"we will probably need
creatures that are more pollination motivated (like a bee/butterfly)"*; the
seed rides home; the pip card could not be read (a still of a one-cell event
— the next card is a zoomed moving sequence); the articulated ants read as
stuck and flashing. Three rulings followed in chat and bind harder than any
of them: **the bodies are a mechanical problem — larger ants get stuck in
complicated terrain, fix that** — and the owner proposed the fix, a flip of
the whole body in place; **ants are not the main pollinators**, a different
creature should be flower-focused, and crafting it by hand is acceptable for
now because evolution alone has not delivered; and **conserve tokens** — no
more lanes spawned to answer questions, only for builds the owner asked for
or a landing needs.

**The bed and the rain (PRs #306, #310).** Four scramblers and a tree joined
the played bed. The thicket copied straight from the measurement bed sat in
the colony's founding gap and seated **2 ants of 52** (6 by 30,000 frames)
against 29 at 6,100 and 38 at 30,000 once moved out of columns 180–330 —
looking before committing caught a scene error that would have read as a
mobility bug. Rain ships **LIGHT**: with the tree in, the bed unwatered loses
16% of its soil water on seed 1, LIGHT holds both seeds within 7%, STEADY
overshoots 15–19% and pools.

**The pollinator design (PR #307, Opus).** Three measurements moved the
build. The thicket is the pollinator's larder: standing flowers at frame
6,000, median 60 on the thicket bed against 16 on herbs. **The bed stops
flowering on its own and the colony is not the cause**: 81 → 10 → 3 flowers
by 40,000 with the colony *removed*, because herb and scrambler are
determinate — an axis ends in a flower and stops — so the first build was a
flower that renews, not a rule keeping ants off flowers. And "nectar out of
the reproductive budget" was a units error in the ecology design (plant
carbon capped at 4.0, where a fruit costs 0.3, set against creature joules):
nectar is two numbers and an exchange rate. The species is `flitter`; the
build order is B1′ → (P1 the bloom sense ∥ B2) → P2 the flitter → C1 the
animals carry pollen → C2 the petal-colour locus → I the instruments. A
side finding on the hopper: the 2.0 hop the owner liked makes *fewer* real
launches than 0.5, and the repair is one wire, `(FoodAdjacent, Impulse,
−2.0)`, not the recorded dead end.

**Nectar (PR #312).** `OrganismCell::nectar` refilled per tick, `NECTAR_COST`
0.01 budget units, `nectar_yield` 120 J through `diet_quality`, the flower
stands; `flower.food_energy` 1,440 stays for a gut that cannot take nectar.
Over a full session on the played bed **`flower_visits = 0`**: no ant reaches
a flower 22 rows up a stem. Five unit tests including the positive control
prove the zero is the world's.

**The seed rides home (PR #313).** A bite's surviving pip rides in the crop
under its own organism id — same genome, lineage and endowment, no re-mint —
and is set down at the first cell the crop drops, with the decay clock
settled as one closed-form roll over the carried span. Median carry 175
frames against a 14,000-frame half-life, so transit costs nothing. Over
120,000 frames on three seeds only one seed bit fruit at all (6 carried, 4
delivered); the other two are byte-for-byte controls. **`plants_from_pip`
is 0 in every arm and every seed**: no pip has yet become a plant anywhere.
The mechanism is in; the rate is the thicket's and the rot clock's problem.
Card `…4ec0c3`, a zoomed moving sequence of the two deliveries.

**The bloom sense (PR #314).** Two brain inputs, `BloomNear` and
`BloomBearing`, recorded on the rays the eye already casts for prey, kin and
threat; `BRAIN_INPUTS` 27 → 29; `mutation_rate` re-derived to 3.18/809 on
every species carrying the field, because a wider genome consumes a
different draw count per birth (the same shape as the earlier `KinNeed` /
`Share` append; the breeding ant colony scene diverges from its first
birth, every other scene is bit-identical). No ant is wired to it, per the
ruling. The hopper, wired: `bloom_seen` 0 / 27 / 56 on seeds 1–3, and it
climbed to **21 rows where the unwired control reached 6 — and the flower
stands at 22**. `flower_visits` 0 on all six runs, so the hopper wiring is
not shipped and is in `dead-ends.md` with its re-test condition. A ground
animal with eyes and a pull is still not a pollinator.

**Re-bloom, and shrubs flower (PR #317).** `rebloom_after` on the species
(herb 4,000, scrambler 3,000, shrub 16,000): the settled stem cell proximal
to a departing fruit is queued and re-flowered from the reproductive budget
at the species' flower ripening cost. Shrub becomes a flowering species for
the first time (a four-metamer determinate rule, ripening rates scaled 4.17x
off herb). Against the stated bar — hold standing flowers at a third of the
frame-6,000 count on a majority of seeds — shipped clears all four required
combinations, the control two of four; paired at 120,000 frames 4 of 6
(seed, colony-state) pairs favour shipped and 2 reverse. `flowers_rebloomed`
0 → double and triple digits; `organ_ripening_blocked` 1.2–3.8x higher;
**plant counts run lower under rebloom in all six pairs**, flagged rather
than explained. The "81 flowers" anchor the brief carried belonged to the
thicket bed, not the played bed (34 / 16 / 12 at frame 6,000). Card
`…3257b2`, a blind A/B.

Its CI failure is the round's cleanest method lesson. `ascii`'s
sessile-colony guard (`forage_trips >= 6` on the 12,000-frame ant colony
scene) read 5 against 15 on `main`, deterministically, with or without the
bloom-sense merge. Not the timer: shrub's 16,000-frame `rebloom_after` is
longer than the scene, and forcing it to 0 everywhere left the count at 5.
**Shrub becoming determinate is the whole cause**: worldgen sows it across
the scene's terrain beside the trees the ants eat, and a shrub that stops
growing once its axes flower competes less with those trees. Every
foraging counter improved — deliveries 418 → 845, eats 1,149 → 1,497,
deaths 7 → 4, standing food energy +13% — and the reach histogram moved from
[681, 84, 37, 15] to [725, 62, 21, 5]: **fewer long excursions is a colony
that stopped needing them.** `forage_probe seeds=N`, which the guard's own
comment named as the seed-axis instrument, cannot see this at all — its scene
is a hand-built stone floor with no worldgen — so the sweep was done by hand
over the scene's worldgen seed: shipped [5, 16, 0, 27, 2] against [15, 16,
0, 34, 2], three of five seeds untouched and the other two at 3.0x and
1.26x, both within the 4.1x the file already priced. The bar is now 3, the
trade stated in the comment, and the wrong claim about `forage_probe`
corrected.

**The bodies (PRs #311, #315, #316, stacked on #303).** The mechanical
answer to the ruling, from the round's first hours: **a body longer than two
cells cannot turn round, and that is the whole of it — length, not width.**
A body follows its head and a landing may not put two cells in one place,
so the only own-cell a head may land on is the tail; for a five-cell body
that is four cells away and unreachable, and in a dead end it is stuck for
ever. The classifier's histogram (two-cell ant 0.6 / 4.0 / 5.1 / 8.6% blocked
on flat / rolling / tunnel / chamber; the wide articulated ant 1.9 / 22.1 /
87.1 / 23.0%) has `boxed_self` equal to `boxed` in all sixteen rows. The
owner's flip — the chain's order swaps, no cell moves — brings the wide body
to 2.3 / 7.1 / 7.6 / 11.5%; backing out was built and rejected (49% still
blocked). It shipped default-off because foraging collapsed with it on
(deliveries 23 → 0).

Lane F found why and turned it on (#316): not state loss and not the
blocked counter read as giving up, but that `is_boxed` cannot tell a dead
end from another ant momentarily standing in the one open heading, so the
flip turned laden foragers round exactly where that misread is commonest,
beside the nest. A laden animal now defers the flip one tick when only
another body is in the way: foraging scene, 12,000 frames, deliveries 297
off / 233 ungated / **290** shipped, round trips 14 / 7 / **25**; mobility
on the four walk presets bit-identical to the flip arm. Two broader gates
were built and rejected — a boxed-tick streak and the traffic check on every
animal cost 16.3% and 77.5% blocked on `tunnel`, where a food-free burrow
packed with animals makes "another ant is in the way" nearly universal.

Lane G founded the body along the surface (#315): each segment tries
flat-with-foothold, then up, then down, then the old straight lay, and the
walk is the viability check. Viable sites 5 → 9 on the colony scene and
11 / 13 / 13 → 13 / 15 / 14 on the played bed, never fewer — **and the
two-cell control founds only 18–29 of 52 on the same bed**, so most of the
remaining gap is the bed's litter and the span-scaled spacing corridor
in `colony_stations`, not spine shape. Card `…7f9bac`. Three reds stay red
and are the owner's ruling: the §9 swarm guard (median 101 against 77 —
the whole-body bite scan), the chamber's `roofed > 0` (still 0 with the
flip; what a colony chooses to dig), and founding at 9 of 52 on the colony
scene.

**What the round overturned.**

- *"Flower sense" is not a new sense.* It is the eye, on rays already cast,
  and **the eye is heritable** (`TRAIT_SIGHT_RANGE`, cap 8, about 4%
  metabolism per 32-cell eye) — the coordinator's claim that only animals we
  give eyes could evolve toward flowers by sight was wrong, and the owner
  corrected it. Whether selection would ever pay for an eye against nectar
  is unmeasured; it is a control in the flitter lane, not a lane.
- *A ground animal that can see a flower still cannot reach one*: 21 rows
  against 22. The pollinator has to fly.
- *The long body's whole immobility was length*, and the flip's foraging
  collapse was a traffic misread, not the trail constants — one gate on
  laden animals restored 97% of deliveries.
- *Founding is not length-bound alone*: the two-cell ant itself founds
  18–29 of 52 on the played bed.
- *Every mechanism of the garden loop is in and its rate is zero*: bites in
  single digits per 120,000 frames, and no pip has become a plant in any
  run this round or last. The next lever is where windfall lands and why a
  pip never germinates, not another mechanism.
- *A colony guard moved through shared procedural terrain while every
  foraging counter improved*, and the instrument its comment named could
  not see it. Fewer long trips can be a thriving colony.
- *The "81 flowers" anchor was the thicket bed.*

**Environment, learned this round.** A poke's fire response names where it
landed (`cse_<lane id>`); a trigger's prompt cannot be edited once bound, so
each message is a new trigger, deleted after it fires. **A sub-agent that
ends its turn to wait for a build never resumes** — one hung two hours with
861 lines uncommitted and was salvaged by committing its worktree and
killing it; a `SendMessage` resumes a finished lane with its context intact,
which is how #317's CI red was fixed for 85k tokens instead of a fresh lane.
The container suspends while the coordinator idles, so keep a check-in
armed. **`labforage`'s SUMMARY line is contested by every lane**: five
textual conflicts across three PRs in one evening, all on the format string
and its argument tail; resolve by keeping `main`'s fields and appending the
branch's, then `cargo check --all-targets` against the main checkout's
warm target directory (`CARGO_TARGET_DIR`, 18 s) before pushing. A push
that merges `main` into a PR branch cancels the branch's running CI, so the
pre-merge head may never have been tested — bracket locally. A task output
file's mtime is not its last write; read the last timestamp in it.

**Open at close.** The bodies' ruling across #303 → #311 → #315 + #316 (land
after greening the two reds, or park and keep the two-cell ant); the flitter
(P2) before the scent plane (P1b), since scent buys a ground animal
nothing; the garden loop's rate; plant counts lower under rebloom; and four
cards unanswered — the seed deliveries (`…4ec0c3`), the founded bodies
(`…7f9bac`), the re-bloom A/B (`…3257b2`), and the long ant in a tunnel
(`…9f00a9`). Ideas the owner asked for and did not yet rule on: sugar water
as the first player verb, a divider that splits the bed into a paired
experiment, evolving the flitter's brain in the selection arena instead of
writing it, a lamp schedule, a predator with an alarm plane, death that
feeds the bed, a follow-camera on one ant.
