# Worked cases behind `CLAUDE.md`'s Method rules

**Status: living record.** The operative rules live in `CLAUDE.md`'s Method
section; this holds the *evidence* each was derived from — the tables, the
controls, and the mechanisms — for the same reason
[`concurrent-sessions.md`](concurrent-sessions.md) and
[`measurement-under-contention.md`](measurement-under-contention.md) §7 exist.

Split out 2026-09-05, when `CLAUDE.md` went from 26,314 to 21,390 always-loaded
tokens. **Every rule and every headline number stayed in `CLAUDE.md`**; what is
here is what a session needs only once it is actually inside the manoeuvre.
Nothing below is new.

**Why the number stays with the rule and only the derivation moves.** This
repo's own record is that a rule behind a pointer stops being followed — the
branch-drift rule asked for a check by convention and the drift happened
anyway, so it became a `SessionStart` hook; `docgrep.py` replaced a discipline
the documentation audit found "does not survive a real session". A claim that
cannot be checked where it is read is a claim that gets skipped.

Current as of: 2026-09-05.

## 1. A cascade censused before it settles

### 1a. What `seedsweep.sh`'s default budget misses

Measured on `scene=worldcrack strike=12`, one build, eight preset/seed pairs,
frame 1,202 (the default's last tile) against frame 3,602:

| | rock destroyed @1,202 | @3,602 |
|---|---|---|
| `terraced 1` | 557 | **1,042** |
| `terraced 7` | none — rock *gained* 647 | **260** |
| `flat 1` | 20 | **199** |
| `rolling 7` | none — rock *gained* 223 | **88** |

Four of the eight destroy rock by 3,602. **The default misses two of them
outright** — it reads rock as net *gained* where the collapse has not yet
arrived — and understates the two it does see, by 1.9x on `terraced 1` and
**10x** on `flat 1`. `terraced 7` reverses outright: −634 cells lost at 1,202
becomes +326 at 3,602.

`terraced 7` reverses outright: −634 cells lost at 1,202 becomes +326 at 3,602.

### 1b. `rock` plateaus; `cells lost` never settles

**Read `rock`, not `cells lost`, for the settling question.** `rock`
plateaus — `terraced 1` runs −952, −1,042, −1,042, −1,052, −1,052 across
frames 1,802 to 9,002 — while `cells lost` never settles at all: the same run
goes 849 → 1,109 → 745 → −126 → −1,322.

### 1c. The drift is the water cycle, and the control that shows it

**That drift is an oscillation, not accumulation, and it is not the cascade.**
An earlier version of this section blamed it on weathering accruing rubble.
The control that settles it is the same scene with **no verb at all**: at
`strike=0`, `terraced 1` reports **zero failures and `rock +0` at every tile**
while `cells lost` swings 0 → 290 → 471 → 44 → −725 → −1,684. Nothing broke,
so no rock became rubble; the rubble census is simply riding something
periodic, and on `wetland` the `rock` column matches the frozen-water count
exactly — `rock +833` against `833 frozen` — which points at the water cycle.
Amplitude is about ±1,700 cells, **larger than most damage figures in the
sweep**, so a `cells lost` reading taken at any single frame is that frame's
phase plus the damage, and the two are not separable. This is the
*divide-the-oscillator-out* problem below, not a too-short budget — and until
it is divided out, `cells lost` cannot be used to compare two models on these
presets at all.

### 1d. Two diverging runs are different worlds

Worse, and the reason this needs its own heading rather than a footnote: two
runs that diverge on one frame are **different worlds** by the next, so a
single cascade scene cannot compare two models at all, settled or not. One
term measured *ten times worse* on `scene=worldcrack strike=12` and nearly
halved the worst case over 24 seeded runs. Comparisons of cascades belong in
`seedsweep.sh`, run to rest, read at the order statistic.

## 2. Ask what your number counts when nothing is wrong

### 2a. The five instruments, and how each lied

| instrument | how it lied |
|---|---|
| a **metric** | the whisker hunt defined a "film" as water with air above and below — *what falling water looks like* — so it counted every droplet in the world |
| a **counter** | 200 cuts reported against a flat queue; the counter counted **calls**, the harness aimed at soil, and 23 swings removed **0** cells |
| a **timing** | three 600-frame windows on the same world gave **0.00, 4.98 and 7.04 ms/frame**, each offered as "the settled field cost" — it was the wind |
| a **difference** | `extra lost = 0`, comparing two things that had both not happened |
| a **census** | counted every `Solid` in the world rather than the platform under test |

### 2b. The four that could not have answered

Six numbers in one session were arithmetically correct, plausible, and about
the wrong thing. Two are the counter and the census above, seen from the other
side — they did not merely count the wrong thing, they *could not have moved*.
The rest:

| what was measured | why it could not answer |
|---|---|
| a flat platform's damage | no span, so no load to concentrate, so no support rule could matter |
| a queue going quiet | means "converged" *or* "made immune", and queue depth cannot tell them apart |
| an A/B whose arms differed in two things | the paint path, not the rule under test, carried half the effect |
| six seeds | 1.64x; the next twelve gave 1.08x and the pooled median was **zero** |

## 3. A cost that vanishes may be work that vanished

The §S backlog — a blast leaving the structural scheduler pinned at its cap
for ever — was attacked with a converged relaxation pass over the damaged
region.

It reads as a complete fix and it was an artifact:
`relax_region` anchors any cell resting on loose ground at distance 0
outright, where `tick` takes that root only as a last resort, so the pass had
rooted the whole blast neighbourhood flat and the structural system simply
had nothing left to say about it.

The control that settled it: one env switch putting `relax_region` back on
`compute_world_distances`' bedrock-only rule, changing nothing else. The queue
came straight back to baseline in a single run.

## 4. An isolated harness overstates what the app will see

Measured 2026-08-26 on the field's
momentum passes. A gate that skipped them for tiles whose neighbourhood
provably could not give them momentum removed **91% of that work** — 1,497
solved tiles down to 147 — and the per-pass timings moved a long way with it:
pressure 0.92 → 0.39, velocity 2.87 → 1.11, advection 3.25 → 1.49, the field
step 14.50 → 9.87 ms. It was bit-identical, and it made the frame **slower**:
eight alternating paired runs of two fixed binaries put the difference at
**+0.59 ms, slower in 7 of 8**. The gate's own bookkeeping was timed and is
not the answer (0.15 ms amortised). What was left is that the skipped passes
had been *touching every solved tile*, and the full-set pass that runs after
them then paid the cold misses instead — the arithmetic went away and the
memory traffic only moved. On a `HashMap` of tiles walked by pointer-chasing,
the traffic is the cost.
