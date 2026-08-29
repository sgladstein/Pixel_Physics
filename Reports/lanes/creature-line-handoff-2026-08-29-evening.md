# Creature line — evening handoff, 2026-08-29

**Written by `session_019RCxVVRPrYbeRtqCmofeUK`** (08:44–22:00), successor to
`creature-line-handoff-2026-08-29.md` (04:00–08:00). Same contract as that
one: the things a fresh session cannot reconstruct from commits are **which of
my claims were wrong**, **which numbers are stale**, and **what is authorised
rather than discussed**.

**Read in this order:** this file, then `Reports/creature-reproduction-economics.md`
(on `claude/creature-reproduction-economics`, unmerged by design), then
`creature-evolution-plan.md` §0 for E11–E14.

---

## 1. STOP — the one thing to do before building anything

**The reproduction analysis is void until a positive control passes, and it has
not been run.** S6 explains a *null* — births never happen — and nothing has
shown the ceiling model can permit a birth **at all**. Run this first:

```
creature_probe start_energy=200 body_energy=20 threshold=241 hunger=0.9 terrain=world frames=24000
```

`birth_cost` 240, `reproduce_at` 241, ceiling 300. `hunger=0.9` is deliberately
in dead-end territory — it is a control, not a proposal. **If it reports
`births 0`, §§1–5 of the economics report are wrong and the build must not
start.** I could not run it: the perf lane owned the box for timings and a
build would have corrupted its numbers.

## 2. What landed (all merged to `main`)

| PR | what |
|---|---|
| #118 | Ants stop walking once you have dug — creature ticks get a reserved per-frame budget |
| #122 | The predation pre-flight, and the decision **not** to build it |
| #123 | Motion baselines re-taken; a headline corrected the same day |
| #124 | Creatures can leave the ground; the body decides what a jump does |
| #126 | S6 reproduction — built, guarded, and unreachable by arithmetic |

## 3. Authorised vs discussed

**Authorised by the owner, in session (no card ids — E9/E10 have them, E11–E14
do not):**

- **E11** gap-crossing → the impulse verb. Shipped.
- **E12** S6 reproduction. Shipped.
- **E13** predation, as E7's one-file probe. Shipped as a *decision not to build*.
- **E14** the horizon change — *"Yes, let them starve"*. **Authorised, NOT
  shipped**; see §5.
- **The reproduction economics build** — *"this all sounds good. start building
  when ready"*. `birth_grant` plus newborn-starts-small-and-grows.

**NOT authorised — do not start:**

- **`birth_size` as a gene.** The economics report refuses it and I verified
  why: `idle_cost`/`move_cost` are **flat per organism** (`creature.rs` ~1288,
  ~1329) — nothing reads `chain.len()`. **E10's premise that "per-cell
  metabolic cost already prices a longer body" is false in the code**, so size
  would ratchet to one extreme however priced. Needs its own owner decision.
- The nest-larder fork of reproduction (fork 2), until its experiment E3 says
  it is reachable.
- `shade-by-cell-type`, heritable chain length, the fear-scent channel,
  `Strike` as a brain verb (slot 12 stays unnamed per motion-design §4b).

## 4. Things I asserted that were WRONG — do not inherit them

Six, and the pattern is worth more than the list: **every one was a claim
generalised from partial evidence I had not finished reading.**

