## What this does

The owner reported that absorbing creature energy destroys the plants around
him, and proposed that the energy particles are colliding with the world.
**They are not, and the `F` key does not touch the world at all** — but the
plants really are dying, and this branch says what is killing them.

Nothing is shipped to the game. What lands is the instrument that could
answer the question, the diagnosis, and the two one-line changes it
prescribes, which live in other lanes' files.

## The answer

**Absorbing writes nothing the simulation reads.** Five paired arms — same
world, same seed, same elapsed time, the only difference being whether `F`
is pressed — came back **equal in every column**: every plant material, every
organism count, every death cause, every unit of `harvested_plant`. At
`1024x512` and at the shipped `2560x960`; at speed 1 and speed 8; with one ant
and with thirty-one; over drawings up to 779 power. `Druid::power` is the only
thing in the entire world that differs.

The motes are `Mote { x, y, bright }` in **screen pixels**, painted by
`render::put` straight into the finished RGBA frame. *"Foreground and not
interacting"* is **already true in both senses**, so building it would have
been a no-op with the report marked addressed — which is the worst available
outcome here, because the plants would have gone on dying.

**What does eat the garden is ants grazing inside a quickening, multiplied by
the speed dial.** On the shipped `bare` start at speed 8, the only difference
being whether a colony stands in the circle:

| | no colony | with a colony |
|---|---|---|
| live plants at end (`1024x512`) | **352** | **177** |
| live plants near (`2560x960`) | **407** | **326** |
| plant energy eaten (full scale) | 0 | **19,732** |
| plants felled (full scale) | 432 | **807** |

The dial sets the rate: eaten **228 → 2,217** and felled **28 → 274** from
speed 1 to speed 8, everything else fixed. That is `Druid::speed` doing
exactly what its own doc says — *"a plant inside a rate-4 circle is having
four times as much life happen to it"* — applied to grazing.

**Absorbing is upstream of that, and the misattribution is a fair reading
rather than a silly one.** Two couplings tie the key to the damage without it
writing a cell: you must stand in a *running* colony to press it at all, and
with the economy live the power it buys keeps the circle standing longer.
That second one is large at scale — full world, 31 ants, ten presses worth 650
power: plant energy eaten **7,097 → 11,606 (+64%)**, felled 380 → 528. What
absorbing buys is more *time running*, so it buys more growth too
(544 → 608 live plants).

## Why the nulls can be believed

`CLAUDE.md`'s worst-recurring failure is a number that is arithmetically
correct and about the wrong thing, and five bit-identical pairs are exactly
the "tidy first result" it warns about. Three controls license reading them:

- **The instrument, against a case known to be broken.** `control=selftest`
  erases 100 plant cells from a grown world and asserts the census reports
  exactly 100.
- **The question, against a case known to be non-zero.** The economy-live pair
  above is the one arm where absorbing *can* affect the world, and the same
  harness sees it immediately: 7,097 → 11,606.
- **Range.** The same census reports 0, 228, 2,217, 15,010 and 19,732 units
  eaten across the arms. It is not stuck.

Identity is the *prediction* here — the sim is deterministic same-build and
absorb provably writes nothing the sim reads — rather than a suspicious
result.

## Also ruled out, by measurement rather than by reading

- **`plant_bending`**, flagged as unchecked (`Druid::new` clears
  `plant_load_failure` and leaves this at its `true` default): off, at speed 8,
  felled **269 against 274**.
- **`step_extra_ticks`'s `world.player.take()`**: clean. `frame::step` sets
  `world.carried = None` when there is no player and `player::step` returns
  immediately — no cell written, no stale body left.

## What lands

- **`examples/druid_garden.rs`** — a paired plant census for the held world,
  which did not exist. `PIXEL_PHYSICS_DRUID_CENSUS` counts tissue inside each
  circle and exits (no *before*, blind to a freed organism); `flora_census`
  never holds its world; `latecensus` reads a `LabBox`. So `CLAUDE.md`'s own
  instruction for this question — *"census the materials before and after"* —
  could not be followed. Plant-kind cells by material (world, a 200-cell
  window, and the 60-cell absorb radius), live plant and animal organisms,
  `deaths_by_cause`, and `energy_ledger.harvested_plant`. `png=<prefix>`
  renders the real game headlessly through `Druid::draw`, needing no surface,
  no xvfb and no GPU.
- **`Reports/absorb-and-the-garden-2026-09-14.md`** — every arm and number,
  with its line in `Reports/README.md`.
- **`Reports/open-bugs-handoff.md` §Z22** — open on the last clause only: the
  cost is invisible, so the player blames the verb he last pressed.
- **`Reports/lanes/absorb-destroys-plants.md`**, **`Reports/instruments.md`**.

## What is prescribed, and not done here

The grazing is the game working and wants no fix. What fails the ethos is that
a large ongoing cost has **no visible cause** — the readout prices a circle in
*power* and says nothing about *plants*. Both changes are one line and neither
file is this lane's:

- `src/druid/hud.rs` — a tissue figure beside the drain, from
  `energy_ledger.harvested_plant` and a live plant count, both already on the
  world.
- `src/druid/mod.rs` — the note the dial raises should say the multiplier
  applies to what eats them too.

## Not settled

The colony's cost is a **range** (a fifth to a half) because it is two worlds
at one seed each. Outcomes here are chaotic in the seed; nothing should be
tuned on those numbers until they are swept. And on a *grown* world nothing is
destroyed on net at any speed — plant cells rise — so the reproduction is the
shipped `bare` start, and card `20260914T043140244Z-b56729` asks the owner
directly whether that is what he saw.

## Gates

`cargo clippy --all-targets --release --locked -- -D warnings` clean;
`cargo test --lib --release` **1,750 passed / 0 failed / 85 ignored**;
`bash scripts/docscheck.sh` clean; `python3 scripts/bugindex.py --check`
current; `python3 scripts/deadendindex.py --touching` 0 hits.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_01QXS28GmjZNMShSj1tR6cXp
