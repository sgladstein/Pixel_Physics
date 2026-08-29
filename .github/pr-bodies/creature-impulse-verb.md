**Creatures can leave the ground, and the body decides what that means.**
One brain output, `Impulse`, and no jump height anywhere: the launch is a
fixed amount of work divided by the body's own mass, and the descent is a
drag law over the body's own bounding box. This is the *motion into the
genome* step — locomotion stops being something the engine does to a
creature and becomes something a genome carries and selection can act on.

Play-facing: [`wiki/ants.md`](../wiki/ants.md). Design and guards:
[`Reports/creature-motion-design.md`](../Reports/creature-motion-design.md).
README's *The ant colony — status* and `PLAN-log.md` both gained a section.

## The decision this reverses, and why that is safe

`step_chain` has refused any step with no footing since two earlier attempts
at airborne creatures each put falls at **59–80% of all moves** (§2d). The
owner authorised reversing it — decision **E11**, *"Yes, they should
cross"* — and §6 call 4, which had stood open, is updated here to record it.

Both earlier failures changed **candidate scoring**, so every ant became
airborne whether it wanted to or not. This adds a separate opt-in path:

* the walk is untouched and still refuses open air;
* `OrganismState::flight` is `Some` only between a launch and a landing, and
  only `creature::launch` ever sets it;
* `ant.ron` authors no weight into the new row, `squash(0.0)` is **exactly**
  0.0, and `impulse > 0.0 && draw.unit_f32() < impulse` short-circuits — so
  the shipped ant takes the same draws in the same order.

## The design rule: no table of creature types

One verb delivers a fixed amount of **work**, not a fixed impulse, so
`v = sqrt(2W/m)`. That is the same `1/sqrt(m)` a muscle's force scaling with
its cross-section gives, and it is why the four shipped bodies stay
distinguishable: at `1/m` they span 4.5x in launch speed and the 6-cell chain
is already immobile, leaving nothing between "shallower hop" and "almost
nothing".

| body | cells | launch speed | terminal speed |
|---|---|---|---|
| `ant` `Chain(2)` | 2 | 2.00 | 1.73 |
| `ant_long` `Chain(6)` | 6 | 1.15 | 1.37 |
| `ant_wide` 5x2 | 9 | 0.94 | 2.04 |
| `ant_block` 3x3 | 9 | 0.94 | **4.74** |

The last two are the whole claim: **identical mass, identical launch, 2.3x
apart on the way down**, the difference being width against height. Played
through the integrator on one plinth over one drop, same seed and same
organism id so both take identical draws, the slab stays up **113 frames** to
the block's **69**.

`Cd`'s two ends (0.5 blunt, 2.0 plate) come from
`rigid::SINK_DRAG_COEFFICIENT`'s own recorded regime check rather than being
invented, and drag is read off the cells **every flight frame** rather than
cached on the species — so six cells strung out flat glide and the same six
coiled do not. `match species` appears nowhere in the path.

**Decision E9's float limit came free**, exactly as §2c predicted:
`buoyant_share` *is* `rigid::drag_through_liquid`'s `carried`, so there is one
buoyancy model in the engine and a body no denser than what it is in has zero
effective weight and hangs.

## What it costs

A flat four walking steps' worth of energy, and the creature's whole turn:
airborne it does not think, eat, dig, steer or deposit, because
`creature_tick` returns before `sense`. One flat price against a
body-dependent benefit is what makes hopping a bargain at 2 cells and worse
than walking at 9 — §1's cost-and-benefit rather than a rule.

## The genome append

`GENOME_LEN` unchanged at 12,352, no weight of any other slot moved — the
second lawful append into the reserve S2 built. `live_slots()` **268 → 288**,
so `random_genome` draws 20 more values and a sampled genome at a given seed
is a *different animal*: the real cost of a live verb, recorded rather than
absorbed. Manifest pin **1,235,247,055 → 717,235,691**. Slot 12 stays
unnamed, per §6 call 2.

## Judged by eye, because no test can answer this one

Review card `20260829T154736312Z-2536dc` — a paired frame sequence of the two
equal-mass bodies, verb off against verb on, with the launch counters in each
item's `meta` (`items[].meta`, which is where the review skill's spec puts
them; the card has no top-level `meta` and a reader looking there finds
`null`). The owner's verdict:

