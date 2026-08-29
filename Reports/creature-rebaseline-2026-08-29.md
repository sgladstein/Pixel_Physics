# Re-baselining the creature record on today's `main` (2026-08-29)

**Status: measured.** Every figure here was taken on `main` at `ba6fc98`,
one 4-core cloud container, one session, from release binaries rebuilt from
source before any of it (`cargo build --release --examples`, exit code read
directly and not through a pipe — `CLAUDE.md`'s two stale-binary gotchas
are the reason that sentence exists).

**Why this document exists.** Every number the creature record quotes was
measured before this week's worldgen work (`39e6f36`, *"double the sky,
deepen the soil blanket ~4x"*, landed 2026-08-29). Terrain is the substrate
the whole creature line stands on, so the record's figures describe a world
that is gone. This is the re-baseline, plus the two repairs that stop it
silently happening again.

---

## 1. The headline: `eats 6 / deaths 0` does not mean what it looks like

The `ascii` foraging scene at 12,000 frames on today's `main`, reproduced
bit-identically across two runs:

```
after 12000 frames: 126 live organisms | moves 11031 blocked 462 falls 1629
  | eats 6 pickups 2645 digs 66 drops 2619 deliveries 763
  | nest-visits 1192 deaths 0
  food stock 3057600 energy, of which corpse 0   (791040 at spawn)
```

The reading offered for this was *"the world feeds a colony for free —
nothing is hungry, nothing dies, so there is no selection pressure of any
kind."* **The conclusion holds and the mechanism does not**, and the
mechanism is what the next stage would be built on.

### What the ledger on the next line says

```
energy census: granted 24300  plant 720  corpse 0
               metabolized 5400  moved 3165  synapses 2808  dissipated 0
               live 13648.08 vs ledger 13647.00
```

| quantity | value | share |
|---|---|---|
| Energy **into** the colony — spawn grant | 24,300 | **97.1%** |
| Energy **into** the colony — food eaten (`harvested_plant`) | 720 | **2.9%** |
| Energy **out** — metabolism + movement + synapse tax | 11,373 | **45% of budget** |

**The world is not feeding this colony.** Food supplied 2.9% of the energy
in it. Delete every leaf in the world and the colony's budget moves by
2.9%.

**Nothing is hungry because the run stops just short of the threshold.**
`ant.ron` sets `hunger_fraction: 0.5`, so an ant is not hungry until it is
50% depleted. The mean ant finishes this run at **45%**. `eats 6` is not a
colony declining to eat — it is the six most-travelled ants, who paid the
most `moved`, crossing 50% ahead of the mean. That is what a 45%-mean
distribution with spread predicts, and it is why the number is 6 rather
than 0 or 600.

### The horizon, stated as arithmetic

`ant.ron` is `start_energy: 900`, `idle_cost: 0.10`, `tick_interval: 6`.
Idle life is therefore ~9,000 ticks = **54,000 frames**. The scene runs
**12,000 frames = 2,000 ticks**, or **22% of an idle lifetime**.

The independent confirmation is in `creature_space`, which overrides
`START_ENERGY` to **90.0** — one tenth of the shipped value — precisely so
that its 18,000-frame runs reach the 900-tick idle death its own output
line advertises. At `ant.ron`'s 900 the selection question is not reachable
inside any horizon that harness could afford. **Two harnesses, one story:
the shipped ant's energy budget is an order of magnitude larger than any
scene that measures it.**

### `food stock` is a canopy-growth curve, not a feeding signal

The stock rose by 2,266,560 while the ants took **720** out of it — they
consumed **0.03%** of the rise. That curve is what six growing trees do
with no ants in the world at all.

`ascii.rs`'s own comment beside the counter warns that a cell count "rises
as a stand of trees grows whatever is happening to the animals" — the
failure mode §13m hid behind. Pricing the cells in energy did not remove
that failure mode; it re-denominated it. The number is arithmetically
correct and answers *"is the canopy growing?"*, which is `CLAUDE.md`'s
single worst-recurring failure, and this instance was written by the
session that most recently invoked that rule.

### Why the distinction decides the fix

The abundance reading sends the next session at food supply — thin the
canopy, make the floor scarcer. The measurement sends them at the
**horizon**: this scene cannot show selection pressure at any food level,
because it ends before the animals have spent their startup grant. Making
food scarcer in a scene that stops at 45% depletion changes `pickups` and
leaves `eats`, `deaths` and the whole selection question exactly where they
are.

