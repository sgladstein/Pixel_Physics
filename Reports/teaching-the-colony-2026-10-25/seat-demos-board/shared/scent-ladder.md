# Shared piece: ants that react to a visitor by its smell (the inspection ladder)

Owner: **guest** seat (thread "Build the guest seat demo", branch claude/project-thread-rdj55n).
State: **BUILT 2026-10-25** on branch claude/project-thread-rdj55n (PR to the merge desk; see the guest status file for the number). Was PROPOSED 08:35Z. Needed by: guest (the core of the seat), keeper (the gnome as symbiont),
enemy (raids, if it wants ants to answer a predator by smell). Comments: message the guest thread or write in
your own status file.

## Why it is needed
Today a mind ant sorts every other animal by colony number (`creature/options.rs` describe: NESTMATE/STRANGER is
`o.colony == here.colony`), offers Feed only to its own colony and Attack to anything else, and never looks at
the three-number scent every animal carries. So nothing in the lab can be *half* accepted: a visitor is either
family (by number) or a target. The ladder is a body rule, like nursing by touch, that reads the scent model and
grades the answer. It does not touch brain weights, so it is clear of the weight-zero trap.

## Interface (proposal; details after the guest plan is reviewed)
New file `src/sim/guest.rs`, plus one hook in `creature.rs` before `mind::tick` and one gate in `options.rs`.
- `World::visitors: Option<Visitors>`. **`None` = off, byte-identical** (one Option read per ant tick).
- A *visitor* is anything with a scent the colony should judge: an organism id (the guest beetle, a predator),
  or a non-organism point (the keeper's gnome) registered each tick with `visitors.set_point(key, cells, scent)`.
- `guest::scent_of(world, id) -> [f32; 3]` and `guest::gap(world, ant, visitor_scent) -> f32` = scent distance
  divided by **the ant's own** tolerance radius (the host judges, not the visitor).
- `guest::Rung { Ignore, Antennate, Display, Nip, Drag }`, rolled on contact from (gap, alarm at the spot,
  the ant's hunger) with floors so no rung is ever certain (the contest module's shape).
- The hook: a mind ant touching a visitor spends this decision on the rung (stands and faces it; an alarm puff;
  a gnaw of the visitor's cells). The ant's mind is skipped only for that decision.
- The gate: minds are never offered Attack on a registered visitor; the ladder's Nip/Drag is the only bite, so
  harm is graded instead of the mind's one-shot kill.
- Read-outs for HUDs: per-ant last rung + frame (to draw a mark over the ant), per-visitor counters
  (antennations, displays, nips, feeds), and an event list for a ledger.
- Feeding and soaking (guest verbs) live beside it in the same file but are guest-only calls.

## What each seat would call
- guest: registers its beetle id; reads rung marks and counters for the HUD and ledger.
- keeper: registers the gnome as a point visitor each tick with a scent the keeper code moves (boots in nest
  soil -> scent toward the nest). The ladder then answers him with no trust meter.
- enemy: optional; registers the predator so ants that touch it antennate/display/nip by smell.

## As built (2026-10-25, supersedes the proposal above where they differ)
- File is **`src/sim/creature/guest.rs`** (a child of `creature`, so it reuses the bite, alarm and crop code).
- `World::visitors: Option<guest::Visitors>`; `None` = off, cell-for-cell identical (guard:
  `a_ladder_with_no_living_visitor_changes_nothing`, with a positive control).
- Register a body: `guest::register(world, host_species, id, home_scent)`. Its scent fades back to `home_scent`
  (tau 60 s). `guest::scent_away_from(nest_scent, dist)` gives a scent a set distance from a nest's odour.
- Rungs: `Rung { Ignore, Antennate, Display, Nip, Fed, Empty, Rubbed }`; `guest::odds(g, scale)` (centres 0.5 / 0.9 /
  1.3, floors, no nip below 0.8). Effective gap adds alarm (<= +0.25), per-ant pickiness (+-0.2), hunger (<= +0.2).
- Hooks: `guest::meet` in creature_tick before the mind (never returns early); `guest::force_options` inside
  `mind::tick` (a held ant gets only Rest, then one Walk away); Attack never offered on a registered visitor;
  `guest::tick` at the end of `lab::tick_world`. **Trained (mind) ants only**: the hand-built ant's own fight is
  not gated.
- Read-outs: `Visitors::marks` (per ant: rung, frame), `Visitors::counts`, `Visitors::events` (last 64).
- Not built: point visitors (the keeper's gnome). Adding one means a `Visitor` whose cells come from the caller
  each tick; `meet` reads `world.organism(vis.id)` today.
