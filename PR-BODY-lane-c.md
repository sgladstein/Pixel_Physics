# Re-baseline the creature record on today's `main`

**What this does.** Every number the creature record quotes was measured
before this week's worldgen work, so none of it was checkable. This
re-measures the standing guards, re-sets the two `forage_reach` bars that had
gone fragile without anyone editing them, prices the one control nobody could
afford to run — and overturns the reading of the colony's headline number.

**Where it sits.** Making the creature line falsifiable again, so the next
stage is built on numbers someone can reproduce rather than on a description
of a world that is gone. Nothing here changes what the simulation does; it
changes what can be checked about it.

## The headline: the colony is not being fed, it is 45% through its startup grant

`ascii`'s foraging scene reports `eats 6`, `deaths 0`, and a food stock going
791,040 → 3,057,600. That was read as *"the world feeds a colony for free —
nothing is hungry, nothing dies, so there is no selection pressure."* The
conclusion holds. The mechanism does not, and the mechanism decides the fix.

The energy ledger printed one line below those counters:

- `granted` **24,300** against `harvested_plant` **720** — food supplied
  **2.9%** of the colony's energy.
- `metabolized + moved + synapses` = **11,373**, so the mean ant ends the run
  at **45% depletion** against `ant.ron`'s `hunger_fraction: 0.5`.

Nothing is hungry because **the run stops just short of the threshold**.
`start_energy: 900` at `idle_cost: 0.10` is a ~54,000-frame idle life; the
scene runs 12,000. `creature_space` independently overrides `START_ENERGY` to
**90.0** — one tenth — precisely so its own runs can reach the death it
advertises. And `food stock` is a canopy-growth curve, not a feeding signal:
it rose by 2,266,560 while the ants took **720** out of it, 0.03%.

**Then `main` moved 30 commits mid-session and tested the claim by accident.**
The plant-organs merge changed the trees, which *are* this scene's food. On
the merged tree:

| | `ba6fc98` | `f96c08d` |
|---|---|---|
| food stock | 3,057,600 | **3,685,920** (+21%) |
| `moved` | 3,165 | **2,507** |
| **`eats`** | **6** | **3** |

**Food abundance rose 21% and `eats` halved.** The abundance reading predicts
the opposite sign. The depletion reading predicts exactly this — less
movement, less depletion, fewer ants across the threshold — and it predicted
the direction of a change nobody planned, on a tree it was not fitted to.
That also disposes of "the counter is broken": a broken counter does not
track a 21% change in movement cost.

## Guards, re-measured

| Guard | 2026-08-23 | re-measured | tree |
|---|---|---|---|
| Frame cost, colony mean over 12,000 frames | 3.488 / 3.491 ms | 2.906 / 2.943 | `ba6fc98` |
| | | 3.362 / 3.417 | `f96c08d` |
| Foraging pays, no moss / moss | +0.474 / +0.466 | **+0.427 / +0.459** | `ba6fc98` |
| Ants fed | 0.75 / 0.75 | **0.68 / 0.75** | `ba6fc98` |
| Reference genomes, `authored` / `zero` | 0.690 / 0.297 | **0.696 / 0.299** | `f96c08d` |
| Determinism, two `ascii` runs | identical | **identical**, both trees | |

All hold. The frame-cost rise is **scene size, not a regression**: the colony
went 126 → 153 live organisms (+21%) while the mean went +16%, so cost per
organism fell. Six runs; the parallel-stress worst-frame proxy swings **77%**
on a fixed binary and scene, so that column is noise, while the mean
reproduces to 1.3%.

**The reference pair not moving is the informative half.** `zero` reads 0.299
across both merges and the economy sweep's `immobile` column reads 0.298 at
all four settings — so the wetland economy scene was barely touched, while
the `ascii` foraging scene lost three quarters of its trips over the same
period. Two creature scenes, opposite answers.

## The `forage_reach` bars had gone fragile without being edited

