# Two games, one repository: what to share, what to scope, what to split

**Status: proposal, not yet built. Revised 2026-08-30 after an adversarial
review that found the first draft's central recommendation was a no-op.**
Answers a question the owner asked directly, after the evolution lab's second
round landed:

> *"There's no way everything is going to be relevant to both projects and
> we're going to be making project specific decisions. Is there any possible
> way to split this up so that our agents don't have to search through a bunch
> of irrelevant reports and code as large parts of our projects start
> diverging, or do I just have to accept everything is shared or everything is
> separate?"*

**The answer is that the choice is not binary.** But the first draft of this
document got the mechanism wrong in a way worth recording at the top, because
it is the same error it correctly warned about two paragraphs later.

---

## 0. The correction that shaped this revision

The first draft proposed moving `CLAUDE.md`'s evidence narrative into
`.claude/rules/method.md` with no `paths:` frontmatter, "available but not
inlined", and claimed 25,635 → ~12,000 always-loaded tokens.

**A rules file with no `paths:` frontmatter loads at launch exactly as
`CLAUDE.md` does.** The saving is zero. That is the identical failure the
draft named for `@imports` — *"organisation, not scoping"* — committed in its
own recommendation.

**And the instrument would have certified it.** `scripts/contextbudget.py:131`
is `ALWAYS_LOADED = ROOT / "CLAUDE.md"`, and its docstring says so. After such
a move it would report ~12,000, `--check` would go current, and `--gate` —
which `docscheck` runs and CI runs — would go **green for a change that moved
nothing**. That is `CLAUDE.md`'s own worst-recurring failure, arriving
prospectively rather than in hindsight: *a number that is arithmetically
correct and answers a different question than the one asked looks exactly like
a result.*

**So: `contextbudget.py` must learn to count `.claude/rules/*.md` and the
nested-`CLAUDE.md` set *before* anything moves, not after.** Otherwise the
measurement that proves the win is structurally incapable of seeing the loss.

---

## 1. The numbers

Taken 2026-08-30. **Measured** rows come from a command; **judged** rows are
classifications by hand and are labelled as such.

| | | |
|---|---|---|
| `CLAUDE.md`, always-loaded before every session | **25,474 tokens** | measured (`contextbudget.py`) |
| — sections its own tool marks *consulted by lookup* | **63%** (~15,987) | measured |
| — of those, the bolded rule statement | ~14% | judged (regex over `**bold**`) |
| — of those, the evidence narrative | ~86% | judged |
| Claude Code's documented target per instruction file | **under 200 lines**; this is **1,584** | documented |
| `Reports/`, greppable prose | **166 files, 5.4 MB** | measured |
| — `du` reports 14 MB; **54% of that is PNGs and logs** a grep never touches | | measured |
| — and it is **not** flat: `img/`, `data/`, `lanes/`, `wp11-wip/` already exist | | measured |

**The documents a session is actually routed into**, which is the owner's
question restated as a table:

| document | tokens | outdoor-dominated? |
|---|---|---|
| `Reports/dead-ends.md` | **133,079** | no — cross-subsystem by design |
| `Reports/open-bugs-handoff.md` | **123,561** | no — cross-subsystem by design |
| **`README.md`** | **71,561** | **yes** — its milestone sections are named for the build |
| `PLAN.md` | 60,199 | yes |
| `PLAN-log.md` | 58,657 | yes |
| `CLAUDE.md` | 25,474 | no — 4.4% of lines name an outdoor-only subsystem |
| `Reports/README.md` | 24,082 | mixed — the index |
| `Reports/instruments.md` | 18,151 | mixed — 53 harnesses, 4 of them lab |
| `wiki/` | 49,947 over 11 pages | ~40-50% |

**`README.md` is the largest document in the repository**, is overwhelmingly
outdoor, and `CLAUDE.md`'s routing table sends *every* agent to its "By topic"
table first. The first draft did not mention it once. That was the biggest
miss in it.

---

## 2. The layers

Five that separate, plus two the first draft omitted. **"Tuning constants" is
not among them** — it was in the first draft's list and it does not belong:
its remedies are code changes inside the shared engine, so it is a discipline
within layer 1, not a layer with a scoping mechanism of its own. It is §4.

