# Making the world four times bigger: where round 7 got to

*Written to be picked up cold. You need neither the conversation that
produced it nor any plan file. Branch: merged to `main`.*

## What this round was for

The owner looked at round 6's output and rejected most of it, in these words:
*"everything needs to be bigger, the whole world, the caves. You cannot
create good looking crystals or stalagmites and stalactites that are only 1-2
pixels wide. So many of these tests were minor tweaks, but I think we need
bigger changes."*

Two diagnoses came out of that, and they are the frame for everything below.

1. **Features are too few cells across to have a shape.** A stalagmite is 3-8
   cells wide, a residual spire ~6. At that size there is no room for a
   silhouette, a taper and an interior, so it renders as a scratch or a slab
   whatever rule produced it. The knob is cells-per-feature, not tuning.
2. **The rejected things are drawn primitives; the things nobody complains
   about emerge from processes.** In the same strip where the spires read as
   slabs, the mesas and benches behind them are fine — and those come from
   erosion acting on strata bands (`worldgen/erosion.rs`), not from a drawn
   shape. The rejected list is the drawn list: a cone (speleothems), a column
   primitive (`residual.rs`), a thresholded Worley lattice (the cave web).

**Owner's decisions**, which are settled and not for re-litigating: bigger
world at the current resolution first (higher resolution later); **4x linear,
8192 x 2560**; build *both* candidate cave shapes and compare; rebuild
residuals as a process; a loading screen rather than generating around the
camera.

## Round 7 did the performance work. That is done.

The world could not grow until a big one was affordable. Measured, paired, at
8192 x 2560:

| | before | after |
|---|---|---|
| generation | 11 813 ms | **6516 ms**, behind a loading screen |
| peak RSS | 539 MiB | **358 MiB** |
| field, amortised over a day/night cycle | 30.4 ms | **16.7 ms** |
| field, worst frame | 132 ms | **72 ms** |
| *shipped 2048x640, amortised* | 7.96 ms | **4.90 ms** |
| *shipped size, worst frame* | 31.6 ms | **12.9 ms** |

**The one target not met**, recorded as a gap rather than relabelled: ≤4 ms
amortised at 4x. It is not reachable by more of the same. `step_diffusion` is
now the largest single pass (~11-14 ms on a sky-step frame) and is *not*
skippable the way the other two were — it is what bleeds light sideways so
shade is soft rather than a hard stencil at field resolution, and a canopy's
whole appearance rests on it. The next honest move is either to make
diffusion cheaper per tile or to accept ~17 ms and revisit after the world
grows.

`Reports/field-settling-2026-08.md` has the whole story, including two
variants that **measured better and were rejected** with their numbers.

## One question waiting on the owner

**A gust currently spreads further at noon than at midnight.** With the sun
up, sky-woken tiles across the whole world are pressure-stepped, so a gust
relaxes through all of them; with the sky flat it advances one tile per
frame, which is what the halo design says a disturbance should do. Making the
two agree is worth **~26 ms of a 54 ms sky-step frame at 4x** — several times
what the change that did land banked.

It changes how wind moves, which the player sees in smoke and leaning trees,
so it is a look-at-it decision rather than a performance one. The rejected
per-tile variant and its measurements are in `field-settling-2026-08.md`
(pressure diverging 11.04 against a 0.01 settle epsilon — that is the *size*
of the behaviour change, not an error bar).

## The instruments, and what each is for

| | Answers |
|---|---|
| `examples/field_cost.rs` | what a frame costs, bucketed by sky-step and gusting |
| `FIELD_PASS=N` | which of the eight field passes the cost is in |
| `FIELD_DRIFT=N` | which *channel* is unsettled, and which of three seeds woke each tile |
| `field::field_hash` | is this build's field bit-identical to that one's |
| `field::field_channels` + `FIELD_DUMP=<dir>` | if not, how far apart, per channel |
| `FIELD_CARRY=0` / `FIELD_SKYFAST=0` | run the unaccelerated baseline in the same session |
| `examples/scale_probe.rs` | generation time, peak RSS and settled cost per world size |
| `PASS_TIMING=1` | wall time per worldgen pass |

And the one that changes what "verify live" means: **the real binary runs
headless.** Recipe in `CLAUDE.md`; it is seconds per frame, so it is for
looking at a frame of the actual app, never for timing.

## Four traps this round paid for, as rules

