# Lane C — founding a colony while standing in a thicket

## 2026-09-14 → coordinator

**Branch `claude/thicket-founding`, cut from `main` at `32f465c6`, with
`origin/main` merged in at `79c0b639`.** Head SHA at the bottom. Every number
below was re-taken on the merged tree and reproduces.

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

## 8. What the guards cost, which is the part worth reading

Four guards, and **two of them shipped in the first commit unable to fail**.
Both were scene errors, both in the same test, and neither was caught by
review — only by running it.

1. It asked for a material called `rock`. This engine's solid is `stone`;
   there is no `rock.ron`. So the overhang case never ran at all. What caught
   it was `matted_bed`'s `unwrap_or_else(|| panic!("{material} material"))`,
   which names the missing material rather than handing back a bed that has
   quietly lost its subject.
2. Fixed to `stone`, it then failed **for a good reason**: it asserted that a
   slab of stone lying on soil is a refusal, and it is not.
   `colony_surface` stops on the first cell that is *not*
   `Empty | Gas | Plant`, so anything `Solid`, `Powder`, `Liquid` or
   `Creature` over the soil **becomes the surface** rather than standing on
   it. A slab on soil is higher ground and founding on top of it is correct.
   The "slab over a leaf" case in the same test was unreachable for the same
   reason. `CLAUDE.md`'s *a scene that contradicts the code will look like a
   bug in the code*, twice in one function.

The reachable non-plant blocker is a **`Gas`** — passable to the surface
scan, and not `World::is_empty`. Both cases are now built out of smoke, and
the third asserts `colony_surface` still lands on the soil *before* asserting
anything about the climb, so it cannot quietly stop testing the climb.

**Two fault injections, and they are not interchangeable.** Deleting the
`!= MaterialKind::Plant` check so the climb crosses anything non-empty: the
refusal guard FAILS, as it must. `PIXEL_PHYSICS_THICKET_CLIMB=off`: 3 of the
4 guards fail and that one stays **green** — at climb 0 the old rule refuses
everything, so a refusal guard cannot be falsified by an arm that refuses by
construction. Recorded because its green under `off` is not coverage and
should not be read as any.

One near-miss worth passing on: my first attempt to run the four used `\|` as
a filter separator. `cargo test` treats it as a literal substring, matched
nothing, and printed `test result: ok. 0 passed` — a green line meaning *I
ran nothing*. Check the count, not the colour.

## 9. Judge-by-eye

Review card **`20260914T050513804Z-623a4b`** (board `creature`, blind A/B,
`owner_can_see_it: true`, verified present on `origin/review-queue` by
`git show` rather than trusted from the post output). Same world, same seed,
same stand (x=1312, the largest paired difference of the 18), one press of
the key in each arm: **2 animals against 7**, both counts in the card's
`meta` beside the picture.

The question put to the owner is the one I cannot answer: the ants now stand
**on grass and leaf rather than on soil**, which is consistent with the
engine's own walk rules and is exactly the kind of thing that is right in the
rules and reads as floating on screen. If it does, `THICKET_CLIMB` is one
number and pulling it back is cheap.

The first pair of cards I rendered were centred on the gnome and showed a
handsome thicket with the ants entirely out of frame — in a wood he stands
*in the canopy*, forty rows above where they land. `shot=` now aims at the
ground under the stand, and upscales, because the skill's own record says the
stills the owner has been able to judge are 700-950 px across and one card at
190x130 came back as "I see none of the changes in it".

## 10. Gates

All green on `89b434dd`, and re-checked after the `main` merge:

- `cargo test --release` — **1,753** lib + 10 bin + 3 determinism + **44
  worldgen**, 0 failed. `tests/worldgen.rs` had never once run on this branch
  before that: `cargo test` stops at the first failing test binary, and the
  `rock`/`stone` guard was that binary. Two gating files invisible behind one
  bad string — the exact asymmetry `CLAUDE.md` §M records.
- `cargo clippy --all-targets --release --locked -- -D warnings` — clean.
- `scripts/acceptance.sh` — all cases met their expectations.
- `scripts/docscheck.sh` — clean, before and after the merge.
- `deadendindex.py --touching` — 0 hits, and silence is not evidence, so §6
  above is the manual pass.

**`main` merged at `79c0b639`** — nine commits, not the six the coordinator
had checked, and `src/druid/mod.rs` and `src/druid/founding.rs` were among
them. Read rather than assumed: the druid changes are the scent-channel
default, `HeldLook::OneHue` and HUD work. Nothing touches the grow phase, the
life densities, `COLONY_SIZE`, or anything the organism count depends on.
**The headline pair was re-taken on the merged tree and reproduces exactly**
— stations 60 → 134, placed 47 → 79 — because a number taken before a merge
is a number about a different tree. `branchcheck`: 7 ahead, 0 behind, 6
files, well under the 300 bar.

**The guards were watched going red.** `PIXEL_PHYSICS_THICKET_CLIMB=off`
restores the pre-change rule exactly, and under it
`a_mat_of_plants_is_a_floor_a_colony_can_stand_on` and
`the_climb_stops_before_it_becomes_a_tree` both fail. That is the fault put
back through the documented arm rather than through an edit, so it is
repeatable by anyone reading this.

## 11. PR and head

**PR [#414](https://github.com/sgladstein/Pixel_Physics/pull/414)** — already
open when I got there, opened on my behalf at 04:35; I updated the body rather
than opening a second one. I do have `mcp__github__get_me`, so I could have
opened it myself had it not existed.

**Head SHA: `58f2ac1e98925d176b6860e6f57ffa9f0fd7a75f`.**