| layer | shared? | mechanism | status |
|---|---|---|---|
| **1. Engine** — `src/sim/` | **shared, permanently** | one library, two binaries | done |
| **2. Game code** — `src/lab/` vs `app.rs`, `worldgen/`, `player.rs` | **already separate** | separate binaries; `sim::frame::step` is the seam | done |
| **3. Always-loaded rules** — `CLAUDE.md` | **move out behind a pointer** | a routed report, or a skill | **see §3, this is the hard one** |
| **4. Reference docs** — `Reports/`, `wiki/`, **`README.md`** | **scope by namespace** | subdirectories + an index column | unused |
| **5. Gates** — CI | **scope by path, narrowly** | path filters; engine jobs always run | unused |
| **6. Skills and workflows** — `.claude/skills/`, `.claude/workflows/` | **already game-divergent, unscoped** | directory-scoped skills (`/lab:name`) | **unused, and it is the mechanism §3 needs** |
| **7. Instruments** — `examples/` | **the same flat-namespace problem** | 53 flat `.rs` files, indexed at 18k tokens | unused |

### 2a. The engine stays shared, and this is not negotiable

The lab's value is that its plants and ants are *the same code*: a creature
evolved in the lab is a `.ron` the outdoor game can plant (`species_export`,
round-trip verified). Two repositories means porting every engine fix twice
and watching the species format drift.

Nothing needs removing to make the lab cheap — blasts 0.000 ms, particles
0.000 ms, the gnome 0.001 ms, the structural scheduler 0.028 ms in a bed with
no rock against 3.389 outdoors. **The lab's speed comes from what is not in
the box, not from what is not in the binary.**

---

## 3. The always-loaded layer, done correctly

Four places the evidence narrative could go. Only two of them save anything.

| where | launch cost | when it fires |
|---|---|---|
| `.claude/rules/x.md`, **no** `paths:` | **unchanged** | always — the first draft's proposal, a no-op |
| `@import` from `CLAUDE.md` | **unchanged** | always — organisation only |
| `.claude/rules/x.md`, **with** `paths:` | saved | only on a path match — see the two objections below |
| **a routed report + a pointer** | saved | on demand |
| **a skill** under `.claude/skills/` | saved | when invoked, or when judged relevant |

**The repository has already done this once and it worked.** `CLAUDE.md`'s
*"Running a program of sessions — moved out"* section says in as many words:
*"It is a report rather than a section here because it applies to a minority
of sessions and cost every one of them ~2,200 always-loaded tokens."* That is
the proven mechanism, in this file, for exactly this problem.

### Why not simply `paths:`-scope each subsystem's rules

The first draft argued the method rules "match nothing in particular". **That
is false and the review demonstrated it**: of the three episodes the draft
cited, two touched `src/**` or `examples/**` and would have fired on a glob
anyone could write. The argument was a rationalisation.

The conclusion survives on two better grounds, both mechanism-level:

1. **A path-scoped rule triggers when a matching file is *read*.** *Look
   before you measure* and *reproduce before you fix* govern what you do
   **before the first file is opened**. A rule that arrives with the first
   `Read` has already missed its cue.
2. **It does not survive compaction unless re-matched.** In a repo whose
   routed documents run 60k-133k tokens each, sessions compact routinely. A
   reflexive rule that silently stops existing mid-session is worse than one
   that costs 2,300 tokens.

So: **rule statements stay inlined; evidence moves to a routed report.** That
is a smaller win than the first draft claimed, and it is a real one.

### Two live bugs that must be settled before any of this is built

