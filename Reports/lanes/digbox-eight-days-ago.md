# Lane note — digbox, eight days ago

**Lane:** `claude/digbox-eight-days-ago` · **Coordinator:** `session_01D7J7soFJPZxyLNKvdX8SbH`
**State 2026-09-19: done. Report landed, six images landed, PR open. Head SHA below.**

## The question, and the answer

*Did the colony's digging behaviour change in the last eight days?*

**Yes — and specifically the rate changed while the destination did not.**
Digging is 1.39–1.77x higher at every stop and **2.11x at matched colony
size**, but `roofed+open` converges to 319 against 325 by frame 30,000. The
newer colony reaches its full standing void by frame 10,000 and then plateaus
and erodes; the older one climbs steadily and arrives at the same place at
30,000. What stays apart at the end is occupancy and haulage, not void.

Full write-up, both tables, the six images and the candidate commits:
[`Reports/digbox-eight-days-ago-2026-09-19.md`](../digbox-eight-days-ago-2026-09-19.md).

## What was run

```
digbox ants=1200 rate=8 w=400 soil=80 frames=30000 stops=10000,20000,30000 out=<path>.png scale=2
RAYON_NUM_THREADS=1        # both arms
```

- **then** `ccaef282` (2026-09-11), `examples/digbox.rs` ported back
- **now** `50bacc64` (2026-09-19), head of `claude/sweet-tesla-ommknn`

Both arms ran in detached worktrees with their own `CARGO_TARGET_DIR`. The
selftest passed on the ported harness **before** any number was quoted — all
four arms, both polarities.

## Things a later session should not have to rediscover

- **The "now" arm reproduced the coordinator's reference figures digit for
  digit** at 10,000 and 20,000 on a different machine. That is the positive
  control that the binary is the tree it claims to be; it also rules out the
  stale-binary trap, which nothing else here would have caught.
- **`sweet-tesla` is 8 behind `main` and all 8 are under `Reports/`.** So
  running "today" on `sweet-tesla` *is* running `main`'s behaviour. Its two
  `src/` additions are env-gated and both switches were unset.
- **The port is pure subtraction**: 235 lines removed, 4 reworded, and the
  census / builder / trickle / run loop are byte-identical between the arms
  (diffed, not assumed). The diff is committed at
  `Reports/data/digbox-port-to-ccaef282.diff`.
- **No `src/` change was needed on either tree.** The brief asked me to stop
  and report if one were. None was.
- **`BrainInput` gained `Stillness = 29` in the window.** `BrainOutput` is
  identical. That widens the ant's genome and is a difference between the two
  engines rather than anything I stripped — the strongest single candidate,
  with `ec1dffdd` *"an animal that stands still gets restless"*.
- **`World::room_surface_at` is absent at `ccaef282`** — a real addition in
  the window — but `digbox` never calls it, so no strip was needed for it.
- `CreatureStats::{dig_rolls, at_nest_crowding}` are both absent at
  `ccaef282` and both are pure `+= 1` instrumentation today (checked at their
  write sites, `creature.rs:8573` and `:4143`), so dropping them changed
  nothing measured. The cost is that the older arm has no `per_roll` column
  and no crowding histogram.
- **`ants` is not matched between the arms** (417 vs 462 at 30,000, 484 vs
  681 at 10,000) despite an identical trickle, so `digs` is not per-capita.
  Per surviving ant the gap is ~1.26x at both ends. The selftest's
  `found_colony_of(30)` arm is the matched-size control and it agrees.

## Not done, deliberately

**Attribution.** §6 of the report lists the candidate commits and stops there,
per the brief. The window is ~70 commits over
`src/sim/creature.rs src/sim/update.rs assets/materials/`.

## Head SHA

`a41646e9` — the report, the six images, both logs, the port diff and the index line.
