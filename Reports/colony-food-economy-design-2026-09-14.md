# The colony food economy, made visible — round 35's design

*2026-09-14. Written by the round-35 coordinator from the owner's ask, before
any lane started. Status: **plan; three lanes implementing it.***

> ***"We should explore better instruments, visualizations, whatever for the
> player to understand the food economy of each colony. I want to know what
> they are eating, where it is coming from, if/where it is being stored or
> movement paths, general colony food stats/balances."***

## 1. The finding that reframes the whole job

**The books already exist and already balance.** `World::energy_ledger` is a
closed double-entry ledger with conservation identities asserted by tests:

| | accounts |
|---|---|
| **sources** | `granted` (metabolic energy at spawn), `stamped` (structural energy, unspendable, becomes meat), `harvested_plant` |
| **transfers** | `harvested_corpse` (meat stock → live stock), `stored_in_meat` (live → meat) |
| **sinks** | `metabolized`, `moved`, `synapse_tax`, `dissipated`, `meat_lost` |
| **correction** | `overdrawn` — charges that landed on an animal which could not pay them |

Its own doc records why that shape was forced: the old ledger had one `eaten`
account defined as *whatever happened*, so when a beetle bit an ant the
victim's remainder was written off, both were booked, the identity held, and
**300 joules were conjured**. The property it now protects is the one
evolution actually needs — **no lineage may extract unbounded energy from a
cycle it controls.**

**So this is not building an economy. Three things are missing and they are the
whole job:**

1. it is **world-wide**, not per colony;
2. it has **no face** — nothing draws it;
3. it has **no location** — no account knows where a joule came from.

And one live fact sizes the work: **70–90% of ant deaths in the lab bed are
starvation, on every seed.** The food economy is not one system among many. It
is what kills almost everything.

## 2. What is already there to build on

- **`OrganismState::colony` is a `u32`**, and `creature::is_living_kin`
  already keys on it. Per-colony identity exists; it is simply never used for
  accounting or for drawing.
- **`Crop { material, shade, unit, cells }`** — what an ant is carrying,
  including *which material* and its face value, is known per animal per frame.
- **`creature::diet_yield` + `EAT_YIELD_THRESHOLD`** price every mouthful, so a
  readout built on them **cannot disagree with the verb**. That is the
  construction `labforage` already uses and the reason it is trustworthy.
