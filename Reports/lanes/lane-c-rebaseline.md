# Lane C — re-baselining the creature record on today's `main`

## 2026-08-29 → coordinator

**Branch `claude/creature-rebaseline-lane-c`.** Head SHA at the bottom of this
note. All figures below were measured on `ba6fc98` (today's `main`), one
4-core cloud container, one session, release binaries rebuilt from source
first (`cargo build --release --examples`, exit 0 read through
`PIPESTATUS`, not through a pipe).

### 1. The parameter-echo assignment was already done, six days ago

**All three named defects are fixed on `main`**, by `b6d25c4` (2026-08-23,
*"The harnesses name their own parameters"*) — the same commit whose message
quotes the `"4 beetles" against BEETLES = 9` line as the thing it fixed. All
three already `panic!` on an unknown argument too, so *"make an unrecognised
argument an error"* is also done. Details in the report.

**The one still broken was not on the list: `gnome_depth`.** It echoed
neither `zoom=` nor `depth=`, and only *warned* on an unknown key. Sharper —
its `depth=` arm fell through to `TreeDepth::Weave` for any unrecognised
**value**, so `depth=fron` rendered a `Weave` sheet indistinguishable from
the `front` one asked for. That is the megastudy failure with the typo moved
from the key to the value, **and it is the one form an echo cannot catch on
its own**: the echo would have said `depth=Weave`, truthfully, to someone who
typed `front`. Both arms now panic; line one names the parameters.

**The method point, not the three rows:** the defect list was a claim about
the past. I audited all five harnesses in `instruments.md` §Creatures rather
than the three named, which is the only reason `gnome_depth` was found.

### 2. §5 — the `eats 6 / deaths 0` reading is **overturned**

Your numbers reproduce exactly, byte for byte, on two runs: food stock
791,040 → **3,057,600**, `of which corpse 0`, **eats 6**, **deaths 0**,
126 live organisms.

**The reading does not follow, and the ledger printed one line below them
says so.** From the same run:

```
energy census: granted 24300  plant 720  corpse 0
               metabolized 5400  moved 3165  synapses 2808  dissipated 0
```

- Energy into the colony: `granted` 24,300 + `harvested_plant` 720 = 25,020.
  **Food supplied 2.9% of it.** The other 97.1% is the spawn grant.
- Energy out: 5,400 + 3,165 + 2,808 = 11,373 — **45% of the budget spent**
  at frame 12,000.
- `hunger_fraction` is 0.5 (`ant.ron`). An ant is not hungry until it is
  **50% depleted**. The mean ant finishes the run at 45%.

So *"the world feeds a colony for free"* is refuted by the ledger: delete
every leaf in the world and the colony's energy budget moves by 2.9%.
Nothing is hungry because **the run ends just short of the hunger
threshold**, not because food is underfoot. `eats 6` is not a colony
declining to eat — it is the six most-travelled ants crossing 50% early,
which is exactly what a 45%-mean distribution with spread predicts.

**And `food stock` is a canopy-growth curve, not a feeding signal.** It rose
by 2,266,560 while the ants took **720** out of it — 0.03%. That curve is
what six growing trees do with no ants present at all. `ascii.rs`'s own
comment beside the counter warns that a cell count "rises as a stand of
trees grows whatever is happening to the animals"; pricing the cells did not
remove that failure mode, it re-denominated it.

**Why the distinction decides the fix, which is the reason this is worth
your time.** Your reading sends someone at food abundance — make the floor
scarcer, thin the canopy. The measurement sends them at the **horizon**:
`ant.ron` is `start_energy: 900`, `idle_cost: 0.10`, `tick_interval: 6`, so
idle life is ~9,000 ticks = **54,000 frames**, and `ascii` runs 12,000
frames = **2,000 ticks**. The scene stops at 22% of an idle lifetime. This
is also why `creature_space` overrides `START_ENERGY` to **90.0** — one
tenth — for its own economy sweep: at `ant.ron`'s 900 the selection question
is not reachable inside any horizon that harness could afford. The scene is
not showing you a colony that cannot starve; it is showing you the first
fifth of one.

Your conclusion — no selection pressure in this scene — **holds**. The
mechanism behind it does not, and the mechanism is what the next stage would
be built on.

### 3. §1 — the standing guards, re-baselined paired on today's `main`

`examples/ascii`, two full back-to-back runs, same session, same container,
nothing else on the box.

| Guard | 2026-08-23 (§4 table) | **today, `ba6fc98`** |
|---|---|---|
| Frame cost, colony scene, mean over 12,000 frames | 3.488 / 3.491 ms | **2.906 / 2.943** (`ba6fc98`), **3.362 / 3.417** (`f96c08d`) |
| Foraging pays — forager minus immobile, no moss / moss | +0.474 / +0.466 | **+0.427 / +0.459** (`ba6fc98`) |
| Ants fed | 0.75 / 0.75 | **0.68 / 0.75** (`ba6fc98`) |
| Reference genomes — `authored` / `zero` | 0.690 / 0.297 | **0.696 / 0.299** (`f96c08d`) |
| Determinism, two `ascii` runs | identical | **identical**, both trees — 0 differing lines |

**All guards hold.** Two things worth your attention:

**The frame cost rise is scene size, not a creature regression.** The colony
went **126 → 153 live organisms** across the organs merge (+21%) while the
mean went +16%, so cost per organism fell. Six runs, and the parallel-stress
worst-frame proxy swings **77% (9.4 → 16.7 ms)** on a fixed binary and scene
— that column is noise, not a machine ruler. The *mean* is stable: runs 1
and 2 differ 1.3% while their proxies differ 57%.

**The reference pair did not move, and that is the informative half.** §4
argued `zero`'s survival is "a pure function of the scene". Across *both*
this week's merges it reads **0.299**, and the economy sweep's `immobile`
column reads 0.298 at all four settings. So the wetland economy scene was
barely touched, while the `ascii` foraging scene lost three quarters of its
trips over the same period. **Two creature scenes, opposite answers** — a
blanket "re-measure everything" would have been wrong one way and "the
guards are fine" wrong the other.

### 4. The `forage_reach` bars in `examples/ascii.rs` have quietly gone fragile

This is the finding with a live consequence.

| | comment in `ascii.rs` says | **measured today** | bar |
|---|---|---|---|
| `forage_trips` | "measured 98 here" | **24** → **23** after the organs merge | `>= 14`, now **`>= 6`** |
| `forage_depth_max` | "measured 18 here" | **37** → **28** | `>= 8`, unchanged |

The trip bar was set as *"a seventh of the trip count … because outcome
spread here is large and a bar near the measurement flakes"*. Against 98
that was a seventh. **Against today's 24 it is 58% of the measured value** —
the headroom the comment claims is gone, and the comment still tells the
next reader the number is 98.

The colony has not gone sessile; it has changed shape — **four times fewer
trips, twice as deep** (mean depth 13.2 → 17.4 → deepest 37). Both halves
are the worldgen reshaping under it.

### 5. §3 — `ant_ablation`'s real cost, and the answer

**It does not buffer.** Timestamped per line: parameter line at **0 s**,
first arm row at **33 s**. Rust's stdout is a `LineWriter`. Your 600 s kill
simply landed two thirds of the way through a **868 s** run (predicted 890 s
from a two-point linear fit; ~1.39 ms per arm-run-frame, 20 arms). If you saw
literally no output, that is the capture layer buffering, not the harness.
It now prints its expected cost up front and streams per-arm/per-seed
progress to stderr.

**The answer, and it is sharper than "yes".** Ablating **`Bias->Move`
reproduces the zero control to every printed digit** — travelled 0.0,
coverage 46, `first-pickup` never. One connection of twelve carries the
entire separation. That is also the positive control the table needs: the
metrics can reach the floor, so the six ablations that return `authored`'s
row unchanged are real nulls, not a dead instrument.

**But the defaults cannot answer the feeding half at all**: `deliv 0.0` and
`eats 0.0` in *every* arm, because `food=corpse pile` is the finite source
`ascii.rs` records as producing zero deliveries. So both `Feed` ablations and
both `Drop` ablations are **vacuous by construction** on the invocation
everybody reaches for first. `terrain=world food=trees` is the configuration
the 28.8-deliveries figure comes from.

Recorded, not acted on: three of the six locomotion sign-sweeps beat the
authored genome on participation, `Caution=hi` by **10x on pickups** (40.8
against 4.0).

### 5b. After your colony fix landed (`d007c156`)

Merged it. **I took no `scene=colony` measurement at all**, per your original
instruction, so nothing here needed re-taking on that account — but the same
merge carried **247 new lines of `src/sim/creature.rs`**, so I re-measured
rather than accepting "ascii is unaffected" on trust.

**Confirmed: every `ascii` counter byte-identical** — eats 3, deaths 0, moves
8812, pickups 3157, deliveries 1047, trips 23, deepest 28, food stock
3,685,920, whole energy census. And the **ablation's full 20-arm table is
byte-identical too**, at 860 s against the earlier 868 s — even though the
merge added a `recurrence` term to `genome_from_wiring` that could have
changed every arm. It does not reach the authored ant.

**I checked the null rather than believing it.** Identical output across a
change that touched the file under test is `CLAUDE.md`'s stale-binary tell,
so: `filmstrip scene=colony` at **its own default seed** — the invocation
that panicked before your fix — runs to completion on the same binary. Your
fix is in it; the build took; the null is real. Your reading was right, and
now it is measured.

**Both of your asks are landed.** The bars are re-set from today's
measurement (trips 23, bar 14 → **6** = 23/4.1 where 4.1x is the largest
legitimate drift on record), the stale 98/18 comment is replaced with what
the numbers were, when, and which merge moved them, and the trade is stated
in the code: a genuine 2x foraging regression would now pass. `-Bias->Move`
is named as the table's positive control in `ant_ablation`'s doc comment,
with the instruction to read it first.

### 6. What I did not do

- **No `scene=colony` measurement or review card**, per your instruction.
- **`Reports/open-bugs-handoff.md` §R's seed table is yours** — you have
  re-measured it and you own the fix, so I left it rather than filing a
  competing edit into the file `CLAUDE.md` names as the repo's most
  collision-prone (118 landings). Your today-column numbers are quoted in my
  report as provenance only.
- **The economy sweep is a `ba6fc98` reading.** `main` moved 30 commits
  mid-session (plant organs, `f96c08d`) and I re-ran `ascii` on the merged
  tree but not the 37-minute economy sweep. It is the one number here I would
  spend another 37 minutes on next.

### How to find this work

**Branch: `claude/creature-rebaseline-lane-c`.** PR body at
`PR-BODY-lane-c.md`; report at `Reports/creature-rebaseline-2026-08-29.md`,
indexed in `Reports/README.md`.

**Head SHA: `3e2bf8bb88ecc8608c8913bd1001bd51a09a9669`** — the last work
commit, which carries the post-`d007c156` re-verification. The branch tip is
one commit later (this note recording the SHA; a note cannot contain its own
hash). `git log --oneline origin/claude/creature-rebaseline-lane-c` shows
both.

Merged `origin/main` through **`d007c156`** (your colony fix). Gates on that
tree: clippy clean on 1.94.1 **and on CI's 1.98.0**, `cargo test --lib`
**1005 passed / 0 failed / 54 ignored**, `docscheck` clean, `examples/ascii`
exit 0 with the new bars.

Files touched, none of them a shared append-only record: `examples/ascii.rs`,
`ant_ablation.rs`, `gnome_depth.rs`, the new report, `Reports/README.md`,
`creature-evolution-plan.md` §4, `instruments.md`, this note. **I did not
touch `open-bugs-handoff.md` or `dead-ends.md`** — nothing here is a new bug
or a rejected mechanism, and §R is yours.
