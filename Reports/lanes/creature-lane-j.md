# Lane J — the sight sense (E15)

**Branch `claude/creature-lane-j-sight-sense`. Built the sense; it ships.**

## Base — read this first, it affects your merge

**Cut from `origin/claude/creature-lane-d-body-extent`, not from `main`.**
The brief made #154 a hard precondition; at cut time
`git show origin/main:assets/species/ant.ron | grep -c idle_cost_per_cell`
read **0** and #154's CI had been *cancelled* (a concurrency group killed the
run two seconds in), so it was not landing within the wait. Per the brief's
fallback I measured the conflict — `git merge-tree --write-tree origin/main
origin/claude/creature-lane-d-body-extent` exits **0 with no conflict** — and
cut from lane D instead. `git merge origin/main` was then already up to date,
since lane D's head already carried `main`.

**So this branch carries #154's diff.** If #154 lands first, this merges
cleanly on top; if it does not, review the two together.

## What landed

Two brain inputs and no new mechanic. `PreyNear` (`1 - d/range`) and
`PreyBearing` (signed turn-to-target, positive right, ±1 for directly
behind), written by a 16-ray fan cast from one cell above the head, marched
to the first `Solid`/`Powder` cell or to `sight_range`. `beetle.ron` authors
`sight_range: 64` and `(PreyBearing, Turn, -2.5)`. **Nothing else in the
world has eyes** — the opt-in is `CreatureDef::sight_range`, tested at the
dispatch site that already holds the def.

Five counters on `CreatureStats`, near side to far side: `sight_casts`,
`sightings`, `sight_dist_sum`, `sight_facing`, `sight_approaches`, plus
`sight_cells_read` for the cost.

Full write-up: **`Reports/creature-sight-sense-2026-08-30.md`**.

## Three things another lane needs from this

1. **`BRAIN_INPUTS` is 18 and the genome manifest moved to `1_520_499_525`.**
   Lawful — two columns of a 64-wide reserve that were already zero,
   `GENOME_LEN` unchanged at 12,352, no existing weight renumbered. This is
   the append S2 reserved the dimensions for. If your branch also appends an
   input, we both move the manifest and the merge needs one literal chosen,
   not both kept.

2. **`creature::sense` now returns a triple**, `([f32; BRAIN_INPUTS],
   Option<Sighting>, u64)`. If you touch that function, that is where the
   conflict is.

3. **A harness that overrides a species' *wiring* must call
   `set_genome(genome_from_wiring(..))` beside `set_creature`.** The genome
   is compiled once at load and `place_creature` stamps that, so overriding
   `instincts` alone changes nothing a creature thinks with — my first sweep
   came back bit-identical across all eight settings because of it.
   Overriding a non-wiring field (`sight_range`, `idle_cost_per_cell`) needs
   only `set_creature`. In `dead-ends.md`.

## Filed against another lane's area

**`open-bugs-handoff.md` §R4 — `BrainOutput::Turn` is nearly inert for a
surface walker on level ground.** Both turning candidates fail on flat
ground, one on passability (the step down is inside the floor) and one on
foothold (the step up is over air), so the heading changes only by random
tumble. Reproduced with byte-identical `moves 898 blocked 28 falls 167`
between an eyed and a blind beetle while the eye reported 139 sightings of
195 casts. **This is not a vision bug and it is bigger than vision** — the
ant's own trail-following routes through `Turn` too, so any authored or
evolved turn weight is worth less than it reads wherever the world is flat,
and `scene=flat` is the structural test bed. Whoever owns movement should
take it.

## Open, and deliberately left to the owner

`(PreyNear, Persist, w)` — releasing straight-ahead persistence — is the
strongest lever in the sweep (mean sighted range 12.5 → 9.1, prey caught 323
→ 355 over 8 seeds) and **is not shipped**, because in a sealed corridor it
took outright kills from 6/8 to 4/8 at an identical 8/8 ant cells taken.
Small samples on both sides of a question about how an animal moves; posted
as a review card. `predation_probe`'s `release=` is the knob;
`dead-ends.md` has the entry with its re-test condition.

**Review card `20260830T075421622Z-76444e`, board `creatures`.** Note: it is
**posted twice** — a tool invocation I re-ran to read its id posted a second
copy, and `review.py` has no withdraw. The earlier duplicate is
`20260830T075404107Z-fb5f12`; they are identical and either can be answered.

## Gates

`cargo test --lib` **1,106 passed / 0 failed / 54 ignored** · `cargo +1.98.0
clippy --all-targets -- -D warnings` **clean** · `cargo run --release
--example ascii` **31 scenes, 0 skipped**, frame cost with 143 live
organisms worst 37.681 ms / mean 4.319 ms over 12,000 frames ·
`scripts/acceptance.sh` **all cases met their expectations** ·
`scripts/docscheck.sh` **clean** · `scripts/bugindex.py --check` **index
current, identifiers unique**.

## Head SHA

    a1e6e8f  Merge main into lane J: #154's per-cell metabolism landed, plus the colony page
    b0df9bd  A beetle can see: E15's sight sense, built to its pre-flight

Merged `origin/main` at `c4bb564` (#157). Post-merge gates re-run on the
merged tree: `cargo test --lib` **1,122 passed / 0 failed / 54 ignored**,
clippy 1.98.0 clean, `ascii` 31 scenes 0 skipped (worst 40.930 ms / mean
4.470 ms with 143 live organisms), acceptance all cases, `docscheck` clean,
`bugindex --check` current and unique.

The four merge conflicts were all in shared registers and generated blocks
and none needed a judgement call: `dead-ends.md` keep-both, the bug
register's *generated* index taken from `main` and regenerated with
`bugindex.py`, `README.md`'s TOC and line-number tables taken from `main`
and regenerated with `readmetoc.py` (which is what the block's own note
says to do), and `wiki/ants.md`'s freshness paragraph written to carry both
sides' entries.

**One thing the merge changed that is worth knowing**: `main` re-titled §R3
from *"No creature body above two cells leaves a living colony"* to *"A
creature chain above two cells overwrites its own head"* — so the thing the
brief warned me about has since been root-caused.
