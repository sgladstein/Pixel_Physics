# Lane C — the pheromones

*Round 36. The owner's two questions, 2026-09-14, in his order: **"do they
fade too fast to be useful?"** and **"make sure they are all wired up
properly."* Both answered. **Full account:
[`../pheromone-lifetime-and-wiring-2026-09-14.md`](../pheromone-lifetime-and-wiring-2026-09-14.md)**
— this note is the summary and the two items routed elsewhere.*

**Nothing was tuned.** Every default ships unchanged; one dial was added
because the parameter that turned out to matter had none.

---

## Q1 — yes, but say it precisely

**A trail here is a live map of where ants are standing now. It is not a
memory of where they went.**

- **Unreinforced, a trail is gone in 144 frames** — 0.065x a 2,200-frame
  round trip, where the module's own arithmetic reads **1.4x**.
- **The 255-pass ceiling is real and its margin is not.** 255 passes is what
  a cell *at 255* survives, and nothing writes 255: the floor is
  subtractive, so a cell's ceiling in passes **equals its own value**. A cell
  laid at `DEPOSIT` (40) has a **40-pass** ceiling, and with the blend
  running it dies in **12**.
- **It stops steering before it is gone** — the ant's own run drive hits zero
  at frame 48 with 77 cells still standing, because quantization flattens the
  ramp into plateaus and `here == ahead` reads exactly 0.
- **A cell needs re-laying every ≤36 frames.** At 60 frames it is gone. On a
  2,200-frame circuit that is ~20–30 ants walking one route cell-for-cell.
- **The real bed agrees** (`mode=world`, six seeds): at ~50 ants the network
  stands at **149–393 cells, peak 39–98 of 255**. It is superlinear in colony
  size — 52→46 ants holds it, **46→20 takes it from 342 cells to 35**.

**So the classic job of a trail — a scout recruiting the colony to a patch it
found — is not reachable.** The scout's trail is gone long before the scout
is home.

### The knob the doc names is inert

**`DECAY_RHO` is not *"the parameter the whole mechanism balances on"*.**
Setting it to **zero** leaves that 144 frames **unchanged**. A trail is a
one-cell-wide line, so the blend takes **16.7% per pass** against decay's
**2.9%** — realised rate ~0.19, *inside* the literature band `DECAY_RHO` is
documented as sitting deliberately below.

**`DIFFUSE` is the term, and it had no setter at all.** This branch ships
`Pheromones::set_channel_diffuse` (per channel, alarm's pending-value split
handled as `alarm_rho` already does it), `DIFFUSE` unchanged at 0.25.
Lowering it is not free: `DIFFUSE`'s own sweep scores 0.10 at 0.623 on-trail
against 0.25's 0.817. **Long-lived and hard to track, or short-lived and easy
— the existing sweep only ever saw one of those axes.**

### Do NOT halve `DEPOSIT`

P-14 says halve it if trails pin at 255. **Measured: the loudest cell in the
bed is 98 of 255, over six seeds.** The trigger has never fired; the plane is
three-quarters empty at its peak and the problem is at the other end.

---

## Q2 — nothing is broken in the Rust; four findings in the authored half

All seven reader slots are computed every tick, both trail planes have a live
reader and a live writer, and the alarm plane's writers fire (336 deposits in
a 9,000-frame bed, plane live). `examples/pherowire` walks every shipped
genome rather than grepping the `.ron` — `ant.ron` wires through hidden
units, and a weight under `brain::W_EPS` is dead on arrival to `eval_brain`.

**1. Four of seven reader slots are read by no species at all** —
`PheroAFront`, `PheroALateral`, `PheroBFront`, `PheroBLateral`, **0 of 11**.
The laterals are deliberate and documented (both sit in open air in a
side-view world, measured 0.000). **The Fronts are not**, and they are the
only *concentration* inputs — the Along slots are scale-free by construction.
**So no shipped animal can tell a strong trail from a weak one**, and that is
the sole justification `DIFFUSE`'s value was chosen on (*"the height of a
well-used trail against a lightly-used one … is the entire path-selection
algorithm"*). The trade was made for a benefit no shipped genome can collect.
*Not proposed here* — it reallocates a shared weighted sum and wants a seed
sweep on an order statistic, not an A/B.

**2. The alarm plane's audible radius is about two cells.** A wound (240):
the loudest a neighbour **one cell away** ever hears is **6 of 255** — input
0.024, **+0.047** into `Attack` against an authored weight of 2.0. Two cells:
**zero, ever**. A **`DISPLAY_DEPOSIT` (40) is inaudible to anyone but the
displaying animal** — that is the measurement `contest.rs` asked for and
nobody had taken. Controlled with a sustained arm (a wound every 6 frames
over a 2-cell body, 40 bites): even then, two cells off reads 0.059, four
cells off reads zero for the whole fight. `creature::sense` calls the
here-read *"a limitation of one slot, not of the plane"* — **it is the
plane**: `ALARM_RHO` grinds the signal down faster than `DIFFUSE` spreads it,
so there is no distance for a directional slot to read even if one existed.

---

## Routed to Lane D — `assets/species/*.ron` is yours, not mine

**1. `ancestor.ron` cannot hear the alarm.** Every other ant-family species
carries `(Alarm, Move, -1.0)` and `(Alarm, Attack, 2.0)`. `ancestor.ron`
carries **neither**, and the word "alarm" does not appear in the file — an
omission, not a recorded choice. It is the only species that reads the trail
planes and not the alarm. **It matters now** because round 35 shipped
rivalry **on**: in a bed founded on the ancestor, fights write the plane
every time and the founding lineage cannot act on it.

**The change I want**: the two weights the other nine already carry, added to
`ancestor.ron`'s instinct list.

**Read it against my §2d before deciding how much it buys.** At the measured
two-cell reach, those weights only fire for an animal already touching the
fight — so this closes a real gap in the wiring and should not be sold as
recruitment. It is cheap and it is correct; it is not a fix for the plane.

**2. `flitter` neither lays nor reads a trail.** It carries the two alarm
weights and nothing else — no `EmitA`/`EmitB`, no reader on any trail slot.
The laterals in finding 1 above are kept in the codebase explicitly *for*
something moving in open space, and the one animal that does is the one that
does not read them. **A design question rather than a defect** — recorded
because the slots' stated justification names this animal.

`beetle` has a brain and no pheromone wiring of any kind; for a solitary
animal that is coherent and I am not flagging it.

---

## For the coordinator

- **Owner's Q1 answer in one line, if you need it for a poke:** *they fade
  fast enough that a trail is a live map of where ants are, not a memory of
  where they went — a scout's trail is gone five times over before the scout
  gets home, and the knob that fixes it (`DIFFUSE`) had no dial until now.*
- **`PHEROMONE_INTERVAL` is on the perf line's handed-forward list** (the
  `roundf`). My numbers are taken at 12 unscaled; `World::step_pheromones`
  scales it by the creature clock and keeps passes-per-tick exact, so the
  *ratios* here survive a clock change and the frame counts do not. **Any
  change to that constant moves the trail's whole lifetime** — it is the
  ceiling's other term.
- **No review card posted.** The queue was empty at 18:40Z and stayed empty;
  every finding here is a number, and the queue is for visual evaluations
  only.

## Gates, all green on this branch

`clippy --all-targets --release --locked -D warnings` · `test --lib` (1,805
passed) · `test --test worldgen --test determinism` (47 passed) ·
`example ascii` (31 scenes, 0 skipped) · `docscheck.sh`

The new guard `the_blend_dial_reaches_the_pass` was **watched going red**
with the setter stubbed to a no-op, per `CLAUDE.md`.
