# Lane C — the pheromones

*Round 36. The owner's two questions, 2026-09-14, in his order: **"do they
fade too fast to be useful?"** and **"make sure they are all wired up
properly."* Both answered. **Full account:
[`../pheromone-lifetime-and-wiring-2026-09-14.md`](../pheromone-lifetime-and-wiring-2026-09-14.md)**
— this note is the summary and the two items routed elsewhere.*

**Nothing was tuned.** Every default ships unchanged; one dial was added
because the parameter that turned out to matter had none.

---

## Q1 and Q2, in one screen — **detail in the report, not here**

Full account, with every table:
[`../pheromone-lifetime-and-wiring-2026-09-14.md`](../pheromone-lifetime-and-wiring-2026-09-14.md).

**Q1 — do they fade too fast?** Yes. **A trail is a live map of where ants
are standing, not a memory of where they went.** Unreinforced it is gone in
**144 frames** against a 2,200-frame round trip (0.065x), where the module's
own arithmetic reads 1.4x — because the 255-pass ceiling prices a cell at 255
and nothing writes 255. **`DECAY_RHO` is inert**: setting it to zero leaves
that 144 unchanged, because a one-cell line loses **16.7%/pass to `DIFFUSE`**
against decay's 2.9%. A cell needs re-laying every **≤36 frames**. Real bed,
six seeds: at ~50 ants the network stands at 149–393 cells, peak 39–98 of 255
— **superlinear in colony size** (46→20 ants takes 342 cells to 35). **Do not
halve `DEPOSIT`**; P-14's trigger has never fired.

**Q2 — is it wired?** Nothing is broken in the Rust. **Four of seven reader
slots are read by no species**: the laterals deliberately, but
`PheroAFront`/`PheroBFront` are the only *concentration* inputs, so **nothing
reads trail height** — which is the sole justification `DIFFUSE`'s value was
chosen on. Named, not changed: it reallocates a shared weighted sum.

## The alarm is fixed (2026-09-14, `claude/pheromone-trail-lifetime`)

**The owner asked "are we fixing any of these?" and he was right to.** Two
answers, because the two planes are different problems.

**Fixed.** §2d's two-cell reach is not a tuning failure — the ceiling is the
*stencil*. A 3x3 mean attenuates ~9x per cell, so even at `DIFFUSE = 1.0` a
wound reads 20 at one cell and 1 at two. **The error was modelling a shout as
a substance**: a mean filter conserves, which is right for a trail (deposits
adding up *is* path selection) and fatal for an alarm. `Spread::ActiveSpace`
propagates by distance falloff — louder of (this cell, neighbour −
`ALARM_FALL`) — which is the *active space* of the real thing. One wound:

| | d=1 | d=2 | d=4 |
|---|---|---|---|
| before (`arm=diffuse`, kept reachable) | 4 | **0** | 0 |
| after | **148** | **88** | 24 |

`->Attack` +1.161 and +0.690 against `ant.ron`'s weight of 2.0, from +0.031
and zero. **The falloff is also the grading** — middle and edge of a fight
read different numbers through the same weight, so the response is a
distribution rather than a binary, free.

**`ALARM_RHO` 0.25 → 0.35 came with it.** Diffusion had been doing a share of
decay's job, so the constant's *"gone in ~150 frames"* was right by accident;
without it 0.25 left a bite audible for 204 frames. Caught by `creature.rs`'s
`the_alarm_forgets_faster_than_a_trail` going **red rather than quiet** —
which passes again untouched, so **no other lane's file was edited.**

**My own guard was found blind and rewritten**: it dialled every arm, so the
shipped default was never under test and reverting the whole change passed;
and it credited termination to the `− fall` contraction when `ALARM_RHO` does
that work. Both faults now go red.