**The counter is sound.** `eats` is checked against a case known to be
non-zero: `creature.rs`'s `a_feed_weight_with_no_dig_weight_still_eats`
asserts `eats > 0` on a scene built for it, and both `eats += 1` call sites
book to the same ledger term the live identity closes on (delta 1.08 on
13,648). This is not a broken counter — it is a correct counter being asked
a question its scene cannot answer.

---

## 2. The standing guards, re-baselined

`examples/ascii`, two full back-to-back runs, same session, same container,
nothing else running — `CLAUDE.md`'s rule that a timing baseline must be
re-measured on the machine that reports it.

| Guard | 2026-08-23 (`creature-evolution-plan.md` §4) | **today, `ba6fc98`** |
|---|---|---|
| Frame cost — colony scene, mean over 12,000 frames | 3.488 / 3.491 ms | **2.906 / 2.943 ms** (worst 39.6 / 38.7) |
| Determinism — two `ascii` runs diff | identical | **identical**; 0 differing lines with timing rows removed |

Frame cost is down ~17%, which is consistent with `ba6fc98`'s own subject
(*"paint the frame on all cores"*). It is quoted as a **same-session
reading**, not as a comparison against 3.488 — that figure came off a
different container and `CLAUDE.md` forbids the subtraction.

---

## 3. The `forage_reach` bars had gone fragile without being edited

| | `ascii.rs` comment said | **measured today** | bar was | bar now |
|---|---|---|---|---|
| `forage_trips` | "measured 98 here" | **24** | `>= 14` | **`>= 6`** |
| `forage_depth_max` | "measured 18 here" | **37** | `>= 8` | `>= 8` |

The trip bar was set at *a seventh* of 98, with the comment explaining that
"a bar near the measurement flakes". Nobody edited it; the world moved
under it. **At 14 against 24 it sat at 58% of the measured value** — the
exact condition the comment warns about — while still telling every reader
the measurement was 98.

The colony has not gone sessile. It has changed shape: **four times fewer
excursions, each roughly twice as deep** (mean depth 10.3 → 17.4, deepest
18 → 37). Fewer, longer trips over reshaped terrain.

**The new bar and the trade it makes.** 6 is 24/4.1, where 4.1x is the
largest legitimate drift on record — the one documented in the row above.
Another drift as large as the one that just happened still passes, and the
failure the guard is named for (a sessile colony scores exactly 0) still
fails. Lowering a bar weakens it and that is stated rather than hidden: **a
genuine 2x foraging regression would now pass this guard.** The depth bar
stays at 8, which the new measurement turns into 4.6x headroom.

**What neither bar can do, recorded rather than fixed.** The scene
generates terrain at a hardcoded `seed: 1`, so this is a single-seed bar
over procedural content — the pattern `CLAUDE.md` says gets rubber-stamped,
because "a guard over a procedural system has to sweep the procedure". The
spread it is blind to is *seed* spread, not noise: run-to-run the scene is
bit-identical. `forage_probe seeds=N` is the instrument that has that axis;
giving `ascii`'s scene one means editing a CI-gated, heavily contested file
and belongs in its own change.

---

## 4. `ant_ablation`: what it costs, and which question its defaults answer

**It does not buffer its output.** A session killed it at 600 s and read
the silence as everything being held to the end. Rust's stdout is a
`LineWriter`: measured with per-line timestamps, the parameter line lands at
**0 s** and the first arm row at **33 s**. What actually happened is that
the defaults need ~890 s and the kill landed two thirds of the way in.

**Runtime is linear in `arms x seeds x frames`**, at ~1.39 ms per
arm-run-frame — two points, `seeds=1 frames=600` in 14 s and
`seeds=1 frames=6000` in 164 s, with fixed setup cost indistinguishable
from zero. There are 20 arms (control, authored, 6 locomotion sign-sweeps,
12 single-instinct ablations).

| invocation | cost |
|---|---|
| **defaults** (`seeds=5 frames=6000`) | **~890 s, just under 15 minutes** |
| `seeds=1 frames=6000` | ~165 s |
| `seeds=6 frames=8000` (its own doc line) | ~22 min |

