# Lane D — body extent, cell-type shading, and pricing the body

**Branch:** `claude/creature-lane-d-body-extent`.
**Cut from `claude/creature-lane-a-birth-grant` (#142), not from `main`** —
see *The precondition* below; this is the one deviation from the brief and it
was taken to satisfy the brief's own stated requirement rather than to skirt
it.
**Head SHA:** `e6f380a6bdf2` — the docs commit; the code is `86b9f2a`, and `516cbd6` is the merge of `origin/main` onto this lane.
**Cost fork:** built the package **and** wrote the finding. Both, because the
finding is that the lever the package was built to open is blocked by
something neither the package nor the appearance report had looked at.

## The one-line version

The body is priced per cell at last — **E10's premise that "per-cell
metabolic cost already prices a longer body" was false, measured at a
difference of exactly zero** — and `ShadeRule::Countershade` ships off by
default. But **no chain above two cells leaves a living colony**, at the old
flat bill as much as the new one and on a flat slab as much as on the world,
so the blind A/B the brief asked for is **held** rather than posted. Full
account: `Reports/creature-body-extent-2026-08-30.md`; the collapse is filed
as `open-bugs-handoff.md` §R3.

## The precondition, and what I did about it

The brief said: do not cut from a `main` that lacks #142; check
`git show origin/main:assets/species/ant.ron | grep -c birth_grant`.

**It read 0 for the whole session and still does.** #142 is open, its
`mergeable_state` is `dirty`, and it had been idle ~25 minutes when I
started. Waiting indefinitely was not obviously better than the alternative,
so I measured the conflict: `git merge-tree` puts it at **one hunk in
`Reports/dead-ends.md`** — the append-only register, keep-both — and *not* in
`creature.rs` at all.

So I cut from #142's head and merged `origin/main` in, resolving that one doc
conflict keep-both (lane A's two creature entries, main's three plant ones;
independent appends, neither superseding the other). That gives me exactly
what the precondition was protecting: `start_energy: 200`, `body_energy: 480`,
`birth_grant` as slot 1, `CREATURE_TRAITS` 2 — and no conflict in the repo's
most contested file. #142 touches `creature.rs`, `organism.rs` and
`assets/species/*.ron`, so cutting from `main` would have *guaranteed* the
conflict the precondition exists to avoid.

**Consequence for whoever lands this: my PR is stacked on #142 and carries its
commits.** It should be based on `claude/creature-lane-a-birth-grant` and will
retarget itself to `main` when #142 merges. If #142 is reworked rather than
merged, this branch needs rebasing.

## For the coordinator, in order of what changes a decision

1. **E10 needs amending, and I have not amended it** — it is an owner
   decision record. Its stated cost basis is wrong in one clause: *"per-cell
   metabolic cost already prices a longer body, so no cost system is owed."*
   Nothing read `chain.len()`. Injecting the old behaviour back into this
   change's own guard reports `0.11155701 -> 0.11155701 (difference 0)` — a
   two-cell and a six-cell ant paid the identical bill. That half is fixed
   now, so the clause is merely *stale* rather than load-bearing, but E10 also
   concludes "heritable body = one integer, zero new mechanics" and that
   conclusion is what item 2 undermines.
2. **The extent lever is blocked, and not by anything this lane changed.**
   `Chain(3)`, `(4)`, `(6)`, `(9)` all reach `live 0` inside 12,000 frames.
   The paired control is the important part: at the **pre-change flat bill**,
   `Chain(6)` gives peak 29 / live 0 and `Chain(3)` peak 34 / live 0 — the
   same peak populations as the priced arms, so placement is matched and both
   die. It reproduces on the hand-built flat slab. §R3 has the table.
3. **So I held the blind A/B, deliberately, and this is the call I most want
   checked.** The brief asked for a runtime selector plus a blind A/B so the
   owner picks the shipped body. Posting that today would ask the owner to
   choose between one body that lives and four that go extinct — an answer to
   a question nobody meant to ask, and the review queue's value depends on the
   cards being honest. If you disagree, the card is cheap to post: the
   appearance arms still render fine, because a 600-frame render never reaches
   the horizon where they die.
4. **The keyboard is full, for real.** `main.rs` says so itself at the
   `Comma` binding — *"every letter and digit is already bound and F9-F12 are
   owned by macOS."* The home for a body selector is the tunables panel,
   which is where `TunableGroup::World` went for exactly this reason and says
   so in its own doc. Not built; §6 of the report has it as owed work behind
   §R3.
5. **`start_energy` is the term I left alone and flagged.** Burn is per cell
   and the tank is flat, so an *n*-cell animal's horizon is `1/n`. That is
   probably wrong as biology. I did not fix it because #142 bound the bank
   ceiling to `start_energy` hours earlier and re-opening that the same night
   would have made two reallocations inseparable. Named in the constants
   table with a verdict, and in `dead-ends.md` with the Kleiber alternative.

6. **One card is on the queue**, blind, board `creatures`, id
   `20260830T063631048Z-ae976a`: random shade against countershade on a
   nine-cell body, arms identical but for the paint. It is **not** the
   body-size card the brief asked for, for the reason in item 3. Collect it
   with `review.py inbox`.

## What another lane should know before touching these files

- **`idle_cost` and `move_cost` no longer exist.** They are
  `idle_cost_per_cell` / `move_cost_per_cell` and are multiplied by the
  **live chain length**. The rename is deliberate: re-derived numbers alone
  would leave species files reading `0.05` where they read `0.10` with
  nothing on the page to say they are the same animal.
- **`creature_probe` gained `body=`, `idle=`, `move=`** and echoes its own
  metabolism line. `idle=`/`move=` are what make it possible to run a body
  against the bill it used to pay instead of confounding the two — that is
  the control that overturned this lane's working assumption, so please keep
  them.
- **`§13k`'s `move_cost` mapping in `creature_space.rs` is in the old
  units.** Double its numbers to read them against the new field. Noted at
  the line.
- **`ShadeRule` defaults to `Random`**, and a guard asserts `ant.ron` still
  carries it. Changing the shipped ant's appearance should be a deliberate
  act with a verdict behind it, per E10.

## Gates

`cargo test --lib` **1095 passed / 0 failed / 54 ignored** ·
`cargo +1.98.0 clippy --all-targets -- -D warnings` **clean** ·
`ascii` **31 scenes, 0 skipped, zero non-timing differences against a
baseline built from this branch's own merge commit in the same session** ·
structural acceptance **all cases** · `docscheck` **clean** ·
`bugindex --check` **index current, identifiers unique**.

The `ascii` identity result is over a live path, not an untouched one: the
colony scene inside it runs 27 creatures, 11,102 moves, 57 eats and 4 deaths.

`wiki/ants.md` and its freshness note are updated in the same change; the
report is indexed in `Reports/README.md`; three `dead-ends.md` entries and one
`open-bugs-handoff.md` section (§R3) land with it.