**Not fixed — and it is a trade, not a defect.** No combination of trail
constants reaches a round trip: the best corner of the whole space
(`DEPOSIT` 240 + diffuse every 8 + decay every 4) is **0.64x**, and that is
three stacked behavioural changes. The ceiling is not quantization either —
diffusion at 0.25 costs a one-cell line **16.7% of peak per pass**, so even
at infinite precision an unreinforced trail is gone in ~30 passes.
**Diffusion and trail life are one knob pulling opposite ways**, which is the
owner's call, not a lane's. `set_channel_diffuse` is the dial; a *cadence*
dial is the companion worth building.

**The fix I first proposed for the trail was wrong and is recorded as such**:
a threshold snap-to-zero in `build_decay_lut`. `dead-ends.md` already rejects
the rounding it needs, and it targets the floor when **truncation** is what
caps lifetime at `deposit` passes.

## For Lane E and the open ruling — evidence, before the decision

**Every measurement in this lane was taken on `e01dd1a1` (branched from
`f4e3b471`), i.e. BEFORE E's plant-grazing fix.** Recorded because the alarm
plane's deposit rate is about to move and a rate taken here will not be
comparable. The channel A/B work is unaffected — it is the alarm plane only.

**1. I can confirm E's diagnosis from the other direction, and it corrects a
number of mine.** My Q2 first cited *"336 alarm deposits in a 9,000-frame
bed"* as evidence the alarm writers fire. They fire; the writer is grazing.
Same bed, same seed, same colony, one variable:

| arm | alarm deposits | plane |
|---|---|---|
| `founders=8` (plants) | **336** | live |
| `founders=0` (no plants) | **0** | **never written** |

`pherolife mode=world frames=9000 seed=1 founders=0`

**Not "mostly" — all of it.** Remove the plants and the plane is never
allocated. **100% of alarm traffic in the shipped single-colony bed is the
grazing path.** That is the owner's tree-only-box control reproduced
inside-out: he sees alarms where nothing can attack, I take the plants away
and the plane stops existing.

**2. On the open question — *should eating another creature raise an alarm,
or should alarm mean only "I was attacked"?*** My finding 2 above is the
evidence that bears on it, and it points one way:

**At the shipped constants, nobody beyond touching distance can hear an alarm
however it was raised.** A wound's loudest reading one cell away is **6 of
255** (+0.047 into `Attack` against a weight of 2.0); two cells is **zero,
ever**; even a sustained 40-bite fight leaves an ant two cells off at 0.059
and four cells off at zero for the whole fight. A `DISPLAY_DEPOSIT` is
inaudible to anyone but the displayer.

**So the recruitment argument for keeping eating-raises-alarm buys nothing
measurable today** — there is no colony-scale response to recruit, because
the signal does not propagate. That is an argument for deciding the semantics
on what the owner wants the word *alarm* to mean, not on what it would
achieve, because at present it achieves nothing past the cells in contact.

**If the ruling is that eating a creature SHOULD raise an alarm and that this
should mean something, the constants must move with it** — `ALARM_RHO 0.25`
against `DIFFUSE 0.25` is what confines it, and **both are now dialable**
(`set_alarm_rho` existed; `set_channel_diffuse` landed with this branch).
That is a second change on a seed sweep gating an order statistic, not a
rider on E's fix. Filed as **§Z25**.

**3. `DISPLAY_DEPOSIT`: my baseline is pre-fix and I am saying so.** The
brief asked me to measure it and the coordinator is right that a deposit-rate
baseline taken now is mostly the bug. **What I measured is not a rate** — it
is propagation from a hand-placed deposit, independent of who wrote it, so
the "inaudible to anyone but the displayer" finding survives E's fix
unchanged. Anyone wanting the display's *frequency* must re-measure after E
lands; `pherolife mode=world founders=0` is the arm that isolates it.

**4. `ALARM_RHO` and `ALARM_DEPOSIT` have never been calibrated against fight
traffic**, because in this bed there has not been any — see the table above.
After E's fix a single-colony bed's alarm rate is **exactly zero**, so the
first bed that can calibrate them is a two-colony one with rivalry on.

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
