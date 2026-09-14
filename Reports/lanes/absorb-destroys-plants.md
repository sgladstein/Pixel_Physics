# Lane E — "absorbing creature energy destroys plants around it"

*State 2026-09-14: diagnosis complete, nothing shipped, nothing to ship in
this lane's files. **PR [#415](https://github.com/sgladstein/Pixel_Physics/pull/415)**,
head `b0cd55ed`. Full account with every arm and number:
[`Reports/absorb-and-the-garden-2026-09-14.md`](../absorb-and-the-garden-2026-09-14.md).*

Owner: *"Absorbing creature energy, destroys plants around it. The energy
particles need to be foreground and not interact with the world."*

## What was found

**The `F` key does not touch the world.** Five paired arms — same world, same
seed, same elapsed time, differing only in whether `F` is pressed — came back
**equal in every column**: every plant material, every organism count, every
death cause, every unit of `harvested_plant`. The only thing in the world that
differed was `Druid::power`. **So the proposed fix is already true** — the
motes are screen-pixel `Mote`s painted into the finished frame — and shipping
it would be a no-op with the owner told his bug was addressed.

**His observation is still correct.** What eats the garden is **ants grazing
inside a quickening, multiplied by the speed dial**. On the shipped `bare`
start, 3,000 ticks at speed 8, the only difference being whether a colony
stands in the circle: live plants **352 against 177** at `1024x512` and **407
against 326** at full scale, with **19,732** units of plant tissue eaten. The
colony costs a fifth to a half of the garden (two worlds, one seed each — a
range, not a number). The dial sets the rate: eaten 228 → 2,217 from speed 1
to speed 8, felled 28 → 274.

Absorbing is **upstream** of that rather than its mechanism, two ways: you must
stand in a running colony to press `F` at all, and with the economy live the
power it buys keeps the circle standing longer. **That second one is not
small at scale** — full world, 31 ants, ten presses worth 650 power: plant
energy eaten **7,097 → 11,606 (+64%)**, felled 380 → 528. What absorbing buys
is more time running, and more time running is more of everything, growth
included (544 → 608 live plants).

Filed as `Reports/open-bugs-handoff.md` **§Z22** — open on the last clause
(the cost is invisible), not the first. Instrument: `examples/druid_garden.rs`,
new on this branch, row in `Reports/instruments.md`. Card `20260914T043140244Z-b56729` puts the two
frames in front of the owner and asks whether that is what he saw.

## For the coordinator to route — neither file is this lane's

- **`src/druid/hud.rs` (Lane A).** The readout prices the circle in power and
  says nothing about plants. Add one line beside the drain, from
  `World::energy_ledger.harvested_plant` and a live plant count — both already
  on the world, neither needing a new pass:
  `EATEN 15010   PLANTS 352 -> 177`. *A consequence with no visible cause is
  unfinished*, and this is that clause literally.
- **`src/druid/mod.rs` (Lane B).** `Druid::speed`'s doc and `drain_for` both
  price the dial honestly in *power*. Nothing tells the player the same
  multiplier applies to **grazing**. One string in the note raised when the
  dial changes closes it.

**No change is recommended to `absorb`, to the motes, or to the dial itself.**
The grazing is the game working; only its invisibility fails the ethos.

## Corrections to the dispatching brief

- *"None of [`bin/druid.rs`'s hooks] census plants"* — `PIXEL_PHYSICS_DRUID_CENSUS`
  does, per circle. It is still the wrong instrument (no *before*, blind to a
  freed organism), so the conclusion stands.
- `step_extra_ticks`'s `world.player.take()` was flagged as worth a look. It
  is clean: `frame::step` sets `world.carried = None` with no player and
  `player::step` returns immediately.
- The hypothesis *"absorbing funds the power that keeps circles alive"* is
  real but is the **smaller** coupling — with power abundant it contributes
  nothing and the plants still die. The larger one is that pressing `F`
  requires standing in a running colony, which the brief did not name.
- `plant_bending` was flagged as unchecked. Ruled out by measurement: off, at
  speed 8, felled 269 against 274.

## Found on the way, not this bug, not this lane

A founding in a grown wood places **2 of its 12 ants**; on bare ground, 32 of
48. `colony_stations` drops every station whose column has no
`colony_ant_site` at the founder's height, and a wood's ground is under its
own litter. Nothing reports it but the count in the log line.
