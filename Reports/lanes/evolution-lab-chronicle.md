# The chronicle carries a census — lane note

Built for `Reports/evolution-lab-late-game-design-2026-09-12.md` brief 0. The
owner plays real sessions that reach hundreds to over a thousand ants, far
past anything a headless run produces here, and wants to hand a session's log
over for us to examine. This is what that handoff looks like.

## What the file is and where it comes from

`assets/chronicles/chronicle-<date>-<bed>-s<seed>.txt`, gitignored (a
player's own record, not authored content). Written by `Lab::write_chronicle`
on three occasions now: pressing `REBUILD`, closing the window, and **pressing
`9` at any point during play** (new — the key list names it under the other
key-only controls). All three call the identical export, so a mid-session
save and the one taken automatically on quit are the same shape.

**To get one from the owner: ask for the file at that path.** It never
uploads itself; it is a local text file the owner attaches or pastes. The
filename carries a date, not a time, so a second `9`-press the same day
overwrites the first save — `9` is for "I want this now," not a mid-session
series.

## What it carries

Four parts in order: a header (bed, seed, founders, colonies, frame, dial),
what dials the player changed from the shipped defaults (blank if none), a
**CENSUS** section, then the LOG/LEGENDS story the chronicle always had.

The CENSUS section is new. One row every 10,000 simulated frames (`Lab::
CHRONICLE_CENSUS_EVERY`, overridable via `PIXEL_PHYSICS_CHRONICLE_CENSUS_EVERY`
for a harness that cannot afford ten thousand real ticks), oldest row first,
frame first:

- **ants, plants** (living organisms, waiting seeds excluded), **seed bank**;
- **edible joules**, gut-priced through `creature::diet_yield`, split leaf /
  fruit / litter / seed / corpse / flower;
- **births, deaths** by cause (starved, killed, other), **eats, digs,
  deliveries**;
- the **nest's footprint**: roofed void (chambers), open pit, packed cells
  above and below the original surface, the mound's height;
- the **dead zone**: bare-ground ratio in the nest's ±64-column band against
  outside it, spelled out again in plain terms under each row — that ratio
  and the mound are what the owner watches (§ below).

Every column and every formatter live in `pixel_physics::lab::census`, one
function shared by this export, `examples/chronicle.rs`'s own `census=1`, and
`examples/latecensus.rs` (now that module's harness and positive control) —
so a number read off a real session's file and a number read off a headless
run are never two implementations of the same idea.

## What a session's log answers that a headless run cannot

Everything in `Reports/evolution-lab-late-game-design-2026-09-12.md` brief 0
turns on scale and on the owner's own interventions — neither of which a
scripted scenario reproduces:

- **whether the colony outgrows its larder before the bed can rebuild it**,
  read as the larder-by-kind columns falling while `ants` climbs;
- **whether digging quietly kills the ground above the nest** — "they dig
  large chambers underground and piles of dirt/chambers above the nest, which
  creates an area where plants don't grow" (the owner's own words) — read
  directly off the bare-ground ratio and the mound height, row over row;
- **what a real player did to the box mid-session** — a rebuild, a changed
  dial, a hand-placed jar — which the DIALS line and the CENSUS section's own
  discontinuities (a sudden `ants` drop with no `killed` to match) show up as
  events a scripted run never produces because nothing scripted it.

## Cost

The census scans the whole grid once per sample; at the 10,000-frame cadence
this is a call every few minutes of played time, not a per-frame cost. See
the PR body for the measured number on the played bed at 100,000 frames.
