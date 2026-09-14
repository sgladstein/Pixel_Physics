# A floor of plants is a floor a colony can stand on

**What it does.** Standing in a thicket, pressing the found-a-colony key now
puts ants down. It used to look at the soil, find a root or a leaf lying on
it, and decline — so a wood was the one place in the world you could not
start a colony, which is backwards for a game about growing things.

**Where it sits.** This is the ground half of making the held world's central
verb work anywhere you can stand. It is not the whole of it: the second half
is filed as §Z21 below and is somebody else's to take, because it is not in
this file and not in this game's rules.

Owner playtest, 2026-09-14: *"It should be easier to found a colony while
standing in a thicket of plants."*

---

## The mechanism

`creature::colony_ant_site` ended on `world.is_empty(cx, sy - 1)` against the
**mineral** surface. A forest floor almost never has air directly over its
soil — it has root, stem base, grass blade, moss or fallen leaf — so every
station in a wood was declined.

The rule now rises through **contiguous** plant tissue to the first genuinely
free cell, and returns the ant's *footing*, which may be a plant cell. The
`Liquid` refusal (`open-bugs-handoff.md` §R2) is untouched and still fires on
its own line; nothing steps through rock, a creature, or water.

**It was already inconsistent with the walk, which is what makes this a
repair rather than a preference.** `step_chain`'s support test counts
`MaterialKind::Plant` as something to stand on, and
`landing_is_placeable_through_tissue` lets a body step into non-woody tissue
outright. An ant that could not be *founded* on a leaf could walk onto that
same leaf one tick later. Founding was the last rule in the creature line
treating a plant as a wall.

`THICKET_CLIMB = 16` is a statement about what a floor is, not a work bound —
set from the measured distribution (p90 14, max 35; the tail is trunks, and a
founder at the top of a trunk is not at the gnome's feet).
`PIXEL_PHYSICS_THICKET_CLIMB=off` is the paired arm, in the same binary.

## Reproduced before fixing

`examples/thicket_probe` (new) buckets every column of a colony's footprint
and breaks the interesting bucket out **by the material standing there**,
because "plant tissue", "litter", "spoil" and "a puddle" are four different
findings and a refusal count cannot tell them apart.

Druid world, at the gnome's own stand, 221 columns: **86 sites on a grown
start against 176 on a bare one**, and **every one of the 135 refusals is a
plant cell** — 53 wood, 51 leaf, 12 grassblade, 10 rootwood, 9 grassroot. Not
spoil, not litter, not a powder that fell, not water. The traced cause holds.

## The paired numbers

One binary, two arms, the semantic rule held fixed and nothing else moved.

**Druid, partly grown world (3,394 organisms, slots free), 18 stands:**

| | off | on |
|---|---|---|
| stations offered | 60 | **134** |
| animals placed | 47 | **79** |
| placed, p50 / p90 | 2 / 8 | **4 / 9** |

**Druid, bare start, 18 stands:** placed **120 → 148**, p50 8 → 10.

**Lab bed (`LabBox` founders=8) grown 6,000 frames, then founded at each of 8
founder columns, 3 seeds — 24 stands:** stations **182 → 269**, placed
**147 → 188**, p50 6 → 8, **0 of 24 stands worse**. Live organism count
identical between arms at the founding moment, so the plant side is untouched
up to that point.

## The lab's existing baselines do not move, and that is measured

`labnest founders=8 seeds=20 frames=9000` is **bit-identical across the arms**
on ants, roofed, packed, digs and buried at all ten sampled frames, on all 20
seeds.

That is the tidy result `CLAUDE.md` warns about, so it got the control rather
than the benefit of the doubt. `thicket_probe lab=8 frames=0` censuses the
colony's band at build time and finds **not one plant-blocked column** over
three seeds: `LabBox::build` sows at `ground_y - 2` — two rows up, in the air
— and founds the colony in the same breath, so the cell over the soil is free
everywhere. **The lab was not unaffected, it was not yet exposed**, and those
two are identical in every counter `labnest` prints. Its runtime founding
verb, reaching a grown bed, is the exposure, and that is the table above.

## §Z21 — the finding that overturned the brief

The brief traced the symptom *"pressing `C` places nobody on a grown or dead
start"* to this line. **It places two, not zero, and this change does not move
that number.** Stations went 31 → 63 on that world and placed the same 2
animals.

`Druid::new` grows **4,093 organisms** against a hard ceiling of **4,095**
(`Cell::organism_id` gives 12 bits to the slot index). Over nine separated
stands: **4,095 of 4,095 live, 26 births refused by `push_organism`, 8 of 9
stands placing nobody.** `Start::Dead` is identical — a senescent plant still
holds its slot, and in a held world nothing rots. The game then reports
*"nothing founded - no ground here"*, which is a confident, specific and
wrong cause.

Not inferred: `World::organisms_refused` is the engine's own counter from the
far side of the call. Positive control, same binary on `Start::Bare` (378
organisms): **0 refusals, 148 placed over 18 stands.**

Filed as **§Z21** in `Reports/open-bugs-handoff.md`, letter from
`bugindex.py --branches` over 72 refs. **Not fixed here** — every candidate is
druid-side or worldgen-side, and this lane does not own those files.

## Tried and rejected

**Extending the same tissue-awareness to `founding_spine_walk`**, which is the
second gate (55 of the 134 druid stations still die there). Not built:
`place_creature` writes body cells with `World::set` and has none of
`relocate_chain`'s parted-tissue bookkeeping, so it would silently erase plant
cells rather than part them. `dead-ends.md`'s `is_partable`-on-woody-tissue
entry records what that costs — ownership resolves through the grid, the cell
stops counting as an anchor, and
`lab::tests::copies_carry_what_was_planted_and_still_diverge` finishes at
`plant_cells 0` in all three copies. *Condition its rejection depends on:*
founding gains a parted-tissue ledger. Not filed in `dead-ends.md` because it
was reasoned from an existing entry rather than built and reverted; it is in
the commit message and `Reports/lanes/thicket-founding.md`.

## Guards

Four new tests in `creature.rs`, **watched going red** through the documented
arm rather than through an edit: under `PIXEL_PHYSICS_THICKET_CLIMB=off`,
`a_mat_of_plants_is_a_floor_a_colony_can_stand_on` and
`the_climb_stops_before_it_becomes_a_tree` both fail. The bound guard is
stated as a transition against `thicket_climb()` rather than against a
literal, so it cannot go green the day the bound moves.

`the_climb_refuses_everything_that_is_not_a_plant` covers the case a naive
"first free cell above" would get wrong: soil, a leaf, a slab of stone, then
air. There *is* free space up there; it is on the far side of a wall.

## Gates

`cargo clippy --all-targets --release --locked -- -D warnings`, full `cargo
test --release`, `scripts/acceptance.sh`, `scripts/docscheck.sh` — green.
`deadendindex.py --touching`: 0 hits (silence is not evidence; the manual pass
is the section above).

Full account, including the reproduction and everything the brief got wrong:
`Reports/lanes/thicket-founding.md`.