- [#16299](https://github.com/anthropics/claude-code/issues/16299) — **open,
  confirmed, with a repro**: `paths:`-scoped rules load at session start
  regardless of the glob.
- [#23569](https://github.com/anthropics/claude-code/issues/23569) — **closed
  as not planned**: under a **git worktree**, the `paths:` filter is ignored
  and the rule always loads.

**The second is disqualifying if still present.** `CLAUDE.md` mandates *"Work
in your own worktree, not the shared checkout"*, and this checkout has four
live agent worktrees right now. A saving that evaporates in exactly the
sessions the repo prescribes is not a saving.

**Settle it before building, not after**: land one throwaway `paths:`-scoped
rule, run `/context` from the main checkout and from a worktree, and read
whether it loaded. The `InstructionsLoaded` hook logs which instruction files
loaded and why, which is the instrument this restructure otherwise lacks —
and a restructure whose entire value is a token count, in a repo whose
standing rule is *put the fault back and watch it go red*, must not ship
without one.

---

## 4. The divergence risk nothing guards

**This is what the owner's question is really about**, and the design guide
already named it: *"the one real coupling risk is constants, not code."*

The plant economy is calibrated against the outdoor world. A lab running
constant light, no wind and a hand-built bed is a **different operating point
for the same weighted sums**. If the two games start wanting different values
for `seed_maturity`, `light_weight` or `crowding_weight`, that is where a
silent fork begins — because it will be done by editing a shared constant and
noticing later.

The guide's three tiers are already decided: species file first, then the
`tunables.rs` registry, then — only if those fail — a per-world parameter
block. **What is missing is not the decision, it is the alarm.** Nothing
notices when a shared constant is edited for one game's benefit.

The cheap version already has a worked shape in this repo:
`the_outdoor_render_is_unchanged_by_the_lab_interior` is three arms, the
middle one a positive control that fails if the feature is deleted. The same
form over the outdoor *stand* would catch the economy fork.

---

## 5. Build order

| | change | effect | real risk |
|---|---|---|---|
| 1 | ~~CI build cache~~ | done 2026-08-30 | — |
| 2 | ~~`rust-toolchain.toml`~~ | done 2026-08-30 | — |
| 3 | **A game column in `README.md`'s By-topic table and `Reports/README.md`** | scopes the two routing layers every agent is sent to first, **71.5k + 24k tokens**, by convention and at once | none; reversible |
| 4 | **A guard that the outdoor game is unchanged** (§4) | catches the constant fork | none |
| 5 | **Teach `contextbudget.py` to count `.claude/rules/*.md` and nested `CLAUDE.md`** | makes step 7 measurable at all | none |
| 6 | **Settle #23569 in a worktree** (§3) | says whether step 7's mechanism works here | none |
| 7 | **Move the evidence narrative to a routed report** | ~25,474 → ~12,000, *if* 5 and 6 come back clean | see §3 |
| 8 | **`Reports/{engine,outdoor,lab}/`** | scopes the grep structurally | **see below — not link churn** |
| 9 | Path-filtered outdoor gates | ~12 min off a lab-only PR | a gate that quietly stops running |

**Step 3 is the highest ratio in the list and should happen regardless of
everything else.** It is one column in two files, it is reversible, and it
covers 95k tokens of the most outdoor-dominated routed material in the repo.

### Step 8's risk, stated correctly

The first draft said "link churn; `docscheck` catches it". **`docscheck` does
not catch it — `docscheck` is the casualty, and it fails silent.** Five
non-recursive globs iterate over what is left at top level:

- `scripts/docscheck.sh:99` — the link-resolution corpus
- `:243`, `:279`, `:295` — checks 3b, 3c, and **4, "every report must be
  indexed"**
- `scripts/reporttoc.py:208`

Move 165 files into subdirectories and all five walk an emptied directory. No
error, no red, output still reads `clean`. **Check 4 is what makes step 3
enforceable, so step 8 would silently disable the gate step 3 depends on.**

Worse: **three of `docscheck --selftest`'s seven fault rows hardcode
`Reports/<name>.md` paths that step 8 moves** — so the change breaks the check
*and* the check on the check, in one commit.

What survives: `scripts/addrcheck.py` uses `rglob` on the basename, so
`dead-ends.md`'s cross-document addresses keep resolving. `bugindex.py`
addresses one file and is unaffected.

**Prerequisite sub-step, and the acceptance condition**: re-path those five
globs and the three selftest rows first, then require `docscheck --selftest`
to report all faults detected — which is this file's own rule about citing a
green.

**And step 8 forces a `CLAUDE.md` edit**, because its "Where knowledge already
lives" table names report paths. Every such edit burns a prompt-cache prefix
no later session can share. **Layers 3, 4 and 5 are not independent**, and the
first draft's table implied they were.

---

## 6. What this does not solve

- **It scopes one of six routed corpora, and not the largest two.**
  `dead-ends.md` (133k) and `open-bugs-handoff.md` (123k) are deliberately
  left alone — both are grepped rather than read, both are explicitly
  cross-subsystem, and `CLAUDE.md` already prescribes grepping the *mechanism*
  rather than the area. Splitting them by game would break the one property
  that makes them work: that a mechanism tried on plants is findable by
  someone about to try it on creatures. **But that means 257k tokens of the
  material an agent searches is out of scope here and needs its own answer.**
- **It does not touch `examples/`** — 53 flat harnesses indexed at 18k tokens,
  the identical problem with the identical fix available.
- **It does not make the two games independent.** They share an engine by
  design; a change to `sim::plant` is felt in both, and that is the point.
- **It does not address `CLAUDE.md`'s growth rate**, which is the underlying
  problem: **+2,583 / −365 lines**, a **7.1:1** add-to-remove ratio. Scoping
  buys headroom once; the file's own removal criterion is what keeps it.
- **The ~12,000 estimate is arithmetic, not a measurement**, and it assumes
  rule statements stand alone without their incidents. Several will not.
