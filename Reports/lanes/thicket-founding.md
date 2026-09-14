# Lane C — founding a colony while standing in a thicket

## 2026-09-14 → coordinator

**Branch `claude/thicket-founding`, cut from `main` at `32f465c6`.** Head SHA
at the bottom.

**The short version.** The traced line was right and it is not the whole
answer. `colony_ant_site`'s `is_empty(cx, sy - 1)` was refusing a forest
floor exactly as you said, and it is fixed: a mat of plants is now a floor an
ant can stand on. But **the headline symptom you handed me — "places nobody
on a grown or dead start" — is not that bug.** It is the organism slot
ceiling, and no change to the ground rule can move it. Both are real, both
are measured below, and the second one is filed as **§Z21** because it is not
mine to fix.

---

## 1. The reproduction, and what it actually showed

`examples/thicket_probe` (new, mine). It censuses every column of a colony's
footprint into one of four buckets and breaks the interesting one out **by
the material standing there**, because "plant tissue", "litter", "spoil" and
"a puddle" are four different findings and a refusal count cannot tell them
apart.

Druid world, shipped 2560x960, at the gnome's own stand, 221 columns:

| | grown start | bare start |
|---|---|---|
| sites | **86** | **176** |
| good floor, cell above occupied | 135 | 45 |

**Every one of the 135 refusals is a plant cell** — 53 wood, 51 leaf, 12
grassblade, 10 rootwood, 9 grassroot. Not spoil, not litter, not a powder
that fell, not water. So: your trace holds, and the scene contains the
situation you thought it did. I checked because `CLAUDE.md` says to and
because the alternative readings were cheap to rule out; they were all zero.

**One correction to the brief.** You wrote *"places nobody on a grown or dead
one"*. It places **two**, not zero, and that difference turned out to be the
whole thread — see §3.

Climb depths over those 135 columns: **min 2, p50 5, p90 14, max 35.** The
tail is trunks. That distribution is what set the bound.

## 2. Which object the rule evaluates

Asked before changing anything, since you flagged it.

- `colony_surface` evaluates a **column** → the mineral ground row.
- `colony_ant_site` evaluates a **column** → *the row an ant stands on*. Its
  caller derives the head as `sy - 1`.
- `place_creature` → `founding_spine_walk` evaluates the **whole body** and
  demands `World::is_empty` at the head.

The defect was that `colony_ant_site` conflated "the ground" with "one cell
above the ground is where the ant goes". On bare ground those are the same
row; in a thicket they are not. So the fix changes what the function
*returns* — the footing, which may now be a plant cell — rather than
loosening what it accepts. The quantities it needs are all defined for a
column, and the head it implies is still genuinely free, which is what
`founding_spine_walk` needs. I assert that in the guard rather than assuming
it.

**It was already inconsistent with the walk, and that is what makes it a
repair rather than a preference.** `step_chain`'s support test counts
`MaterialKind::Plant` as something to stand on; `landing_is_placeable_
through_tissue` lets a body step into non-woody tissue outright. An ant that
could not be *founded* on a leaf could walk onto that leaf one tick later.
Founding was the last rule in the creature line treating a plant as a wall.

`THICKET_CLIMB = 16` sits just past p90 and a long way short of the tail.
`PIXEL_PHYSICS_THICKET_CLIMB=off` is the paired arm, in the same binary.

## 3. The finding that overturns the brief's premise — §Z21

Running the paired sweep on the grown world gave the tidy result `CLAUDE.md`
warns about: **stations 31 → 63, animals placed 2 → 2.** The lever was
connected, doubled the ground offered, and moved nothing a player sees.

The reason is downstream and has nothing to do with plants. `Druid::new`
grows **4,093 organisms** against a ceiling of **4,095**
(`Cell::organism_id` gives 12 bits to the slot index). Over nine separated
stands: **4,095 of 4,095 live, 26 births refused by `push_organism`, 2
animals placed, 8 of 9 stands placing nobody.** `Start::Dead` is identical —
a senescent plant still holds its slot, and in a held world nothing rots.

I did not infer this. `World::organisms_refused` is the engine's own counter
from the far side of the call; the probe reads it before and after each
stand. Positive control: the same binary on `Start::Bare` (378 organisms),
**0 refusals, 148 placed over 18 stands.**

**The game names the wrong cause out loud.** `Druid::found_colony` prints
*"nothing founded - no ground here"*. The ground is fine.

Filed as **§Z21** in `Reports/open-bugs-handoff.md` (letter from
`bugindex.py --branches`, which answered §Z21 over 72 refs; `--check` would
have been the confident wrong answer). **I did not fix it** — the candidates
are all druid-side or worldgen-side and you told me not to reach into
`src/druid/*`. Cheapest-to-most-principled, none measured:

1. reserve a slot band for animals;
2. free a senescent plant's slot on `Start::Dead` (the bodies must still
   *render*, so this is not a deletion);
3. thin the grow phase's seed scatter so it makes fewer, larger organisms.

**Route this.** It blocks the held world's central verb, and it silently
refuses *every* birth, so budding and reproduction are gone too — a colony
that cannot grow looks exactly like a colony that will not.

