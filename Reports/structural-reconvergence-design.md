# Making the support field converge — a scope for §S

**Status: design, nothing built.** Written 2026-08-25 after §S was measured
and after two attempts at it were withdrawn. It exists because the third
attempt should start from what the first two established rather than
rediscovering it.

Read `open-bugs-handoff.md` §S for the bug and `dead-ends.md`'s scheduler
section for what has already been rejected.

## What §S is, in one paragraph

Of the five production callers of `World::record_disturbance`, only
`World::paint_capsule` pays for a converged pass over what it changed. The
blast, the hammer, the pick and fire burnout all hand the correction to
`structural::tick`'s reactive wavefront, which advances **one cell per five
frames** and is served at `MAX_SITES_PER_FRAME`. At the shipped world size
that costs 200 pick cuts a frame of **30.44 ms against 13.33 ms idle**, with
86.2% of frames over budget and the world materially still. One blast does the
same. The queue does not drain: 117,166 sites still pending 9,000 frames after
a single charge.

## The three pieces, and their true sizes

The first scope of this said "converge, add a delay, amortise" and implied
three comparable jobs. Reading the code says otherwise: **one is large, one is
nearly free, and one is optional at first.**

### 1. Converge the field over what actually changed — the real work

Both withdrawn attempts used `relax_region` over a **box**, and both failed
for the same reason: `relax_region` seeds its boundary from the values just
outside the box and trusts them. Where a charge severs a load path the outside
is stale-low, so the interior converges to a value that must rise again, and
the correction restarts from the box edge. **Growing the box moves the
boundary without removing it** — which is why an 8x box was no better in kind
than a 1x box, only differently wrong.

The shape that has no boundary is the textbook increase-aware dynamic
shortest-path update:

1. **Collect what was destroyed.** Every verb already has a per-destroyed-cell
   hook — `detach_exposed_neighbours` and `schedule_structural_check_around`
   are called at each cleared cell by `mine_swept`, `strike` and the
   explosion. Accumulate those positions on the `World` for the frame instead
   of only scheduling from them.
2. **Invalidate the subtree that routed through them.** A cell is *affected*
   if its support path passed through a destroyed cell. Walk outward from the
   destroyed set through the reverse of the support-parent relation, setting
   each affected cell to `u16::MAX`. Over-invalidating is safe — it costs work
   and cannot produce a wrong answer — so the walk may be conservative where
   the parent is ambiguous.
3. **Dijkstra from the boundary of the invalidated set.** Every cell adjacent
   to it but not in it still holds a correct distance, *by construction*: it
   did not route through anything destroyed. That is the boundary condition
   the box never had.

Cost is O(affected log affected) with the affected set discovered rather than
guessed. The measured affected region for one radius-20 charge is ~63 chunks,
about 250,000 cells — which is why the next section matters.

**Do not build this on `relax_region` as it stood before PR #64.** Its ground
rule differed from `tick`'s, and the first attempt's spectacular result was
that difference rather than convergence (§S2). With the rule now consistent, a
pass can be judged on whether it converges.

### 2. An explicit delay timer — the machinery already exists

Today's delay between "judged unsupported" and "visibly falling" is an
accident of queue saturation. It ranges from about a second early in a session
to **never** once the backlog saturates, and `CHAIN_WINDOW_FRAMES` is 600
frames of deliberate generosity that exists to keep the licence open long
enough for that crawl to arrive. Converging the field removes the crawl and
therefore the delay.

The owner's requirement — *"collapse must be obvious and delayed, so the
player can get supports in first"* — should then be met on purpose:

`StagedFracture` already carries a `next_frame`, and
`advance_staged_fractures` already paces a collapse off it. The failure path
in `structural::tick` currently takes the **nearest slice immediately** and
stages only the remainder at `frame + STRUCTURAL_TICK_INTERVAL`. A deliberate
delay is: stage the *whole* region at `frame + COLLAPSE_DELAY_FRAMES` and take
no slice now.

That is a few lines and one constant. Two things to get right:

- **It must not remove breakage at the point of impact.** `explosion.rs`
  already opens the near joints on the bang frame precisely because *"breakage
  used to arrive 7-15 seconds after the flash"* — the near field is handled
  separately and stays immediate. What this delays is the far-field collapse,
  which is the part that should read as spreading.
- **`CHAIN_WINDOW_FRAMES` becomes vestigial rather than wrong.** Once the
  delay is explicit, the licence window no longer has a crawl to accommodate.
  Leave it alone in the same change; re-derive it separately against the new
  timer.

### 3. Amortising the pass — optional at first, and safe to defer

The withdrawn prototype cost a **440.75 ms single frame**, corroborated three
ways (its own run's mean x frames = 456 ms; an independent run at 367.6 ms; a
dose-response of 6.08 ms x 64 area = 389 ms). A quarter-second freeze at the
bang is not a trade this repo makes, and that cost is independent of how the
region is chosen.

Amortising means processing the invalidation and the Dijkstra in bounded
slices across frames, with the in-progress state on the `World`.

**And it is safe to defer to a second commit**, which is worth saying because
it makes the first commit much smaller: *during* a partial reconvergence the
field is inconsistent — which is exactly the state it is in permanently today.
Partial progress is strictly better than the status quo, so the first version
can be correct-but-spiky and the second can pace it.

## What to measure, and against what

The instruments exist; none of this needs a new harness.

| question | instrument |
|---|---|
| does the queue drain | `SCHED_PASS`, watching `deferred` return to its ~5,400 idle value |
| is it converging or being made immune | the `[struct]` census — `worsened` should fall to near zero, and **`max aux` must stay at its honest value** (2,482 on the blast scene, not 142) |
| what does it cost per frame | `scale_probe phases=1 load=blast:200:1`, whole frame against 31.21 ms |
| what does it cost in the spike | the `blasts` row's worst, against 440.75 ms |
| did destruction change | `scripts/acceptance.sh`, and `anchor_probe` for the anchor rules |
| did it change *how much* | `scripts/blastsweep.sh`'s `FailureCounts` at the order statistic |

**Two traps this work has already fallen into**, both recorded in
`CLAUDE.md`:

- *A cost that vanishes may be work that vanished.* The queue going quiet is
  not evidence of convergence; pair it with `max aux`.
- *Six seeds is not a sweep.* Anything read over procedural content wants
  more, and the order statistic rather than the mean.

## What success looks like

The loaded frame at 31.21 ms is roughly 11.6 ms scheduler, 10.4 ms field,
5.9 ms organisms, 3.4 ms sweep. Converging the field should take the scheduler
back toward its **0.32 ms idle** figure, putting the loaded frame near
**20 ms** — still over the 16.6 ms budget, but no longer degrading with play,
which is the property that actually matters.

Do not promise the budget. The field and the organisms are the next two rows
and neither is addressed here.
