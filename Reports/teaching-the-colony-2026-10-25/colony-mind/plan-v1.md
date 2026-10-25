# Colony mind seat: implementation plan to a playable demo (v1)

*2026-10-25, colony-mind seat thread, branch `claude/project-thread-6xokl9`. Inputs: the design
(`designs/seat-colony-mind.md`), its feasibility review (`colony-mind/feasibility-review.md`: not feasible as
written, feasible as a smaller "colony's voice" demo if trained ants visibly react), and step 0
(`colony-mind/step0-measurements.md`: two of three verbs pass). This plan is for an independent review, then
revision, then the build.*

## What the demo is, in one paragraph

**`cargo run --release --bin lab -- demo=mind`** opens a lab box where a trained colony lives on a food supply that
comes and goes. You have no body. You are the gain on two of the colony's own channels: **the food trail its
foragers lay coming home**, and **the meals its ants pass mouth to mouth**. Hold the mouse on the door and every
forager arriving under your hand lays her trail louder, and more food comes home (measured: +19% to +43% by hold
time). Drag a disc over a chamber and close the stomach there: sharing stops, a few ants stay fat and more go lean
(measured: sharing -95%, spread of hunger doubled). Both cost **the voice**, a meter the colony's own deliveries
refill, so a starving colony has a quiet voice. A third verb, **favour**, marks an ant's line so the next eggs
carry it (through the queen seat's shared who-lays hook). The stake is the colony's boom and bust: on today's
trained ant the box swings between ~10 and ~230 ants and sometimes dies, and the question the demo asks is whether
a player's hand on these two channels can steer it through the famines.

## What is cut from the design, and why (all measured or verified)

| Cut | Why |
|---|---|
| Swell the news (the met-a-returner stamp) | No trained ant reads or writes it (`creature.rs:7998-8022`; review) |
| Hush the door (as designed) | Same stamp. Replaced by **hush the trail**, the swell's inverse, if build step 2 measures it |
| Echo / swallow a cry | Alarm held over a disc moved no ant count on 16 seeds (step 0). Handed to the enemy seat as a finding |
| Warm a spot / shift the nursery | Warmth moved nothing (step 0); minds cannot carry brood (`options.rs:838`) |
| Make the brood beg, spend the reserve | Read only by the hand-built walk / `act()` (review) |
| Habits, quorum, police, step down, hearing chart | Large new work; the chart has no heritable substrate (no mutation in the lab, 30 near-copy genomes) |

## What a player does (the verbs, keys and what they see)

| Verb | Input | Engine effect | What the player sees | Measured |
|---|---|---|---|---|
| **Swell the trail** | Hold left mouse (brush radius `[`/`]`, default 15) | Every **laden** ant whose head is in the brush lays extra food trail at its head each tick: `DEPOSIT x g(t)`, `g` ramping 0 -> 5 over 4 s of hold (game time) | An amber ring on each swelled forager; the food-trail overlay brightens along their path; DELIVERED/MIN climbs on the HUD | `trailnear` 240 f: +62 (12/16 up); 1,000 f: +143 (15/16) |
| **Hush the trail** | Shift + hold left mouse | Food trail under every laden ant in the brush is scrubbed to 0 each tick (new `World` write: a clamp, not a deposit) | The trail behind them goes dark; DELIVERED/MIN falls (if measured) | **Not yet**: build step 2 gate |
| **Close / open the stomach** | Right-drag a disc (C toggles the armed state) | `Minds::closed_stomach` gains a disc; adults whose head is inside are not offered Feed to another adult (larvae still fed). Right-click inside a closed disc opens it | A dashed ring on the disc; with the hunger tint on (H), ants coloured by bank on a fixed dark-to-bright ramp; SHARES/MIN in the disc falls to ~0; the spread bar widens | 3,000 f: feeds -1,054, spread +0.91 tank, lean +4.3 (16/16) |
| **Favour a line** | F, then click an ant | `who_lays` store: that ant's id gets weight +1 (decays by half every 10,000 frames); mix knob p = 0.5 in this demo | Eggs laid after carry a fleck of the favoured line's colour; the LINE SHARE bar shows each favoured line's share of eggs | Not measurable before the queen's hook lands; build step 5 |

**The voice.** One number on the HUD, drawn as a ring at the door. Refilled by each cell delivered to the nest (+1
per cell, cap 400); spent by swell (0.02 per swelled ant per tick) and by each closed disc (0.01 per disc per
tick). At 0, swell does nothing and closed discs open themselves, oldest first. No engine change: it is demo
state. The constants are a first guess, tuned in build step 4 so that a held swell over a busy door empties a full
voice in about 30 seconds at 1x.

## The bed: `assets/lab_scenarios/demo_mind.ron`

`steady_income`'s box and colony (52 trained ants at column 256, no plants, rain off), with income **40 cells
every 1,000 frames for 20,000 frames, then none for 15,000, repeating** (a famine every 35,000 frames). Starts with
`LAB_TICKS`-style warm-up of 15,000 frames so the player opens on a colony that has found the food. The famine
length is a first guess, set in build step 6 so that an unattended colony loses roughly half its ants per famine
on most seeds but rarely dies outright (graded, recoverable).

## Build steps, each with its proof

| # | Step | Proof (a picture and a number) |
|---|---|---|
| 1 | **Launcher stub** (queen seat's shared piece: `demo=mind`, `DemoSeat::Mind`, per-tick hooks, mouse routing). If it has not landed, build on the queen branch's launcher commit | `lab -- demo=mind` opens on the bed; with no `demo=`, `examples/ascii.rs` output byte-identical to main |
| 2 | **Swell and hush** in `src/lab/demo/mind.rs`, applied in `before_tick`. A harness `examples/mindvoice.rs` drives the **demo's own functions** (not a re-implementation) on paired seeds | Deliveries graded by hold (0 / 240 / 1,000 frames), 16 seeds; hush the inverse sign on >= 12/16 or it is cut. GIF of a swell at the door (filmstrip `gif=1`) with the amber rings |
| 3 | **Close/open the stomach** on `Minds::closed_stomach` (hook already built, 6 lines) + the **hunger tint** overlay (fixed ramp, full replace, drawn only while H is on) | Paired: shares in disc -> ~0, spread up, 16 seeds; positive control: a disc placed in empty soil changes nothing. A still of the tint, open vs closed, same seed and frame |
| 4 | **The voice** meter and HUD counters (DELIVERED/MIN, SHARES/MIN in discs, LEAN ANTS, VOICE, LINE SHARE) | The meter falls with use and refills with deliveries in a scripted run; HUD screenshot via `PIXEL_PHYSICS_SCREENSHOT_AFTER_FRAMES` + `LAB_CLICK` |
| 5 | **Favour** on the queen's `who_lays` store (weights by ant id, p = 0.5) + egg flecks + LINE SHARE | Paired: favoured line's share of eggs laid in 20,000 frames vs no favour, 8 seeds; the line share must move on every seed |
| 6 | **The stake**: famine timeline, then four scripted player policies on 16 seeds: none / swell whenever the voice is over half / close the stomach over the nest in famine / both | Headcount over 150,000 frames, extinction count, trough depth. **Stop rule**: if no policy changes trough depth or extinctions beyond the seed spread, say so plainly on the artifact page: the demo is a toy whose verbs work but do not matter, and recommend folding swell into the queen game as a late-stage verb rather than a seat |
| 7 | **Frame cost**: `examples/ascii.rs` worst-frame with the demo off (must equal main) and `labperf` with the demo on, swell held, tint on | Numbers quoted on the PR; swell is O(laden ants) per tick, the tint draws only while on |
| 8 | **Land**: report `Reports/teaching-the-colony-2026-10-25/colony-mind-demo.md` (+ its line in `Reports/README.md`), how-the-ant-works stale lines fixed, `how-we-test.md` rule on paired chaos floors, Artifact page with the GIF and stills, PR to the merge desk | Merge desk CI green |

## Engine changes, all off by default

1. `Minds::closed_stomach` + `options::stomach_closed` (built; an empty Vec in every world but the demo, so the
   shipped lab pays one `Option` read on each adult-feed candidate).
2. A scrub write for hush (`pheromone` clamp on one cell), only if step 2 keeps hush.
3. Nothing else: swell uses `World::deposit_pheromone`; favour uses the queen's shared hook; the voice and HUD are
   demo state.

## How it combines with the other seats

- **Queen -> scout -> mind arc.** The mind opens at ~100 workers on top of the queen game: the verbs are tools,
  not a genre change, and the queen body stays steerable (design's own rule). Favour and the queen's choice of
  whose line lays are the same hook.
- **Keeper's recall head** (`shared/recall-head.md`): if `share_on_feed` lands, a remembered place spreads by
  meals, and closing the stomach becomes a quarantine on knowledge, not only on food. No dependency in v1.
- **Enemy seat:** alarm is not a verb here (step 0); a raider is the natural famine-free stake if step 6 fails.

## Risks

- **One recipe** (swell always, close nothing). The voice cost and the famine are what push back; step 6
  measures whether they do.
- **Swell plateaus** between 240 and 1,000 frames, so holding longer buys less: good for grading, but the curve
  must read on screen.
- **Boom and bust is the bed's, not the player's**: a player may credit or blame the voice for swings the colony
  makes anyway. A NO HAND twin (the same seed, unplayed, its headcount drawn as a ghost line) would answer this;
  it costs a second world in memory and half the speed. Recommended for v2, not v1.
- **Favour moves near-copies**: line share will move, behaviour will not, because lab minds do not mutate. Said on
  screen.
