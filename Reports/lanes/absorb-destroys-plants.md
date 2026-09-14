# Lane E — "absorbing creature energy destroys plants around it"

*State 2026-09-14: diagnosis complete, nothing shipped, nothing to ship in
this lane's files. **PR [#415](https://github.com/sgladstein/Pixel_Physics/pull/415)**;
merged `main` (0 behind), and **every number re-taken on it**. Full account:
[`Reports/absorb-and-the-garden-2026-09-14.md`](../absorb-and-the-garden-2026-09-14.md);
filed as `open-bugs-handoff.md` **§Z22**.*

Owner: *"Absorbing creature energy, destroys plants around it. The energy
particles need to be foreground and not interact with the world."*

## What was found

**The `F` key does not touch the world.** Seven paired arms — same world, same
seed, same elapsed time, differing only in whether `F` is pressed — came back
**equal in every column**: every plant material, every organism count, every
death cause, every unit of `harvested_plant`. At both world sizes, at speed 1
and 8, and **again after merging `main`**, over drawings up to 779 power. The
only thing in the world that differed was `Druid::power`. **So the proposed fix
is already true** — the motes are screen-pixel `Mote`s painted into the
finished frame — and shipping it would be a no-op with the owner told his bug
was addressed.

**His observation is still correct.** What eats the garden is **ants grazing
inside a quickening, multiplied by the speed dial**. Bare start, 3,000 ticks,
speed 8, the only difference being whether a colony stands in the circle: live
plants **352 → 295** at `1024x512` and **690 → 573** at full scale, and near
the colony **407 → 289**. The colony costs about a sixth of the garden and up
to about a third of what stands next to it. The dial sets the rate: eaten
**912 → 21,813** and felled **39 → 672** from speed 1 to speed 8.

Absorbing is **upstream** of that rather than its mechanism, two ways: you must
stand in a running colony to press `F` at all, and with the economy live the
power it buys keeps the circle standing longer. **That second one is big** —
full world, 41 founders, ten presses worth 723 power: plant energy eaten
**6,459 → 13,809 (+114%)**, felled 319 → 535. What absorbing buys is more time
running, and more time running is more of everything, growth included.

Instrument: `examples/druid_garden.rs`, row in `Reports/instruments.md`. Card
**`20260914T071714820Z-8f4d71`** asks the owner whether that is what he saw.

## For the coordinator to route — neither file is this lane's

- **`src/druid/hud.rs` (Lane A).** The readout prices the circle in power and
  says nothing about plants. Add one line beside the drain, from
  `World::energy_ledger.harvested_plant` and a live plant count — both already
  on the world, neither needing a new pass: `EATEN 21813  PLANTS 690 -> 573`.
  *A consequence with no visible cause is unfinished*, and this is that clause
  literally.
- **`src/druid/mod.rs` (Lane B).** `Druid::speed`'s doc and `drain_for` both
  price the dial honestly in *power*. Nothing tells the player the same
  multiplier applies to **grazing**. One string in the note the dial raises
  closes it.

**No change is recommended to `absorb`, to the motes, or to the dial itself.**
The grazing is the game working; only its invisibility fails the ethos.

## Corrections — including two to my own work

**Mine, and they are the ones worth reading.**

- **The colony's cost was overstated threefold.** Measured before PR #414
  landed it was −50% of the garden; on today's `main` it is **−16%**.
  `claude/thicket-founding`'s repair (*a floor of plants is a floor a colony
  can stand on*) changed where founders stand — 32 ants became 40, and the
  garden went from 177 plants to 295. **More ants, less damage.** My first
  review card was built on the old figure and showed a circle chewed to bare
  ground; I **reposted** (protocol: amend for a defect in the writing, repost
  for a defect in the artifact) and the new card opens by telling the owner to
  ignore the old one. He had not answered, so nothing was judged on it.
- **I repeated a wrong cause the game states out loud.** I said a founding in
  a grown wood places 2 of 12 ants because `colony_stations` drops stations
  for want of ground. **§Z21** measured the real cause at full scale — the
  organism-slot ceiling, 4,093 against a hard 4,095 — and `found_colony`'s
  *"no ground here"* is precisely the wrong explanation I echoed. The tell was
  in my own log and I read past it: `grew 4093 organisms`. (It is not all of
  it: the `1024x512` grown world holds 973 organisms and still places 5 of 12.)

**To the dispatching brief.**

- *"None of [`bin/druid.rs`'s hooks] census plants"* — `PIXEL_PHYSICS_DRUID_CENSUS`
  does, per circle. Still the wrong instrument (no *before*, blind to a freed
  organism), so the conclusion stands.
- `step_extra_ticks`'s `world.player.take()` was flagged as worth a look. It
  is clean: `frame::step` sets `world.carried = None` with no player and
  `player::step` returns immediately.
- The hypothesis *"absorbing funds the power that keeps circles alive"* is
  real and large (+114%), but it is not the *only* coupling and with power
  abundant it contributes nothing while the plants still die. The one the
  brief did not name: pressing `F` requires standing in a running colony.
- `plant_bending` was flagged as unchecked. Ruled out by measurement: off, at
  speed 8, felled 269 against 274.