- **Never quote a field mean over less than a full day/night cycle.** The sky
  is a 3600-frame oscillator and the wind is a slower one. Three 600-frame
  windows on the *same* world, differing only in start frame, measured 0.00,
  4.98 and 7.04 ms/frame — each offered as "the settled cost".
- **A whole-world hash can say "identical" about a real bug.** It did:
  worldgen leaves every chunk dirty, so the broken path never ran in the
  scene being hashed. Every hand-built test scene hit it instantly. A hash is
  necessary and not sufficient.
- **"Did it fire at all" needs a counter beside the timing.** A pass costing
  0.00 ms because it was skipped and one costing 0.00 ms because it was fast
  look identical. The first version of the momentum skip never fired once and
  the timings looked completely normal.
- **A cost that tracks the frame number is not necessarily a function of the
  frame number.** A "~4500-frame load transient" was written up twice, with a
  planned fix aimed at generation. It was the wind: `weather::at(seed, frame)`
  is a pure function of the frame, and seed 1 opens on a gale.

## What is next, in order

**Phase 2 — the world gets big.** `WORLD_WIDTH`/`HEIGHT` in `app.rs` to
8192 x 2560, and then the actual work: **re-derive every worldgen dimension
with it.** Cave envelope bounds, formation widths and heights, boulder and
residual sizes, strata thickness, pocket lens sizes, talus and brow reaches.
A feature that stays the same size in a 4x world has become 4x less
significant, which is the complaint that started the round.

Also: the pass margins stated in cells (`vaults` 224, guarded by
`a_cave_cannot_reach_past_its_declared_margin`; `talus` 200, `residuals` 80,
`brows` 40) must be re-derived, and the guard must keep asserting against
constants rather than literals. `MAX_TOTAL_REGIONS = 64` (`region.rs`) clamps
a 4x world that wants up to 80, silently widening every region. And several
doc comments still describe a 512-wide world and a 3x6 gnome.

**Expect Phase 2 alone to look worse.** It makes the cave honeycomb *larger*,
not better — every cave in the game is currently the same ~8x4 Voronoi cell
because `CaveEnv::cell()` scales the lattice with the envelope, so a big cave
is a literal zoom of a small one. Phase 2 and Phase 3 are judged together, or
the first strip after Phase 2 reads as a regression.

**Phase 3 — cave shape.** Two candidates, built and compared on a blind A/B
at play zoom: warp the lattice (sample Worley at `(x + A*fbm, y + B*fbm)`,
vary the threshold along a slow field, and decouple aspect from the size
draw) versus carve by process (dissolution following the strata and the water
table — the only candidate that can produce the *chains* the owner asked
for). `Purpose::CaveVariety = 29` is reserved and unused for exactly this.

**Already refused — do not re-propose:** discs around feature points; a
second `F3 - F1` threshold (*"buys size, not drama"*); the lattice-trio
retunes at either extreme; growing the envelope with the cell held fixed;
Wave Function Collapse (determinism); relocating a rejected placement.

**Phase 4 — residuals as a process.** Turn the current pass off first
(`residual_density: 0.0`) so the world stops showing slabs while the
replacement is built; then mark the rock harder in the plan and let
plan-space erosion carve the land away around it, which inherits talus and a
broken profile for free. `HardnessField` already exists per strata band. B1
measured that erosion alone never *reaches* residual height, so the hardness
contrast and the erosion budget need re-deriving together — and **do not
retune the erosion rate constants**, which were set by eye over a whole
session, without saying so explicitly.

**Phase 5 — the halo far-field.** The owner accepted the glow fix and said of
what is left: *"the faint blocking further out from the big halo still
bothers me. Fix it."* Beyond `NEAR_GLOW_RADIUS = 14` the coarse field alone
carries the halo, and it is an 8-cell grid whose emitter was quantised before
diffusion, read back bilinearly — a smooth pyramid over 8-cell samples. Let
the analytic per-emitter term carry the whole halo out to where it fades
below perception. Must not break
`a_settled_glow_does_not_rebuild_its_halo_every_frame` or
`dirty_rect_skip_is_pixel_identical_to_a_full_redraw`.

## Verification, for anything touching worldgen

A guard over a procedural system has to sweep the procedure: **16 seeds, and
gate an order statistic (p90 or max), never a single seed.** Outcomes here are
chaotic in the seed, so which one is worst reshuffles on any legitimate change
and a per-seed baseline gets rubber-stamped. Build the sweep *before* changing
a model that governs procedural content — twice in one session, a change was
green on all eight acceptance scenes and ate fifty times more world than the
bug it fixed.
