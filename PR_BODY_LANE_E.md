## What this does

The owner reported that absorbing creature energy destroys the plants around
him, and proposed that the energy particles are colliding with the world.
**They are not, and the `F` key does not touch the world at all** — but the
plants really are dying, and this branch says what is killing them.

Nothing is shipped to the game. What lands is the instrument that could
answer the question, the diagnosis, and the two one-line changes it
prescribes, which live in other lanes' files.

## The answer

**Absorbing writes nothing the simulation reads.** Seven paired arms — same
world, same seed, same elapsed time, the only difference being whether `F`
is pressed — came back **equal in every column**: every plant material, every
organism count, every death cause, every unit of `harvested_plant`. At
`1024x512` and at the shipped `2560x960`; at speed 1 and speed 8; over
drawings up to 779 power; and **again after merging `main`**, an engine change
that moved `src/sim/creature.rs` by 247 lines. `Druid::power` is the only
thing in the entire world that differs.

The motes are `Mote { x, y, bright }` in **screen pixels**, painted by
`render::put` straight into the finished RGBA frame. *"Foreground and not
interacting"* is **already true in both senses**, so building it would have
been a no-op with the report marked addressed — which is the worst available
outcome here, because the plants would have gone on dying.

**What does eat the garden is ants grazing inside a quickening, multiplied by
the speed dial.** On the shipped `bare` start at speed 8, the only difference
being whether a colony stands in the circle:

| | no colony | with a colony | |
|---|---|---|---|
| live plants at end (`1024x512`) | **352** | **295** | −16% |
| live plants at end (`2560x960`) | **690** | **573** | −17% |
| ...of those, near the colony | **407** | **289** | **−29%** |
| plant energy eaten (full scale) | 0 | **21,813** | |
| plants felled (full scale) | 432 | **672** | |

The dial sets the rate: eaten **912 → 21,813** and felled **39 → 672** from
speed 1 to speed 8, everything else fixed. That is `Druid::speed` doing
exactly what its own doc says — *"a plant inside a rate-4 circle is having
four times as much life happen to it"* — applied to grazing.

**Absorbing is upstream of that, and the misattribution is a fair reading
rather than a silly one.** Two couplings tie the key to the damage without it
writing a cell: you must stand in a *running* colony to press it at all, and
with the economy live the power it buys keeps the circle standing longer.
That second one is large — full world, 41 founders, ten presses worth 723
power: plant energy eaten **6,459 → 13,809 (+114%)**, felled 319 → 535. What
absorbing buys is more *time running*, so it buys more growth too
(545 → 563 live plants). **Absorbing really does cost the player plants** —
just not by the mechanism he proposed, and only through the economy.

## Why the nulls can be believed

`CLAUDE.md`'s worst-recurring failure is a number that is arithmetically
correct and about the wrong thing, and seven bit-identical pairs are exactly
the "tidy first result" it warns about. Three controls license reading them:

- **The instrument, against a case known to be broken.** `control=selftest`
  erases 100 plant cells from a grown world and asserts the census reports
  exactly 100.
- **The question, against a case known to be non-zero.** The economy-live pair
  above is the one arm where absorbing *can* affect the world, and the same
  harness sees it immediately: 6,459 → 13,809.
- **Range.** The same census reports 0, 912, 6,459, 13,809 and 21,813 units
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
  cost is invisible, so the player blames the verb he last pressed. The merge
  conflicted here with §Z21, which landed while this branch was measuring;
  both sections survive and the generated index was regenerated rather than
  hand-merged, per `CLAUDE.md`.
- **`Reports/lanes/absorb-destroys-plants.md`**, **`Reports/instruments.md`**.

## What is prescribed, and not done here

The grazing is the game working and wants no fix. What fails the ethos is that
a large ongoing cost has **no visible cause** — the readout prices a circle in
*power* and says nothing about *plants*. Both changes are one line and neither
file is this lane's:

- `src/druid/hud.rs` — a tissue figure beside the drain (`EATEN 21813
  PLANTS 690 -> 573`), from `energy_ledger.harvested_plant` and a live plant
  count, both already on the world.
- `src/druid/mod.rs` — the note the dial raises should say the multiplier
  applies to what eats them too.

## Where I was wrong, and what it cost

**The colony's cost was overstated threefold, and the first review card was
built on it.** Measured before PR #414 landed, the colony took the garden from
352 plants to **177** — half — and I posted a card showing a circle chewed
back to bare ground. `claude/thicket-founding`'s repair (*a floor of plants is
a floor a colony can stand on*) changed where founders can stand: the colony
went from 32 animals to 40 and the garden from 177 to **295**, i.e. −50% to
−16%. **More ants, less damage.** Every number in this PR has been re-taken on
the merged tree; §8 of the report keeps the superseded ones, because the size
of the revision is itself the warning. I **reposted** the card rather than
amending it, per the review protocol — *amend for a defect in the writing,
repost for a defect in the artifact* — and the new card
`20260914T071714820Z-8f4d71` opens by telling the owner to ignore the old one
and why. He had not answered, so nothing was judged on the superseded picture.

**And I repeated a wrong cause the game states out loud.** I reported a
founding in a grown wood placing 2 of 12 ants and blamed `colony_stations`
dropping stations for want of ground — which is `found_colony`'s own *"no
ground here"*, the wrong cause **§Z21** (landed on `main` mid-measurement)
shows the game states out loud. The real one at full scale is the
organism-slot ceiling, 4,093 against a hard 4,095. The tell was in my own log:
`grew 4093 organisms`.

## Not settled

Everything here is seed 1, and the paragraph above is a demonstration that a
single sample can be off threefold for reasons that have nothing to do with
the seed. The **−16%/−17% agreement across two world sizes** is the strongest
thing here and it is still two samples; nothing should be tuned on it without
a sweep. On a *grown* world nothing is destroyed on net at any speed — plant
cells rise — so the reproduction is the shipped `bare` start, and the card
asks the owner directly whether that is what he saw.

## Gates

`cargo clippy --all-targets --release --locked -- -D warnings` clean;
`cargo test --release` on the merged tree **1,756 + 44 + 10 + 3 passed / 0
failed** across all seven test binaries;
`bash scripts/docscheck.sh` clean; `python3 scripts/bugindex.py --check`
current; `python3 scripts/deadendindex.py --touching` 0 hits.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_01QXS28GmjZNMShSj1tR6cXp