- **Three agent-facing food harnesses already exist** and should be read before
  anything is rebuilt: `labforage` (is the colony starving because the food is
  gone or because it never gets to it — its `unvisited` column is the
  separator), `larder_probe` (store or flow, by tracking the band as a *set of
  positions*), `windfall_probe` (Little's law on the fruit pipeline).
  **All three are CLI instruments for answering a measurement question. None of
  them is a thing a player sees.** That gap is the assignment.
- **The lab has twelve panels** — Plants, Ants, Box, Params, Shelf, Chambers,
  PlantList, AntList, Log, Compare, Scenarios, History — **and not one of them
  is about food.**

## 3. The five pieces, and which lane owns each

**Lane A — the half that is seen.** `src/render.rs`, `src/bin/lab.rs`.

- **The laden trail.** A decaying trail of where ants were *carrying*, distinct
  from where they merely walked. The food road, as a shape.
- **The harvest map.** The world shaded by joules taken from there, per colony,
  decaying so it shows current foraging grounds rather than an all-time smear.

**Lane B — the books.** `src/sim/*`, `src/lab/*`.

- **The per-colony ledger split.**
- **The diet band** — joules in by source material over time.
- **A food panel** — income by source, outgo split, margin and trend.

**Lane C — why colonies never fight or eat each other.** `Reports/*`. §5.

## 4. The traps, each already paid for once

- **Count joules, not cells.** A cell of moss and a cell of corpse are not the
  same food. A cell-count diet chart is a confident wrong answer of exactly the
  shape `CLAUDE.md` calls this repo's worst-recurring failure.
- **Splitting a closed ledger per colony breaks closure** unless *transfers
  between colonies* become their own account — one colony's ant eating
  another's is a transfer, not a source. Design it in, or the identities will
  not close and the temptation will be a free term, which is the exact failure
  the ledger's own doc records.
- **Show a distribution, not an average.** Round 33 cost precisely this: a
  pooled idle rate could not separate *"everyone rests briefly"* from *"a fifth
  are frozen"* — both give 75% — and the owner caught it from the screen when
  the number did not. A colony's *average* forager will hide the same thing.
  `OrganismState::deliveries` is already per animal.
- **An outcome is a distribution, not a binary** — the owner's stated core
  value. "Thriving / starving" is the binary it rejects; show near-starvation.
- **A standing count cannot tell a store from a conveyor.** `larder_probe`
  already found a "granary of ten cells" that was ten cells in transit,
  `resident` 0 from frame 200. The readout that answers *"is it stored"* is
  **dwell time**, not quantity.
- **Overlays must be a full replace on a fixed dark→bright ramp**, never a
  blend into the cell's own colour: a magnitude-scaled blend once produced a
  sheet that read as blank, because the ramp was red, wood is brown, and a
  mid-range value moved one colour byte from 139 to 155.
- **Frame cost is a hard constraint.** Accumulate on a coarse grid, and gate
  the work so an overlay that is off costs nothing — an animated grain once
  looked free in every moving scene and cost ~10 ms/frame on a *settled* one,
  because what it defeats is the dirty-rect render skip.
- **For creatures, a GIF is not a preference, it is the only instrument.**
  Owner, on a contact sheet of a starving colony: *"visually, I cannot tell
  anything from these. ants are mostly visible with there motion."*

## 5. Why colonies never fight or eat each other

The owner asked this as a **review-and-explore**, not as a build. Most of the
answer was already in the repo; three independent gaps, **all authorship
rather than machinery**:

**Gap 1 — they are not strangers, and this is the least known of the three.**
`Behavior::scent_spread` defaults to **0**, which its own doc says *"puts every
colony at the species' authored point, so two colonies are one family: the
shipped behaviour."* At `1.0` with the ancestral tolerance at `-1`, *"every
click is a stranger to every other, which is exactly and only what the retired
`colony rivalry` switch did."* **So the shipped bed holds no rival colonies —
only one extended family.**

**Gap 2 — the shipped ant is blind.** `sight()` = 0 on every ant variant.

**Gap 3 — nothing wires initiation.** Per
[`held-world-game-concept-2026-09-13.md`](held-world-game-concept-2026-09-13.md)
§10a: ***"The engine has a fully working fight and no way to start one."***
`Attack` is implemented, priced, counted, guarded and authored into 9 of 20
species; put a non-kin animal in reach and it works hard — **296–478 attacks
over 9,000 frames**. On the owner's bed: **`attacks 0`, every death
starvation, over nine runs of 300,000 frames.** `ThreatNear` and
`ThreatBearing` exist, are written and read, and **carry zero authored weight
in any species file**; the only route to `Attack` is `Alarm`, which only a
landed bite raises. So the wire is **retaliation, never initiation**.

**And "eating each other" is a separate question from "fighting."**
`harvested_corpse` is already its own account, so a colony may already be
scavenging another's dead with nobody able to see it — which would make it a
**readout** gap rather than a mechanism gap, and Lane B's split is what would
reveal it.

**Any switch ships default-off pending the owner's eye.** The standing rule is
*ship new behaviours on*; this one changes the ecology of every bed in both
games, he asked to explore rather than to build, and he has not seen it. The
honest shape is default-off plus a card, and he can rule otherwise in a
sentence.

**One rejection binds the design**: `dead-ends.md` :272, :276, :403 —
**protection as an exemption rather than a capacity multiplier**, which killed
four successive support models. Nothing here may give one side an exempt state;
it has to be a capacity on the same axis as everything else.