**The defaults answer the locomotion question emphatically and cannot
answer the feeding one.** Every arm reports `deliv 0.0` and `eats 0.0`,
because `food=corpse pile` is the finite food source `ascii.rs` records as
producing "2.5 pickups and *zero* deliveries per run". So *"is the authored
brain doing anything, or is it the substrate?"* is answered **yes, on
movement** — and is not answered at all on foraging by the invocation
everybody reaches for first.

It now prints its expected cost up front and streams per-arm and per-seed
progress to **stderr**, leaving the stdout table byte-clean for diffing.
Predicted 890 s from the two-point fit, **measured 868 s** — 2.5% out.

### The answer, at the defaults, on today's `main`

`seeds=5 frames=6000`, 20 arms, 868 s. Abbreviated to the columns that move:

| arm | travelled | commute | coverage | roamed | foraged | pickups |
|---|---|---|---|---|---|---|
| `zero` (CONTROL) | 0.0 | 0.000 | 46 | 0.00 | 0.00 | 0.0 |
| `authored` | 73.6 | 0.043 | 1319 | 0.99 | 0.04 | 4.0 |
| **`-Bias->Move`** | **0.0** | **0.000** | **46** | **0.00** | **0.00** | **0.0** |
| `-TempAboveAmb->Turn` | 73.6 | 0.043 | 1319 | 0.99 | 0.04 | 4.0 |
| `-Bias->Dig` | 73.6 | 0.043 | 1319 | 0.99 | 0.04 | 4.0 |
| `-FoodAdjacent->Dig` | 73.6 | 0.043 | 1319 | 0.99 | 0.04 | 4.0 |
| `-AtNest->Drop` | 73.6 | 0.043 | 1319 | 0.99 | 0.04 | 4.0 |
| `Caution=hi` (added, not ablated) | 66.7 | 0.039 | 1246 | 1.00 | 0.07 | **40.8** |

**The brain is doing something, and one connection of twelve is doing
almost all of it.** Ablating `Bias->Move` reproduces the zero control *to
every printed digit* — travelled 0.0, coverage 46, `first-pickup` never.
That is also the positive control this table needs: the metrics can reach
the floor, so the identical rows elsewhere are real nulls and not a dead
instrument.

**Six of the twelve ablations are invisible.** `-TempAboveAmb->Turn`,
`-Bias->Dig`, `-FoodAdjacent->Dig`, `-AtNest->Drop` and both `Feed` arms
return `authored`'s row unchanged on every locomotion column. Two of those
are vacuous *by construction*: with `deliv 0.0` and `eats 0.0` in every
arm, a `Feed` or `Drop` instinct has nothing on this scene it could
express. This is `CLAUDE.md`'s "a change that moves nothing is different
evidence from one that moves a little" — the condition keyed on is
degenerate, and the right response is a scene where food is deliverable,
not a conclusion that the instincts are dead.

**And the authored genome is not near an optimum here.** Three of the six
locomotion sign-sweeps beat it on participation, `Caution=hi` by **10x on
pickups** (40.8 against 4.0) while travelling slightly less. Recorded as a
reading, not as a proposed retune: this is one scene, five seeds, and the
food source is the one that cannot close a foraging loop.

---

## 5. The parameter-echo work was already done — and the list was stale

All three named defects were fixed on 2026-08-23 by `b6d25c4`, whose commit
message quotes the *"4 beetles against `BEETLES = 9`"* line as the thing it
was fixing. All three harnesses also already `panic!` on an unknown
argument.

**The harness that is still broken was not on the list: `gnome_depth`.** It
takes `zoom=` and `depth=` and echoes neither, and its unknown-argument arm
was `eprintln!("ignoring unknown argument")` — a warning a redirect loses.
Sharper still, its `depth=` arm fell through to `TreeDepth::Weave` for any
unrecognised *value*, so `depth=fron` silently rendered a `Weave` contact
sheet indistinguishable from the `front` one that was asked for. That is
the megastudy failure with the typo moved from the key to the value, and it
is the one form the "echo your parameters" rule does not catch on its own —
the echo would have said `depth=Weave`, truthfully, to a reader who typed
`front`. Both are fixed here: the value arm panics, the key arm panics, and
line one names `zoom`, `depth`, the widths and the offsets.

**The method point outlives the three rows.** A defect list is a claim about
the past. Auditing all five harnesses in `instruments.md` §Creatures cost
one grep each and is the only reason `gnome_depth` was found.
