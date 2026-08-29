# Creature line — evening handoff, 2026-08-29

**From `session_019RCxVVRPrYbeRtqCmofeUK`** (08:44–23:00), successor to
`creature-line-handoff-2026-08-29.md` (04:00–08:00).

## 0. How to use this file

**Sections 1–5 are measured. Section 6 is my opinion and you should form your
own.** I have been in one subsystem all day, which makes me well informed about
the creature line and *badly* placed to judge what the project needs next. Read
the sources and decide for yourself:

- **`CLAUDE.md`** — the method. Its *By topic* table maps subsystem to owning
  section; do not read `README.md` whole (~46k tokens).
- **`PLAN.md`** Contents, and the dated *(State …)* line in any handoff section.
- **`Reports/README.md`** — the index, with each report's standing.
- **`Reports/open-bugs-handoff.md`** — its generated status index is the first
  table; read that, then only the sections for your area.
- **`Reports/dead-ends.md`** — grep the **mechanism** you are about to build,
  never your subsystem (~97k tokens).
- `creature-evolution-plan.md` §0 for decisions E1–E14.

**If your reading of the priorities differs from §6, follow yours.** The one
thing I would ask you not to skip is §1, because it is a gate rather than a
preference.

## 1. STOP — the gate, not yet run

**The reproduction analysis is unproven until a positive control passes.** S6
explains a *null* — births never happen — and nothing has shown the ceiling
model can permit a birth **at all**:

```
creature_probe start_energy=200 body_energy=20 threshold=241 hunger=0.9 terrain=world frames=24000
```

`birth_cost` 240, `reproduce_at` 241, ceiling 300. (`hunger=0.9` is deliberately
in dead-end territory — a control, not a proposal.) **If it reports `births 0`,
the economics report's §§1–5 are wrong.** I could not run it: the perf lane
owned the box, and a build would have corrupted its timings.

## 2. What landed today

| PR | what |
|---|---|
| #118 | Ants stop walking once you have dug — creature ticks get a reserved per-frame budget |
| #122 | The predation pre-flight, and the decision **not** to build it |
| #123 | Motion baselines re-taken; a headline corrected the same day |
| #124 | Creatures can leave the ground; the body decides what a jump does |
| #126 | S6 reproduction — built, guarded, unreachable by arithmetic |

**Unmerged and open:** `claude/frame-cost-bisect` (the render fix — needs a PR
merged once CI is green), `claude/creature-reproduction-economics` (a design
document, deliberately no PR), and this branch.

## 3. Decisions — authorised vs not

**Authorised by the owner, in session** (no card ids; E9/E10 have them):
**E11** gap-crossing → the impulse verb · **E12** S6 reproduction · **E13**
predation as E7's one-file probe · **E14** the horizon change, *"Yes, let them
starve"* — **authorised, NOT shipped**, see §5 · and the reproduction economics
build (`birth_grant` + newborn-starts-small).

**Not authorised:**

- **`birth_size` as a gene.** `idle_cost`/`move_cost` are **flat per organism**
  (`creature.rs` ~1288, ~1329) — nothing reads `chain.len()`. **E10's premise
  that per-cell metabolic cost already prices a longer body is false in the
  code**, so size would ratchet to one extreme however priced. Needs an owner
  decision.
- The nest-larder fork of reproduction, until its experiment E3 says it exists.
- `shade-by-cell-type`, heritable chain length, the fear-scent channel,
  `Strike` as a brain verb (slot 12 stays unnamed).

## 4. Where I was wrong — do not inherit these

Six, and **the pattern is worth more than the list: every one was generalised
from partial evidence I had not finished reading.**

