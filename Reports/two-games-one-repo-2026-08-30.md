# Two games, one repository: what to share, what to scope, what to split

**Status: proposal, not yet built.** Answers a question the owner asked
directly on 2026-08-30, after the evolution lab's second round landed:

> *"There's no way everything is going to be relevant to both projects and
> we're going to be making project specific decisions. Is there any possible
> way to split this up so that our agents don't have to search through a bunch
> of irrelevant reports and code as large parts of our projects start
> diverging, or do I just have to accept everything is shared or everything is
> separate?"*

**The answer is that the choice is not binary, and the repository is already
paying for treating it as though it were.** There are six separable layers
here. Two must stay shared, one is already split, and three are shared today
only because nobody has used the mechanisms that would scope them.

---

## 0. The measurements this rests on

Taken 2026-08-30. Every figure is reproducible from a command in this repo.

| | |
|---|---|
| `CLAUDE.md`, always-loaded before every session | **25,635 tokens** (`scripts/contextbudget.py`) |
| — of it, sections its own tool marks *consulted by lookup* | **63%** (~16,027 tokens) |
| — of those, the **rule statement** (the bolded imperative) | **14%** (~2,308 tokens) |
| — of those, the **evidence narrative** behind the rules | **86%** (~13,719 tokens) |
| Claude Code's documented target for one instruction file | **under 200 lines**; this file is **1,584** |
| `Reports/`, one flat directory an agent greps | **165 files, 14 MB** |
| — outdoor-flavoured by filename | 40 |
| — lab | 1 |
| — plant/creature/method, genuinely shared | 64 |
| — unclassifiable by filename | **60** |
| `wiki/`, routed to one page at a time | 49,535 tokens over 11 pages, **40% of it irrelevant to the lab** |
| `dead-ends.md` / `open-bugs-handoff.md`, grepped by everyone | ~97,000 tokens each |

**The flat `Reports/` namespace is the part that gets worse on its own.** The
lab has one report today and will have twenty. Nothing about the current
layout tells a grep for *"light"* that `sky-light-design.md` is outdoor work
and `evolution-lab-gate-1` is not.

---

## 1. The six layers, and the answer for each

| layer | shared? | mechanism | status |
|---|---|---|---|
| **Engine** — `src/sim/` | **shared, permanently** | one library, two binaries | done |
| **Game code** — `src/lab/` vs `app.rs`, `worldgen/`, `player.rs` | **already separate** | separate binaries; `sim::frame::step` is the seam | done |
| **Always-loaded rules** — `CLAUDE.md` | **scope by path** | `.claude/rules/*.md` with `paths:` frontmatter | **unused** |
| **Reference docs** — `Reports/`, `wiki/` | **scope by namespace** | subdirectories + an index that says which game | **unused** |
| **Gates** — CI | **scope by path** | path filters; engine jobs always run | **unused** |
| **Tuning constants** — species `.ron`, `tunables.rs` | **the real divergence risk** | design guide §7a's three tiers | **decided, unenforced** |

### 1a. The engine stays shared, and this is not negotiable

The lab's entire value is that its plants and ants are *the same code*: a
creature evolved in the lab is a `.ron` the outdoor game can plant
(`species_export`, round-trip verified). Two repositories means porting every
engine fix twice and watching the species format drift apart.

The feasibility work already settled that nothing needs removing to make the
lab cheap: blasts 0.000 ms, particles 0.000 ms, the gnome 0.001 ms, the
structural scheduler 0.028 ms in a bed with no rock against 3.389 outdoors.
**The lab's speed comes from what is not in the box, not from what is not in
the binary.**

### 1b. Game code is already split, and the seam is proven

`src/lab/` against `src/app.rs` + `src/worldgen/` + `src/sim/player.rs`, two
binaries, one library. The one place they could silently fork — the frame
sequence — is now `sim::frame::step`, guarded by a hash taken off `main`
through the old inline version before the extraction landed.

### 1c. Always-loaded rules: `.claude/rules/` with `paths:`

**This is the mechanism the repository is missing, and it is the direct
answer to the owner's question.** A file at `.claude/rules/<topic>.md` with

```yaml
---
paths:
  - "src/sim/structural.rs"
  - "src/sim/load.rs"
---
```

loads **only when Claude reads or edits a file matching those globs.** Rules
with no `paths:` load at startup as `CLAUDE.md` does.

So the split is not "one file or two repositories". It is:

- **`CLAUDE.md` keeps what fires reflexively on every task** — the ethos, what
  the project optimises for, where knowledge lives, the commands, how to get
  the owner's judgement, how to write to the owner, working alongside another
  session, and the ~2,300 tokens of *rule statements* from Method, Gotchas and
  Conventions. Estimated **~9,000-12,000 tokens**.
- **`.claude/rules/method.md`** carries the evidence narrative with no
  `paths:`, so it is available but not inlined — or, better, is split by the
  subsystem each incident came from.
- **`.claude/rules/structural.md`**, **`plants.md`**, **`worldgen.md`**,
  **`lab.md`** each carry their subsystem's gotchas behind a `paths:` glob.

**Two things that look like this mechanism and are not.** `@imports` in
`CLAUDE.md` expand at launch and **save no tokens at all** — they are
organisation, not scoping. And a nested `src/lab/CLAUDE.md` loads only once a
file in that directory is read, which is useful but *additive*: it cannot make
the root file smaller.