## 4. The paired numbers, on worlds that are not at the ceiling

One binary, two arms, semantic rule held fixed.

**Druid, partly grown (`grow=2500`, 3,394 organisms, slots free), 18 stands
at 128 spacing:**

| | off | on |
|---|---|---|
| stations offered (total) | 60 | **134** |
| animals placed (total) | 47 | **79** |
| placed, p50 / p90 | 2 / 8 | **4 / 9** |
| stands placing nobody | 5 | **4** |

**Druid, bare start, 18 stands:** placed **120 → 148**, p50 8 → 10. Slot
refusals 0 in both arms.

**Lab bed (`LabBox`, founders=8), grown 6,000 frames, then founded at each of
8 founder columns, 3 seeds — 24 stands:**

| | off | on |
|---|---|---|
| stations offered (total) | 182 | **269** |
| animals placed (total) | 147 | **188** |
| placed, min / p10 / p50 / p90 / max | 3 / 4 / 6 / 8 / 11 | **5 / 6 / 8 / 10 / 11** |
| stands worse than the other arm | — | **0 of 24** |

24 of 24 stands equal or better, none worse, and the bed's live organism
count is identical between arms at the founding moment (81 / 111 / 81), so
the plant side is untouched up to that point.

## 5. The lab's shipped baselines do not move, and here is why that is not luck

`labnest founders=8 seeds=20 frames=9000` is **bit-identical across the arms
on ants, roofed, packed, digs and buried at all ten sampled frames, on every
one of 20 seeds.**

That is the tidy result again, so I ran the control rather than believing it.
`thicket_probe lab=8 frames=0` censuses the colony's own band at build time
and finds **not one plant-blocked column** over three seeds. The reason is in
the scene: `LabBox::build` sows at `ground_y - 2` — two rows up, in the air —
and founds the colony in the same breath, so the cell over the soil is free
everywhere. **The lab was not unaffected, it was not yet exposed**, and those
are identical in every counter `labnest` prints.

The lab's real exposure is its *runtime* founding verb reaching a bed that
has been growing, which is what §4's lab table measures. So: existing lab
baselines stand; a lab session founding into a grown bed gets more founders
than it used to, and should say which side of 2026-09-14 its numbers are on.

## 6. Tried and rejected — for `dead-ends.md` if you want it there

**Extending the same tissue-awareness to `founding_spine_walk`.** It is the
second gate and a big one: 55 of the 134 druid stations still die there, and
the same inconsistency argument applies to it (the walk parts tissue, the
founding walk does not).

**Not built, and the rejection is not mine — it is `dead-ends.md` entry at
line 990.** `place_creature` writes body cells with `World::set` and has
**none** of `relocate_chain`'s parted-tissue bookkeeping, so a tissue-aware
spine walk would silently erase plant cells rather than part them. That entry
records what happens when a non-plant cell lands where a plant's own cell
was: ownership is resolved through the grid, the cell stops counting as an
anchor, and `lab::tests::copies_carry_what_was_planted_and_still_diverge`
finishes at `plant_cells 0` in all three copies. *Condition its rejection
depends on:* founding gains a parted-tissue ledger (or `organism::Crossing`
is made reachable from placement). Until then this is not a one-line
extension, it is a plant-killer.

I did **not** file this in `dead-ends.md`: I reasoned it from code and an
existing entry rather than building and reverting it, and that register is
for what was tried. It is here and in the commit message instead.

## 7. What I touched

- `src/sim/creature.rs` — mine, per the split.
- `examples/thicket_probe.rs` — new, mine.
- `Reports/instruments.md` — **one row, not on my list.** `docscheck.sh`
  check 5 fails on an `examples/` binary with no row, so the alternative was
  landing a red gate. Same call Lane C made on 2026-08-30.
- `Reports/open-bugs-handoff.md` — §Z21 appended, index regenerated. This is
  the most contested file in the repo (118 landings); I grepped it first and
  took the letter from `--branches`.
- This note.

Nothing under `src/druid/*`, `src/bin/druid.rs` or `src/lab/stats.rs`.

**No `wiki/` page needs updating.** `wiki/ants.md` owns `creature.rs` and
describes what ants do, not where a founding key will accept ground; I read
it and there is no sentence this change makes false. If you disagree, the
sentence to add would go in whatever page owns the held world's verbs, and
there is no such page yet.

## 8. Gates

`cargo clippy --all-targets --release --locked -- -D warnings`, full `cargo
test --release`, `scripts/acceptance.sh`, `scripts/docscheck.sh` — all green;
see the PR body for the run. `deadendindex.py --touching`: 0 hits, and
silence is not evidence, so §6 above is the manual pass.

**The guards were watched going red.** `PIXEL_PHYSICS_THICKET_CLIMB=off`
restores the pre-change rule exactly, and under it
`a_mat_of_plants_is_a_floor_a_colony_can_stand_on` and
`the_climb_stops_before_it_becomes_a_tree` both fail. That is the fault put
back through the documented arm rather than through an edit, so it is
repeatable by anyone reading this.