| I claimed | truth |
|---|---|
| An ant's bank ceiling is **930** | **570.** The diet matched filter pays a neutral gut **120** from a 480 leaf. I quoted the measured bank (**568**) and the 930 model in one message without noticing they disagreed by 40% |
| `food_energy == body_energy` everywhere | **False.** `flower` 1440, `fruit`/`windfall` 960 against 480. I read the first eight matches alphabetically and generalised. The pump invariant binds **flesh** only |
| "Only creatures are slow" rules out a frame-rate cause | **Wrong reasoning.** Creature ticks are quantised (nothing on 5 frames in 6), so they degrade visibly while the CA degrades smoothly. A lane pushed back and was right |
| The scheduler is not starving creatures | **Scene-limited.** True of an undisturbed colony, false once a pick is in the world |
| Counters were missing from a review card | They were in **`items[].meta`**, which is where the spec puts them |
| **The sky is the frame cost** | **Correlated, not causal** — see §5. I passed this to another session, who could have cut `sky_rows` on my say-so |

I also **pushed a merge without rebuilding** and broke CI. A clean text merge
is not a working tree.

## 5. Measured — do not re-derive

- **The render was 2/3 of the frame and nobody had ever measured it.**
  `Renderer::draw` is not in `App::update`, which is what every instrument
  times, so **every whole-frame figure on record is half a number.** Simulation
  18.9 ms against a **39.7 ms** redraw.
- **The cause was `rebuild_near_glow` hashing a `ChunkCoord` twice per disc
  cell** (~615 cells x ~6,900 discs, every full redraw) — not the sky. Control:
  with vaults off **both** worlds cost 5.2 ms and place the same number of
  glowing cells, so it was chunk *spread*, not crystal count. Fixed
  chunk-major: **~42 ms → ~7.5 ms, and PR #94 stays.**
- **The horizon change works and does not fix breeding.** `deaths` 0 → 25,
  `eats` 16 → 58; but cutting `start_energy` lowers the bank ceiling faster
  than the birth bar.
- **Falls-per-move reads backwards for a hopping species** — the ratio climbs
  0.225 → 0.298 while *absolute* falls drop 7,516 → ~1,520, because `moves`
  counts walking steps only. A ratio whose denominator the change moved.
- **§13o's "no beetle ever touched an ant" is dead** — beetles kill ~2 in 52.
  Conclusion survives, stated reason does not.
- **Predation is blocked by the sampler**, not perception: 31 nonzero trail
  cells in an 81,920-cell world against a two-cell read.
- **`open-bugs-handoff.md` §S is marked closed and is not** — under sustained
  mining the structural queue is self-sustaining, 5,558–9,080 produced against
  2,000 drained.

## 6. What I would do next — a suggestion, not a work order

**Judge this against the sources in §0 and your own read of what the project
needs. I am one subsystem deep and may be over-weighting it.**

1. **Merge the render fix.** Biggest player-visible win available (~17 → ~38
   fps) and it is already measured and built.
2. **Run §1's control.** Cheap, and it either unblocks the reproduction build
   or voids a report.
3. **Then the reproduction build** if the control passes.
4. **Put the body-size pricing question to the owner** — E10 rests on something
   untrue, and it blocks heritable anatomy.
5. **Someone should own `§S`** — it is marked fixed and is not.

**What I would *not* do:** start a new mechanic before §1's control runs; treat
my §6 order as settled; or re-measure anything in §5 without reading the report
that produced it first.

## 7. Environment notes that cost me time

- **`SendMessage` does not resolve cloud sessions.** Cross-session contact
  works via `create_trigger` + `persistent_session_id` + `fire_trigger`.
- **`review.py inbox` filters by author** — it will not show a lane's own card.
  Read `.git/pixel-physics-review/sync/cards/` and `sync/responses/`.
- **`/tmp` scratchpad notes die with the container.** Anything worth keeping
  goes on a branch.
- Container clippy is **1.94.1**, CI pins **1.98.0** — that drift produced two
  red PRs today.
- **The box is a measurement instrument.** Frame timings are worthless while
  another lane compiles; a byte-identical binary has read 2.42x apart on load
  alone. Run one lane when timing.
- `assets/worldgen.ron` is runtime-loaded; `materials/*.ron` and `species/*.ron`
  are `include_str!`ed. A worldgen A/B needs no rebuild and has no
  stale-binary hazard; a materials A/B has nothing but.