**The risk, stated plainly, because it is real.** Nearly everything that went
right in the lab's two rounds came from a rule firing *unprompted* — *look
before you measure* caught the interior drawing as sky; *ask what your number
counts when nothing is wrong* killed a finding that had already been relayed
to the owner as fact; *you are probably measuring a stale binary* caught a
harness describing a bed the game does not build. **A rule behind a `paths:`
glob only fires when the glob matches, and the ones that saved those cases are
method rules that match nothing in particular.** So the cut has to be
*rule stays, evidence moves* — not *this subsystem's rules move*. Getting that
backwards would trade 13,700 tokens for the thing that makes the file work.

### 1d. Reference docs: a namespace, not a pile

165 files in one directory, and an agent's first move on any question is to
grep it. The fix is boring and it is the whole of the problem:

```
Reports/engine/     shared: method, measurement, plants, creatures, the substrate
Reports/outdoor/    worldgen, structural, destruction, the gnome, weather
Reports/lab/        the evolution lab
Reports/README.md   the index, with a game column
```

`wiki/` the same: `wiki/lab/` beside the existing pages, and `wiki/README.md`
saying which game each page describes.

**What makes this cheap here**: `Reports/README.md` is *already* the routing
layer every agent is told to read first, and `scripts/docscheck.sh` already
gates that every report has a line in it. Adding a game column to that index
costs one column and makes the grep scoped by convention immediately, before
any file moves. **Do the index column first and the directory move second** —
the column delivers most of the benefit and is reversible.

**`dead-ends.md` and `open-bugs-handoff.md` do not move.** Both are grepped,
not read; both are explicitly cross-subsystem; and `CLAUDE.md` already
prescribes grepping the *mechanism* rather than the area. Splitting them by
game would break the one property that makes them work — that a mechanism
tried on plants is findable by someone about to try it on creatures.

### 1e. Gates: path filters, with the engine always gated

CI runs nine jobs on every push, critical path 19.4 min, 55.6 min of compute.
A lab-only change cannot affect `worldgencheck` or `acceptance` — but a change
to `src/sim/` can affect everything, and **most lab work touches `src/sim/`**,
so a naive path filter would be a gate that quietly stops running.

The safe form is a filter that fires *generously*: run the outdoor gates
unless the diff is confined to `src/lab/`, `src/bin/`, `examples/lab*` and
`Reports/lab/`. That is a narrow exemption and it is the only one worth
having, because it is the only one that is provably safe.

**A build cache is worth more than a filter and carries none of that risk**,
which is why it landed first (2026-08-30). Clippy is the evidence: it only
*checks*, so 9 s of real work becoming 47 s on CI is nearly all dependency
compilation, and seven jobs were each paying it from cold.

### 1f. The real divergence risk is constants, and nothing guards it

**This is the layer the owner's question is really about**, and the design
guide already named it: *"the one real coupling risk is constants, not
code."* The plant economy is calibrated against the outdoor world. A lab that
runs constant light, no wind and a hand-built bed is a **different operating
point for the same weighted sums**. If the two games start wanting different
values for `seed_maturity`, `light_weight` or `crowding_weight`, that is where
a silent fork begins — because it will be done by editing a shared constant
and noticing later.

The guide's three tiers, cheapest first, are already decided:

1. **Push the difference into the species file.** A lab herb is a different
   `.ron`, not a different constant. Five species already differ this way.
2. **Make the scene carry its own tuning** via the `tunables.rs` registry.
3. **Only if 1 and 2 fail:** a per-world parameter block, resisted until a
   real case forces it.

**What is missing is not the decision, it is the alarm.** Nothing today
notices when a shared constant is edited for one game's benefit. The cheap
version is a test asserting the outdoor stand is unchanged — which the lab's
interior work already demonstrates the shape of
(`the_outdoor_render_is_unchanged_by_the_lab_interior`, three arms, the middle
one a positive control that fails if the feature is deleted).

---

## 2. Build order

Ordered by value per unit of risk, not by size.

| | change | effect | risk |
|---|---|---|---|
| 1 | ~~CI build cache~~ | done 2026-08-30 | — |
| 2 | ~~`rust-toolchain.toml`~~ | done 2026-08-30 | — |
| 3 | **Game column in `Reports/README.md`** | scopes the grep by convention at once | none; reversible |
| 4 | **`.claude/rules/` for the evidence narrative** | 25,635 → ~12,000 always-loaded | **real** — see §1c |
| 5 | **`Reports/{engine,outdoor,lab}/`** | scopes the grep structurally | link churn; `docscheck` catches it |
| 6 | **A guard that the outdoor game is unchanged** | catches the constant fork | none |
| 7 | Path-filtered outdoor gates | ~12 min off a lab-only PR | a gate that stops running |

**3 and 6 are free and should happen regardless of what is decided about 4.**

---

## 3. What this does not solve

- **It does not make the two games independent.** They share an engine by
  design; a change to `sim::plant` is felt in both, and that is the point.
- **It does not decide anything about game content.** Whether lab-evolved
  species appear in the outdoor world is open question §8.8 of the design
  guide and is not a repository-layout question.
- **It does not address `CLAUDE.md`'s growth rate**, which is the underlying
  problem: +2,583 / −365 lines over its history, a **7.1:1** add-to-remove
  ratio. Scoping buys headroom once. The file's own removal criterion is what
  keeps it.
- **The 12,000-token estimate for step 4 is arithmetic, not a measurement.**
  It assumes the evidence narrative moves cleanly and the rule statements
  stand alone, and some of them will not — several rules are only intelligible
  *because* of their incident. Budget for the estimate being optimistic.
