# Lane A — birth grant, and the horizon cut

**Branch:** `claude/creature-lane-a-birth-grant`, cut from `origin/main` at
`e7b72e7`.
**Head SHA:** `5e368a64d76e` — the merge of `origin/main` (48 commits,
zero conflicts) onto this lane. The two content commits are `afc41b1` (the
package) and `247bc8b` (the `deaths` correction and the lane note); everything
after is the merge.
**Cost fork:** built the package **and** wrote the finding. Both, because the
finding is that the package cannot reach the objective it was scoped against.

## The one-line version

`birth_grant` and E14 are built, all five gates are green, and **they do not
make the shipped ant breed — no setting of either can.** The binding term is
the 960-point body stamp, which neither lever touches, and cutting
`start_energy` makes the ratio *worse*. Full arithmetic, four controls and
the three routes that would close it:
`Reports/creature-birth-grant-2026-08-30.md`.

## For the coordinator, in order of what changes a decision

1. **The brief's premise is wrong, and I have corrected the documents that
   carry it.** "Two authorised, unbuilt things close that gap" — they do not.
   `birth_grant` moves the shipped ant's bar 1,860 → 1,040 against a measured
   bank ceiling of 219–567. **At a grant of zero the bar is still 961.** This
   is `creature-reproduction-economics.md` §3.6's own conclusion, carried one
   step further and measured.
2. **`deaths` did not read "0 everywhere" before this change.** E14's
   decision record says it did, the brief repeats it, and I wrote it into a
   commit message before catching it. The uncut ant dies at 36,000 frames and
   keeps dying (0/0/14/34 at 12k/24k/36k/54k). It was caught only because a
   review-card render on the pre-change binary came back with 11 deaths where
   the story said none. **§4a of the report is the correction, and the real
   result is better than the claimed one**: the cut colony takes 16 losses
   inside 12,000 frames and then stops (16/17/20/**22**), so by 54,000 frames
   it has lost *fewer* ants than the uncut one. Anyone quoting "deaths 0
   everywhere" downstream should stop.
3. **Nothing here blocks Lanes B or C.** I touched only
   `src/sim/creature.rs`, `src/sim/organism.rs`, `assets/species/*.ron`,
   `examples/creature_probe.rs`, `wiki/ants.md`, and three `Reports/` files.
   I did **not** touch `brain.rs`, `examples/vision_probe.rs`,
   `examples/larder_probe.rs` or `examples/common/mod.rs`.
4. **One thing needs the owner and is with them.** Starved corpses render at
   the darkest end of the worth ramp — correctly, they are worth least — which
   makes them invisible against dark ants on dark soil. So the one death the
   world can now produce for itself is the one it draws least legibly, which
   fails the ethos bar. Posted as review card `20260830T031945607Z-7e0999`
   (board `creatures`) rather than guessed at, since the fix trades against
   the worth ramp meaning what it says. **Collect that verdict** —
   `python3 scripts/review.py inbox`.

## Interface changes another lane may trip over

- **`CREATURE_TRAITS` is 2, not 1.** Every `traits:` and `trait_variance:`
  tuple in `assets/species/*.ron` gained a second element. If you are
  serialising creature traits (Lane B's branch name suggests you might), the
  array widened and the serde default is `[0.0, 1.0]` — **not** `[0.0; N]`,
  because zero on the new axis is a *half* grant and a plain `Default` would
  silently halve every unauthored species' endowment.
- **`birth_cost(def)` is now the *ancestral* cost.** The per-birth cost is
  `birth_cost_of(def, grant)`; `try_bud` takes `threshold.max(cost + 1.0)` so
  `reproduce_at`'s no-suicide guarantee stays total under a heritable grant.
- **`ant.ron` `start_energy` is 200, not 900.** Any baseline measured on the
  shipped ant before 2026-08-30 does not transfer. `creature_probe` takes
  `start_energy=` in-process, so a comparison against the old economy needs
  no rebuild.
- **`creature_probe` gained `grant=`** (a fraction of `start_energy`, not the
  axis position) and its `economy:` line now echoes the grant, so a log that
  does not name one was written by a binary that did not have it.

## Gates, all run on the head below

`cargo test --lib` **1085 passed / 0 failed / 54 ignored** ·
`cargo +1.98.0 clippy --all-targets -- -D warnings` **clean** ·
`cargo run --release --example ascii` **31 scenes, 0 skipped** ·
`bash scripts/acceptance.sh` **all cases** · `bash scripts/docscheck.sh`
**clean**.

Frame cost at a breeding population: **4.30 ms mean at peak 1,781 ants**
against 3.17 ms at 45, one binary. The worst frame (65.6 ms) is *not* pinned
by any aggregate — mean × frames is 103,296 ms against it — so it is an order
statistic and not worth quoting. Note the byte-identical control scene reads
2.98 ms on this binary and 2.61 ms on the pre-change one: **14% machine
drift on a provably identical simulation**, which is why every frame figure
here was re-measured rather than compared against the morning's numbers.

## Two tests were re-derived, not re-fitted

Both had windows calibrated against an ant that could not starve, and both
went red on the cut. `the_standing_meat_never_exceeds_what_was_put_into_it`
now *searches* for the last sample before any death instead of hardcoding
10,000 frames; `a_lone_grazer_cannot_farm_a_moss_lawn_forever` computes its
horizon as 1.1 idle lifetimes instead of hardcoding 60,000 frames — its
*control* arm had starved, which reported "this scene cannot feed anything"
about a scene that was fine. The grazer guard was then checked for blindness
at its 5x-shorter horizon: moss `food_energy` × 40 takes it red at **21.033**
against the 1.0 pump line.

## PR

Opened by this lane (it has the GitHub tools). **The coordinator owns the
merge.**

---

*All five gates were re-run **after** the merge, on `5e368a64d76e`, not
only on the pre-merge head — `cargo test --lib` 1086 passed / 0 failed,
clippy 1.98.0 clean, ascii 31 scenes 0 skipped, acceptance all cases,
docscheck clean. Main had added `examples/selection_arena.rs`, a creature
example, and `CREATURE_TRAITS` widened in this lane, so a zero-conflict merge
was not evidence of anything on its own.*