> *"With the new jump is great. I choose B"*

The counters say the mechanism fired (5 launches, 200 airborne frames, 0
falls, against 0 / 0 / 9 in the control arm). That sentence says it is worth
having, which is the claim `CLAUDE.md`'s ethos puts above any individual
mechanic and the one three previous models were overturned on.

## §7's five guards

Every figure taken on this branch merged up to `main` at `3c464c2`, on a
container shared with three other agents. Everything gated is a **counter**;
the one wall-clock claim is quoted with its caveat. Full write-up in the
report's new §7b.

| | guard | reading |
|---|---|---|
| **1** | falls per move | `forage_probe seeds=12 frames=12000 spacing=4`: min **0.208** / median **0.225** / max **0.334** — lane C's pre-verb baseline to three decimals. Blocked per move likewise unchanged at 0.031 / 0.034 / 0.065 |
| **2** | blocked moves | `creature_look mode=live count=40 frames=600`: `ant` **5%**, `ant_long` **4%**, `ant_wide` **41%**, `ant_block` **43%** — `creature-appearance-design.md` §5's table reproduced, rigid 8–10x the chains |
| **3** | `ascii` counters | **1,109 counter lines byte-identical** against `origin/main`; the 146 that differ all carry a millisecond figure |
| **4** | ablatable | `Impulse=lo` reproduces `authored` **to every printed digit**; `-Bias->Move` still reproduces `zero`; `Impulse=hi` moves every column — travelled 71.2 → **331.4**, coverage 1,267 → **3,989**, foraged 0.06 → **0.50**, first pickup 1,208 → **118**, pickups 2.7 → **29.3**, deliveries **0.0 → 8.3** |
| **5** | frame cost | An airborne creature is **6x** its walking scheduler footprint and carries no `eval_brain`. Against `MAX_CREATURE_SITES_PER_FRAME` = **256**: 52 ants walking offer ~9 sites/frame, all-airborne 52. Binds at 256 concurrently airborne against ~1,536 walking — and #118's own comment records the budget bounds work rather than gating a tick, so exhaustion stretches an arc rather than dropping one |

**The bar ships as something that fails.** `forage_probe gate=1` sets it at
**0.40** — above the *worst* seed, not on the median, because which seed is
worst reshuffles on any legitimate change — and it **refuses to run at any
other frame budget**, since the statistic does not settle (0.239 / 0.225 /
0.215 at 6k / 12k / 24k). Checked: at `seeds=2 frames=6000` it exits 2 and
says why.

**And guard 4's last row is a warning as much as a result.** On those columns
hopping is *strictly better*, which §1 says makes every lineage converge on
it. The cost is real — 4x `move_cost` per launch, and no acting while
airborne — but **this scene cannot see it**: `start_energy` 900 at
`idle_cost` 0.10 and `tick_interval` 6 is an idle life of ~54,000 frames
against a 6,000-frame run. That is decision **E14**'s arithmetic. Until the
horizon change lands, *"is the impulse priced correctly"* is not a question
any instrument here can answer, and nothing in this branch claims it is.

## One asymmetry deliberately left standing

There are now two ways to be in the air and they disagree: a launched
creature descends under the drag law at 1.7–4.7 cells/frame, and one that
*walks off a ledge* still falls through `step_chain`'s own branch at one cell
per tick — 0.167 cells/frame at `tick_interval` 6. Unifying them means
editing the walk, which is precisely what got the two earlier attempts
reverted, so it is a decision rather than a drive-by fix.

## Instruments added

* `filmstrip scene=hop` — four body plans on identical plinths over one drop,
  with `impulse=` as its own control, `body=` to attribute a figure to one
  plan, and `shelf=` to trade drop height for legible zoom.
* `report_colony` prints the launch counters and no longer gates on
  `moves > 0` — a creature that only ever launches never walks, and a hop
  sheet with four animals in mid-air printed nothing at all.
* `forage_probe` prints the launch counters beside falls.
* `ant_ablation` sweeps `Impulse` at both extremes (22 arms, was 20). An
  *ablation* arm would be `authored` by construction — there is no authored
  weight to remove — so the sweep is the arm that can fail.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_019RCxVVRPrYbeRtqCmofeUK
