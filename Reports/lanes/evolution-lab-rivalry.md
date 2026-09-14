# Lane C — why colonies do not fight

*Evolution lab round 35. Coordinator: `Reports/lanes/evolution-lab-coordinator.md`.
The owner's ask: "review/explore why we don't have different colonies fighting
or eating each other." A review-and-explore brief — the deliverable is the
explanation, the evidence and what closing each gap would look like, not a bed
with a war switched on in it. Written 2026-09-14.*

## What this lane found, in one line

**Nothing is missing a fight; what is missing is a stranger.** One species
field — `CreatureDef::scent_spread`, shipped at `0` — puts every colony of a
kind at one point in scent space, so `creature::nearest_foe` returns `None`
for every animal in the bed and no eye and no brain wire can change that.
Move that one number and colonies kill each other with **nothing else
touched**.

## Deliverables

| | |
|---|---|
| Report | [`Reports/why-colonies-do-not-fight-2026-09-14.md`](../why-colonies-do-not-fight-2026-09-14.md) |
| Harness | `examples/rivalry.rs` — the chain measured link by link on one run, with both controls in `control=selftest` |
| Card | see **Cards** below |
| Bug filed | `Reports/open-bugs-handoff.md` §Z23 — `nearest_foe` counts a plant as a foe |
| PR | [#416](https://github.com/sgladstein/Pixel_Physics/pull/416), head `f3d2ef56`. **Coordinator owns the merge.** |

**No shipped species file was edited and no default changed.** Every arm in
the report is a run-time override in the harness; the bed a player opens is
byte-identical to the one before this lane ran.

## Three corrections to the standing account

1. **`creature::is_living_kin` does not consult the colony label.** The lane
   brief describes kin as needing "the same colony *and* scent tolerance";
   it is **species and scent only**. (The reports have this right —
   `Reports/creature-signature-and-castes-2026-09-06.md` says "with the
   colony label nowhere in it", and the colony clause in its §0 table
   describes the *retired* switch. The correction is to the brief, not to
   them.) The label enters exactly once, at founding, through
   `colony_scent_offset` — which returns the zero vector whenever
   `scent_spread <= 0`, and that is the whole of why the shipped bed is one
   family.
2. **`Reports/held-world-game-concept-2026-09-13.md` §10a names the wrong
   cause.** Its two gaps (the blind ant, the retaliation-only `Alarm` wire)
   are both real and **neither is binding**: closed together, attacks do not
   move, because there is still nobody an ant is allowed to bite.
3. **Initiation already exists, and it is the mouth.** A stranger is food
   (`adjacent_food`'s kin filter does not exclude it, the ant's
   `TRAIT_GUT_BIAS` is a neutral `0.0`), the swallow calls `cry_alarm`, and
   `ant.ron`'s shipped `(Alarm, Attack, 2.0)` does the rest. **Predation is
   the ignition the combat layer was said to lack.**

## Two traps this lane walked into, both worth the next session's time

**`labstats`' `rivalry=1` alias is not "two rival colonies".** It sets
`spread=1 tolerance=-1`, and a tolerance of `-1` is a radius of **zero — an
exact match only**. With `ant.ron` shipping `scent_drift: 0.15`, a newborn is
a stranger to its own mother, so the alias makes **every ant a stranger to
every other ant**, colonies included. `creature.rs`'s own
`attacking_costs_the_jaw_and_yields_no_food` records measuring 11 attacks
between animals it had never made strangers for exactly this reason and pins
drift off. **Any table run under `rivalry=1` without pinning drift is
measuring speciation-within-a-colony.** The report's arms use `spread=1` at
the ancestral tolerance instead, and split the stranger census into
*between-colony* and *within-colony* so the two cannot be confused again.

**`scent_spread = 1` is a draw, not a setting.** The offsets are uniform per
slot, so the gap between two colonies has to *happen* to clear the tolerance
radius of 1.0. On seed 1 it does not (gap under the radius, `strangers 0%`,
and the run is **byte-identical to the baseline** — the classic "identical
output across a change that must have moved something" tell, here with an
innocent explanation). The harness prints the gap for exactly this reason.

## After the round moved under this lane (2026-09-14, 08:36–09:00Z)

**The bug is §Z23.** The coordinator renumbered it on this branch at
`18b7fa2b` — `claude/absorb-destroys-plants` had filed a different §Z22 an
hour earlier on an unlanded branch, which is precisely the case
`bugindex.py --check` cannot see and `--branches` can. Verified the diff: the
heading, the index row and five cross-references, and **no claim in the
section changed**. Accepted as-is, not re-filed.

**PR #417 (Lane D) landed on `main` and touches this lane's subject; §Z23
survives it whole, and that was measured rather than assumed.** #417 made
`nearest_foe`'s **odds count** animals-only and left the **target rule**
alone, stating the reason in a comment. Two consequences, both now in §Z23:
its new gate is `assessing = victim != 0 && is_animal && contest::enabled()`,
so **a plant is the one target struck with no assessment at all**; and
re-running seed 1 on `origin/main` **with this branch merged into a scratch
worktree and built fresh** gives **475 attacks / 79 cells / 629 deaths,
byte-identical** to the pre-#417 figure — predicted first, because with every
foe a plant the commitment branch takes no RNG draw.

**§Z23's proposed repair was wrong and has been rewritten.** It first said
"test that the target is an animal"; #417 argues against exactly that, and
correctly. The repair moved upstream: gate `cry_alarm`'s two **feeding** call
sites on the victim being an animal, which fixes the vandalism and leaves the
target rule intact.

**One correction back to the coordinator: this lane ships no switch and no
default.** The poke asks what this lane's "ON default" for `scent_spread` is
and says to re-derive it. There is nothing to re-derive — the PR changes no
species file, no default and no file under `src/`, and the report's
recommendation is to expose the dial rather than choose a value, default-off.
Lane D's §8.3 sweep is still useful and is now cited in the report: the dial
is a **threshold, not a slope**, saturating by **2**, so a dial whose top is 1
would ship a mechanism a third of beds never show.

**The two lanes' stranger figures differ by denominator, not by fact**, and
the report now says so: Lane D's 0.093 is over *all* ordered ant pairs
(kin-heavy); this lane's `between%` is over *cross-colony* pairs only, which
is why it is 100% or 0% and never in between. Pooled, **7 of 10 seeds
separate at `spread=1`** (4 of 6 here, 3 of 4 there).

**Not merged `main` in.** `git merge-tree` reports **0 conflicts** at 12
behind x 6 files = 72, well under the 300 bar, and the coordinator owns the
merge order this round.

## Cards

**Card `20260914T053420770Z-6a7aa9`** — *Two colonies that are strangers to
each other*, board `creatures`. An A/B of **frame sequences** (175 frames
each, 960x300, the surface band where the two colonies meet), un-blinded and
labelled, because the question is "can you see it" rather than "which is
better" and blinding would remove the very thing the owner needs to answer.
Same bed, same seed, same 4,200 frames, same patch of ground; the only
difference is `scent_spread`. `meta` carries the number a picture cannot give
— **0 killings between colonies against 9** — per the queue's house rule.

**A sequence rather than only a GIF**, on the review skill's own head-to-head
finding that a posted sequence played where a valid GIF did not; the GIF was
written too and is in the scratch directory if it is ever wanted.

**`20260914T053354967Z-98ac84` is a duplicate of it** — the same card posted
twice by mistake, identical in every field. Either may be answered; the other
can be ignored.


## Files this lane owns

`Reports/why-colonies-do-not-fight-2026-09-14.md`, this note, its line in
`Reports/README.md`, `examples/rivalry.rs`, and the §Z23 section of
`Reports/open-bugs-handoff.md`. **`src/sim/creature.rs`, `src/render.rs`,
`src/bin/lab.rs` and `src/lab/ui.rs` were not touched** — no change to
`creature.rs` turned out to be needed, so there is nothing for the
coordinator to sequence against Lane B.

## Head SHA

**`afb6bfe1dd80253f9cd486c180e5da2abd71acb6`** on `claude/evolution-lab-rivalry`
— `main` moved 7 commits under this branch while it was measuring, so that
commit merges it in and **CI is 9 of 9 green on it**. The one conflict was
`Reports/open-bugs-handoff.md`, the contested file the rules warn about: the
generated index block was taken whole from `main` and regenerated with
`scripts/bugindex.py` rather than hand-merged, and the two bug sections
appended to the same end of the file — **§Z21 from `main` and §Z23 from here**
— were both kept. `docscheck` was re-run after the merge, unconditionally.

**The letter changed after that merge, and not because of it.** This lane
filed §Z22 at 05:47; `claude/absorb-destroys-plants` had already filed a
different §Z22 at 04:48 on an unlanded branch, which is exactly the case
`bugindex.py --check` cannot see and `--branches` can. First filed wins, so
this lane's section is **§Z23** and the earlier filing keeps §Z22. The
renumber was made by the round coordinator rather than by this lane, because
the lane was idle and the round was waiting on it; it is a rename of the
heading, the index row and five cross-references, and no claim in the section
changed.

Gates re-run **after** the merge: clippy clean; `cargo test --release --lib`
**1,756 passed / 0 failed / 85 ignored**; `--test worldgen --test determinism`
**47 passed / 0 failed**; `docscheck` clean.

The pre-merge SHA was `55efeb17`, whose gates were: clippy clean; `cargo test --release --lib` **1,750 passed / 0 failed / 85 ignored**;
`--test worldgen --test determinism` **47 passed / 0 failed**; `docscheck`
clean; `deadendindex --touching` 0 hits.

**The 54-run sweep was verified byte-identical across two rebuilds** (the
card-capture additions, and the clippy `is_multiple_of` fix) by re-running
`base seed=1` on each new binary and matching every column — this repo's own
stale-binary rule, met from the side where the binary is new and the table is
old.
