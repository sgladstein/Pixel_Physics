<!-- PR body for branch claude/creature-genome-serialisation-lane-b.
     This lane has no GitHub tools; the coordinator opens the PR.
     Suggested title:
       an evolved creature can now be saved and added to the game — the exit E8 asked for, which had no implementation
-->

## What this does

**A creature you have evolved can now be kept.** One live individual is
written back out as an `assets/species/<name>.ron` that the game's existing
loader reads to the same animal — same brain, same gut, same body — so it
can be dropped into `assets/species/`, given one `include_str!` line, and be
a species in the world alongside the ant and the beetle.

## Where it sits

This is the **exit** on the owner's own framing of evolution, and it was the
one part of that framing with nothing behind it. From the E5 card, recorded
as decision **E8**:

> *"We do want the evolution and it may not be visible in play but we can use
> it to create new creatures that get saved and added to the game. So it can
> be used as a dev tool."*

Evolve creatures → **save the good ones** → add them to the game. Measured on
`main` at `ba6fc98`: `serde::Deserialize` was derived on every species and
genome type in the tree and `Serialize` on **none** of them. Everything in
the creature line could measure, film and rank an individual; nothing could
keep one, so every animal the engine has ever produced has died with it.

The economy spine — making the world capable of killing an ant, predation,
switching reproduction on — is the other lane and this depends on none of it.
When S6 lands and something finally breeds, the loop closes with no further
work here: `species_export::organism_as_species` already takes an
`&OrganismState`.

**Nothing on screen changes.** This is a dev tool, not a mechanic.

## The finding, which is worth more than the plumbing

**10 of the first 41 genomes `examples/creature_space.rs` samples carry a
weight in a genome block that no species file could describe — 24%.**

The brain genome has four blocks. Three have an authored form. The fourth,
**hidden self-recurrence** — the four weights that let a hidden unit read its
own previous activation, which is the only memory the creature has — does
not. `eval_brain` reads it every tick, `is_live_slot` counts it live, so
`random_genome` fills it and S6's mutation operator will move it. The three
wiring lists a species file writes cannot reach it.

Exporting such an animal through the existing lists produces a file that
loads, spawns, and has **no memory**. Not a crash, not a parse failure — a
subtly different creature, with nothing in any test or on any contact sheet
saying so. It was invisible for exactly as long as nothing wrote a genome
out, which is why it survived S1–S5.

Fixed by a fourth list, `recurrence: [(0, 0.75)]`, empty on every shipped
species — so their behaviour is untouched — with a test that removes it again
and watches the round trip go red, so the guard is evidence about the
mechanism rather than about itself.

## The design calls, stated so they can be overruled

**A saved creature is a species file, not a genome sidecar.** A species file
is the unit the game already adds; a genome alone is not the whole animal
(body plan, metabolism, nest, dig force and sensor offset are the species')
and would leave every new creature a dependent of the ant, which is what E8
says the result must not be. And the sparse named form stays *reviewable*:
`brain.rs`'s argument that "168 raw floats is not something anyone can
review" does not stop being true because evolution wrote the numbers. So the
export inverts the expansion rather than inventing a second genome format —
what lands on disk looks exactly like a hand-authored file.

**The manifest stamp is a provenance stamp, not a migration, and is described
as one.** `dead-ends.md`'s genome-layout entry carried the re-test condition
*"lawful only while nothing persists a genome"*; this branch is what meets
it, and the entry is annotated in place. Every export stamps
`brain::genome_manifest()` and both load sites refuse a mismatch. It is
load-bearing on exactly one axis — hidden units are indexed positionally and
have no name to be checked against — and it cannot see a slot whose meaning
changed under an unchanged name. The day a reserve fills, a real migration is
still owed.

**A derive with no caller is still a broken channel, and this branch found
one in itself.** `ByOrder<T>`'s new `Serialize` wrote a tuple where its own
`Deserialize` reads a list, so every plant the export touched produced a file
that would not parse — with `cargo test --lib`, both clippy toolchains and
`docscheck` all green, because `individual_as_species` refuses a plant and
the impl therefore had no caller. It is one word
(`self.values.as_slice()`), and the test that now holds it —
`a_plant_species_survives_the_serializer_it_never_asked_for` — was written
*because* nothing called it, not because anything failed.

**An export never overwrites.** `assets/species/` is full of hand-authored
files whose comments carry the reasoning behind every number in them, and a
`ron::ser` round trip destroys comments — which is the whole reason
`tunables::write_field_value` exists. Structural rather than a convention,
because the name comes from a harness argument.

## Using it

```
cargo run --release --example species_export -- from=ant name=grazer genome=r041 gut=-1.0
```

`genome=rNNN` takes the same labels `creature_space` ranks, so a good row in
that sweep — including the random genome that already beats the hand-authored
ant, 0.541 survival against 0.504 — can now be saved as a species instead of
being a number in a log. `verify=1` (the default) reads the file back through
`SpeciesRegistry::reload` and asserts the genome it gets is the genome it
wrote.

## One thing the export deliberately does not write

A creature species needs an `assets/materials/<name>.ron` of the same name —
`plant_creature_seed` resolves the body's material by species name, so an
exported creature with no material hatches *nothing*, silently. The example
checks for it and prints what to do.

It is left to a person because a material is a **palette**, and E8 is
explicit that new creatures "must **not** look like recoloured ants" — which
is exactly what a generated one would be.

## Gates

- `cargo test --lib` — **984 passed, 0 failed, 54 ignored**, including 15 new
- `cargo clippy --all-targets -- -D warnings` — clean on the container's
  1.94.1 **and** on CI's 1.98.0 (`CLAUDE.md`'s toolchain-drift gotcha)
- `bash scripts/docscheck.sh` — clean; `scripts/bugindex.py --check` clean

## Collision surface

`src/sim/species_export.rs` and `examples/species_export.rs` are new.
`brain.rs`, `organism.rs`, `mod.rs`, `ant_ablation.rs` and docs are edited.
**`src/sim/creature.rs` — the 39-landing hotspot — is touched on two lines
only**, both inside `#[cfg(test)]`, both the mechanical fourth argument to
`genome_from_wiring`.

Design record: `Reports/creature-export-design.md`. Lane note:
`Reports/lanes/lane-b-serialisation.md`.
