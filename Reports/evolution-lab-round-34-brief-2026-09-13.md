# Round 34 brief — find out what happens at the knee

*2026-09-13, left by round 33's coordinator. Read
[`evolution-lab-round-33-2026-09-13.md`](evolution-lab-round-33-2026-09-13.md)
first; it is short.*

## The one thing that matters

The owner's standing priority is unchanged and now has one attack fewer:

> ***"The biggest issue is the performance after the creature numbers get high
> and that is our #1 priority by far."***

**Round 33 removed the obvious answer.** The creature pass *can* run across
cores — exactly, verifiably — and it **costs 3–4% of the frame** on a four-core
box, because `hit rate x parallel speedup` is `0.35 x 2.1`. It ships
`ParMode::Off`. Do not re-propose parallelising the existing pass without first
re-running `antcost par=on,off` on the machine in question and showing the
product exceeds 1.

**So the lead is the knee.** Round 32 measured that an ant costs **~0.3–0.5 µs
below ~400 of them and ~2.6–2.9 µs above ~600** — a factor of six or more,
across a threshold, in a cost that is otherwise *diffuse* (no single term above
31% of a decision's 57,314 instructions). A diffuse cost that suddenly
sextuples is not a diffuse cost getting slower; **something specific changes
state at that threshold**, and nobody has looked.

### Task 1 — find what changes at the knee, before proposing any fix

Candidates worth ruling in or out, cheapest first. None has been tested:

- **Cache behaviour.** The world at 600 animals may have crossed out of a
  level of cache that it fit inside at 400. This is the hypothesis the diffuse
  profile most suggests, and it predicts the knee moves with *bed size* as well
  as animal count — which is a cheap discriminator.
- **The active-site scheduler.** More animals means more awake chunks; a
  scheduler whose per-frame set grows superlinearly would show exactly this.
- **Contention.** `parallel.rs`'s checkerboard has fixed parallelism; a
  counter downstream of it is not load-independent (`CLAUDE.md`). Pin
  `RAYON_NUM_THREADS` for anything you compare.
- **Density rather than count.** 600 animals in the shipped bed and 600 spread
  over a wide bed are different worlds. If the knee tracks *density*, it is a
  neighbourhood-query cost and not a population cost at all.

**Method, and it is not optional here.** Round 32's own instruction: calibrate
any creature-cost harness **above 800 ants** — below the knee it reads ~0.3 and
looks like a broken harness. And `CLAUDE.md`'s recurring failure applies
directly: **run the positive control.** Construct the case whose answer you
know is non-zero and check the instrument reports it, *and* the case you know
is fine and check it stays quiet. A tidy first number here is evidence of an
artifact, not of a strong effect.

**Do not skip to a fix.** The round-33 record exists partly because
parallelism was built before anyone asked whether it would pay. Size the
problem at the moment it starts.

## Task 2 — the rung-3 card, which is owed to the owner

The zoom ladder is `1, 2, 3, 4` and the pixel budget is a power of two, so
**rung 3 absorbs none of it**. For the held world rung 3 spans `1536x960` —
the first rung showing the world's entire height, and the only soft rung of
four. Three options are priced in
[`held-world-zoom-plan-2026-09-13.md`](held-world-zoom-plan-2026-09-13.md) §6:
leave it, drop 3 from the ladder, or allow a non-power-of-two budget.

**This is a look question and he should judge it, not be told.** Round 33
promised the card and could not build it honestly:

- **`zoomout_pixels` cannot express it.** `Arm::new` holds
  `scale * stride == SPAN_STRIDE` — the constant-reads invariant that is the
  whole reason that instrument can attribute a delta to per-pixel work. Rungs 3
  and 4 differ in *span*. **Do not "just add an arm".**
- **`labzoom` can, and #395 made it possible.** It already renders one tile per
  rung at the real 512x320 viewport with VOID and MID beside each. It predates
  `Lab::pixel_budget` and needs a `budget=` arm over its existing step loop.

Then post rung 3 soft against rung 4 sharp, blind, and let the verdict wait.
**Give both panes the same menu** — round 33's own correction, after two cards
with different menus made his verdicts look like a preference for different
settings per game.

## Task 3 — the held world's zoom, once the ladder is settled

#400 is a plan, not a build. Its recommendation: **extract the pixel budget
into one shared type before writing it a third time.** `App` and `Lab` each
hold the same `pixel_budget` / `viewport()` / `cycle_pixel_budget()` /
`resize_buffer` shape, and writing it twice is exactly how the lab inherited
the sandbox's 15-in-16 discard and none of its fix. The constraint that rules
out the obvious shape is in `Renderer::pixel_scale_for`'s own doc: its stored
copy is stale **by design**.

The held world reaches **rung 4** (measured), so the budget buys the full fix
there. It needs a zoom control — it has none — plus the HUD scale and its new
key added to `KEYS`, whose test asserts every bound key is named on screen. It
does **not** need cursor conversion (keyboard-only) or ring arithmetic
(`world_to_screen` already answers in buffer pixels).

Files belong to the held-world line; check the open PR list rather than this
paragraph.

## Standing facts that will cost you time otherwise

- **`main` before `f9dd3295` is not a valid behavioural control arm.** #396
  moved `live_slots` 846 → 870 and re-derived every species' `mutation_rate`.
- **The full `cargo test --release` exceeds the 600 s Bash cap.** Split it:
  `--lib`, then `--test worldgen --test determinism`. `--lib` alone cannot
  reach `tests/*.rs`, where the registry-sweep guards live.
- **Every branch merging `main` conflicts on the generated `.claude/README.md`.**
  Regenerate with `python3 scripts/contextbudget.py --write`; never pick a side,
  and never `--theirs` a whole README.
- **`fire_trigger` returning success is not evidence a lane woke** — read
  `last_run` on the trigger and `updated_at` on the session. **Before archiving
  a session, sweep `list_triggers` and delete every trigger bound to it**,
  including ones the lane armed for itself.
