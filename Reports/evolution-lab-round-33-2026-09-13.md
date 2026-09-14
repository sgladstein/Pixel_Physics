# Round 33 — the parallelism answer is no, and the animals were never resting

*2026-09-13. Coordinator record. Landed #391, #392, #395, #396, #398, #400.*

The round opened on the owner's standing priority: **"the biggest issue is the
performance after the creature numbers get high and that is our #1 priority by
far."** Round 32 had named the one lever worth more than everything else
combined — the creature pass has no parallelism, at ~86% of the frame on one
core. Round 33 built it, and the answer is **no**.

## 1. The creature pass can run across cores. It should not. (#398)

`ParTuning`, `writewatch.rs` and a speculating read phase, shipping
**`ParMode::Off`**.

**It is exact**, and the evidence is not a hash. `par=verify` speculates,
validates, then **recomputes the read anyway and names the brain input that
disagrees** — 0 mismatches at 450 ants. `par=on` and `par=off` hash alike
(`0x2ea88e9292a56d45`) against a deliberately-wrong `unchecked` arm
(`0xfbd67688ba6cc3e7`), with identical counters between the two arms (85.7
creature ticks/frame, 40.4 moves/frame, 9.6% blocked) — so the animals did the
same things, not fewer of them.

**And it costs 3–4% of the frame.** A speculation pays only if
`hit rate x parallel speedup > 1`, and here that is `0.35 x 2.1`. **Both terms
are properties of the box and the bed rather than of the code**, which is why
the mechanism ships wired up and one flag away instead of being deleted:
`antcost par=on,off` re-runs the entire comparison in one command on a machine
with more cores or a bed with a different density.

**So the owner's #1 is still open.** Spreading the existing work wider was the
obvious attack and it has now been measured rather than assumed. The lead that
remains is round 32's: **the creature cost is not linear** — ~0.3–0.5 µs per
ant below ~400 and ~2.6–2.9 µs above ~600. A knee means something specific
happens at a threshold, and finding *what* is a different investigation from
parallelising.

**The instrument outlived the negative result, and immediately earned it.**
When #396 landed a new brain input under this branch, the question "does the
speculation now read stale state?" was answered by `par=verify` in one run
naming zero disagreeing inputs — not by a green hash, which says only yes or no
and cannot say *which read*.

## 2. §Z13 reopened and closed: they were not resting, they could not move (#396)

Round 32 closed §Z13 as *"the marked ants are resting"*. The owner overruled
it:

> ***"How long do they go without asking to move. If they never ask to move
> that is still stuck, it is just because the rest mechanism needs fixing."***

He was right, and the mechanism is arithmetic. An animal's urge to walk is a
weighted sum squashed into `(-1, 1)` and then clamped to `[0, 1]`.
`brain::squash` returns a genuinely **negative** number for a negative sum, so
**every degree of "would rather not" lands on the same exact zero**, after
which the roll can never succeed. The shipped two-cell ant sat at exactly 0.0
for **77.5%** of its decision ticks — worse than the long ant that prompted the
complaint.

**The owner's reading of his own screen was closer than the coordinator's
number.** Shown a pooled idle rate he said: *"95% of the creatures on the
screen are not resting and then a few decide to rest for a large portion of the
entire gameplay… It reads some creatures got frozen."* **A rate cannot tell
that apart from "every animal takes brief breaks" — both produce 75%.**
Re-censused per animal: neither story is exactly right and his is far the
closer — the idle fraction spreads across all ten deciles, and **18.5% and
21.9% of every animal on the bed go quiet and are never once seen elsewhere
again**, with median longest stands of 13,500 and 8,100 frames and a longest of
43,200. They were alive throughout. After the fix: zero, on every seed, while
the median rest bout is **unchanged at 2–4 ticks** and animals still decline to
step on 83–94% of decisions.

The fix is `brain::BrainInput::Stillness` — the square of
`still_ticks / STILL_SATURATION`, wired at 1.5 in all eleven shipped species,
so a lineage can breed itself more or less restless. It **completes** the
owner's 2026-09-09 ruling rather than reversing it: rest is still the absence
of a reason to act, and standing still long enough is now one of the reasons.

It costs foraging on 7 of 9 seeds (median 2.68 → 1.63 deliveries per 1,000
ant-ticks) and **recovers the two seeds where foraging had collapsed
outright**. Starvations fall on 7 of 9. No measurable frame cost.

**`live_slots` 846 → 870 re-derives every species' `mutation_rate`**, so
`origin/main` before `f9dd3295` is not a valid control arm for anything
behavioural.

## 3. Zoom, and a third game that has no opinion about it

The owner ranked zoom second, below performance. #392 ships the sandbox's
zoom-out pixel budget at **x4** and #395 the lab's, from his own blind verdicts.

**A correction the coordinator owed him.** The two round-32 cards offered
**different menus** — the lab card x1/x2/x4, the game card x1 against x2 — so
x4 was never on offer in the game, and his two verdicts read as a preference
for different settings per game. He rejected that reading: *"I am not sure what
questions that I answered that suggest zoom should be different between the
games, but that doesn't seem like what I want."* **A comparison can only return
a verdict about the options it contains.** If two cards will be compared
against each other, give them the same menu.

He then asked whether the change reaches the held world, and ruled that it
should. It does not: `src/bin/druid.rs` runs its own loop, allocates one fixed
`Pixels::new(WIDTH, HEIGHT, ...)`, drives the camera with `Renderer::follow`
and never calls `zoom_within`. The plan is #400
([`held-world-zoom-plan-2026-09-13.md`](held-world-zoom-plan-2026-09-13.md));
its recommendation is to **extract the budget once before writing it a third
time**, since writing it twice is exactly how the lab inherited the sandbox's
discard and none of its fix.

**The open question it surfaced, which belongs to all three games.** The zoom
ladder is `1, 2, 3, 4` and the budget is a power of two, so **rung 3 can absorb
none of it**. For the held world rung 3 spans `1536x960` — the first rung
showing the world's whole height, the rung a player wanting to see where they
are will stop at, and the only soft one of the four. Three options are priced
in that report's §6. **The blind A/B was promised and is not yet posted**:
`zoomout_pixels` holds `scale * stride == SPAN_STRIDE` by construction and
rungs 3 and 4 differ in span, so it cannot express the question; `labzoom` can
once it gains a `budget=` arm, which #395 makes possible.

## 4. What the round cost in coordination, which is worth writing down

**`fire_trigger` returning success is not evidence a lane woke.** Two pokes
were sent to a lane that had been idle and disconnected for two hours; both
calls returned success, `last_run` on the trigger recorded **no run at all**,
and the lane was twice reported to the owner as contacted. The evidence is
`last_run` on the trigger and `updated_at` on the session, not the send.

**Before archiving a session, sweep `list_triggers` and delete every trigger
bound to it — including ones the lane armed for itself.** Lane E was archived
with its own check-in still live; it fired forty minutes later into a gone
session and failed where the owner could see it.

**A stalled lane's finished work is the coordinator's to land.** Two pokes is
enough; after that the branch is resolved and pushed by the coordinator. #395
sat green on its own head for four hours behind nothing but trunk churn.

**Every conflict across six landings was a generated file** —
`.claude/README.md`, README's contents tables, the dead-ends index. **Resolve
them by regenerating, hunk-wise, never `--theirs` on the whole file**, which
takes main's copy entire and silently drops the branch's own prose.

## 5. What round 34 inherits

The brief is
[`evolution-lab-round-34-brief-2026-09-13.md`](evolution-lab-round-34-brief-2026-09-13.md).