| | comment said | `ba6fc98` | `f96c08d` | bar was | bar now |
|---|---|---|---|---|---|
| `forage_trips` | "measured 98 here" | 24 | **23** | `>= 14` | **`>= 6`** |
| `forage_depth_max` | "measured 18 here" | 37 | **28** | `>= 8` | `>= 8` |

The trip bar was set at *a seventh* of 98 with a comment warning that a bar
near the measurement flakes. Nobody edited it; the world moved under it, and
at 14 against 23 it sat at 61% of the value while still telling every reader
the measurement was 98. **6 is 23/4.1**, where 4.1x is the largest legitimate
drift on record — another drift that large still passes, and a sessile colony
still scores 0 and still fails.

**The trade is stated rather than hidden: a genuine 2x foraging regression
would now pass this guard.** Also recorded rather than fixed — the scene
generates terrain at a hardcoded `seed: 1`, so this is a single-seed bar over
procedural content.

## `ant_ablation`: it does not buffer, it costs 868 s, and its defaults answer half the question

Header at **0 s**, first arm row at **33 s** — Rust's stdout is a
`LineWriter`. The 600 s kill landed two thirds through an **868 s** run
(predicted 890 s from a two-point fit at ~1.39 ms per arm-run-frame over 20
arms). It now prints its expected cost up front and streams per-arm progress
to stderr, leaving stdout byte-clean for diffing.

**The answer is sharper than "yes".** Ablating `Bias->Move` reproduces the
zero control to every printed digit — one connection of twelve carries the
entire separation. That is also the positive control proving the six
invisible ablations are real nulls rather than a dead instrument. **But
`deliv` and `eats` are 0.0 in every arm at the defaults**, so both `Feed` and
both `Drop` ablations are vacuous by construction; the feeding question needs
`terrain=world food=trees`.

## The parameter-echo work was already done — and the list was stale

All three named defects were fixed on 2026-08-23 by `b6d25c4`, and all three
harnesses already `panic!` on an unknown argument. **The one still broken was
not on the list: `gnome_depth`.** It echoed neither `zoom=` nor `depth=` and
only *warned* on an unknown key. Sharper, its `depth=` arm fell through to
`Weave` for any unrecognised **value**, so `depth=fron` rendered a `Weave`
sheet indistinguishable from the `front` one asked for — the megastudy
failure with the typo moved from the key to the value, and the one form an
echo cannot catch on its own. Both arms now panic; line one names the
parameters. Found only by auditing all five harnesses rather than the three
named.

## Re-verified after the colony-scene fix (`d007c156`)

**No figure here came from `scene=colony`**, so nothing needed re-taking on
that account. But the same merge carried **247 new lines of
`src/sim/creature.rs`** and a `recurrence` term added to
`genome_from_wiring`, so everything was re-measured rather than assumed safe.

Every `ascii` counter came back **byte-identical** (eats 3, deaths 0, moves
8812, pickups 3157, deliveries 1047, trips 23, deepest 28, the whole energy
census), and so did `ant_ablation`'s entire 20-arm table — at **860 s against
the earlier 868 s**, two independent timings 0.9% apart.

**The null was checked, not believed.** Identical output across a change that
touched the file under test is the stale-binary tell, so: `filmstrip
scene=colony` at **its own default seed** — the invocation that panicked
before `d007c156` — runs to completion on the same binary. The build took;
the null is real.

`-Bias->Move` is now named in `ant_ablation`'s doc comment as **the table's
positive control**, with the instruction to read it first. The `zero` arm
only shows the metrics can be low; this arm shows a single instinct driving
them from `authored`'s values to the floor, which is what makes the six
unchanged rows readable as real nulls rather than a dead instrument.

## Gates

`cargo clippy --all-targets -- -D warnings` clean on 1.94.1 **and on CI's
1.98.0**; `cargo test --lib` 1005 passed / 0 failed / 54 ignored;
`bash scripts/docscheck.sh` clean; `examples/ascii` exits 0 on all three
trees with the new bars in place.

## Known gap, left visible

The 37-minute economy sweep is a `ba6fc98` reading — `main` moved between it
and the reference pair. It is the one measurement here I would spend another
37 minutes on next, and it is flagged as such in the report and at the §4
table itself.