| I claimed | truth |
|---|---|
| An ant's bank ceiling is **930** | **570.** The diet matched filter pays a neutral gut **120** from a 480 leaf, not 480. I quoted the measured bank (**568**) and the 930 model in the same message and did not notice they disagreed by 40% |
| `food_energy == body_energy` across every edible material | **False.** `flower` 1440, `fruit`/`windfall` 960 against `body_energy` 480. I read the first eight matches alphabetically — all 480 — and generalised. **The pump invariant binds *flesh* only** |
| "Only creatures are slow" rules out a frame-rate cause | **Wrong reasoning.** Creature ticks are quantised (nothing on 5 frames in 6), so they degrade *visibly* while the CA sweep degrades smoothly. I told a lane to drop that hypothesis; it pushed back and was right |
| The scheduler is not starving creatures | **Scene-limited, not general.** True of an undisturbed colony, false once a pick is in the world. I relayed a null as though it were universal |
| A card's counters were missing because top-level `meta` was null | They were in **`items[].meta`**, which is where the spec puts per-item counters |
| A local test result would carry to the current head | It would not — another PR had landed 1,400 lines under it. Caught only because I checked |

I also **pushed a merge without rebuilding** and broke CI: `main` and a lane
both edited one format string, git merged the text cleanly, and the result had
one more argument than slots. A clean text merge is not a working tree.

## 5. Stale numbers and standing traps

- **The horizon change is authorised and deliberately unshipped.** It does what
  it was designed to do (`deaths` 0 → 25, `eats` 16 → 58) and **does not fix
  breeding**: cutting `start_energy` lowers the bank ceiling faster than the
  birth bar. It belongs with the ecology decision, not riding in on a mechanism
  change.
- **Falls-per-move for a hopping species reads backwards.** The ratio climbs
  0.225 → 0.298 while *absolute* falls drop 7,516 → ~1,520, because
  `CreatureStats::moves` counts walking steps only. `gate=1` prints both terms;
  read both. A ratio whose denominator the change moved is not in `CLAUDE.md`'s
  metric-trap table.
- **§13o's "no beetle ever touched an ant" is dead.** Beetles kill ~2 ants in
  52; S5's food model changed under that measurement. The conclusion survives,
  the stated reason does not.
- **`open-bugs-handoff.md` §S is marked closed and is not.** Under sustained
  mining the structural queue is self-sustaining — 5,558–9,080 sites produced
  against 2,000 drained, pending through 62,658. #118 insulates creatures from
  it and deliberately does not touch the cause.
- **Every frame figure this repo has ever quoted excludes `Renderer::draw`.**
  Simulation 18.9 ms against a 39.7 ms redraw — the honest frame is ~59 ms
  (~17 fps) and the render is the larger part 2:1. It is the sky from PR #94,
  and it is a **cliff between `sky_rows` 115 and 120**, not a ramp. Handed to
  the rendering session; see `claude/frame-cost-bisect`.

## 6. Where the work goes next

1. **The positive control in §1.** Nothing else until it passes.
2. **`birth_grant` + newborn-grows-into-its-plan** (economics report fork 1).
3. **Experiment E3** — does the nest-larder fork exist?
4. **The body-size pricing decision** (§3) — E10 rests on something untrue.
5. **E9 water response** — still unstarted. Note its old justification is
   weaker now: starvation already puts carrion in the world, and a corpse
   **emits nothing on either pheromone plane**, so carrion helps a blunderer
   without making anything findable.

## 7. Environment notes that cost me time

- **`SendMessage` does not resolve cloud sessions.** Cross-session contact
  works via `create_trigger` with `persistent_session_id` + `fire_trigger` —
  the mechanism `Reports/session-programs.md` documents. Both handoffs today
  went that way after `SendMessage` failed on ID *and* title.
- **`review.py inbox` filters by author**, so it will not show a lane's own
  card. Read `.git/pixel-physics-review/sync/cards/` and `sync/responses/`
  directly. Per-item counters live in `items[].meta`.
- **`/tmp` scratchpad notes die with the container.** Anything worth keeping
  goes on a branch.
- Container clippy is **1.94.1**, CI pins **1.98.0**. That drift produced two
  red PRs today. `cargo +1.98.0 clippy --all-targets -- -D warnings`.
- **The box is a measurement instrument.** Frame timings are worthless while
  another lane compiles — a byte-identical binary has been recorded 2.42x
  apart on load alone. Run one lane when timing.
