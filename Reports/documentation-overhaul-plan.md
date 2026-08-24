# Documentation overhaul — the executing plan

**Status: executed 2026-08-21/22 in seven commits on `main`; four of the
thirteen CLAUDE.md recommendations deferred and still open.** Recovered
2026-08-24 from the originating session's local plans directory, which git
does not track and the default 30-day transcript cleanup eventually reaches.
The document below is the plan **as written before execution** and is left
verbatim: its counts are pre-execution snapshots (the census reads 501
entries here and landed at 542; the audit is described as "in-tree,
uncommitted" and is now `documentation-audit.md`), and the
`C:\Users\Scott\.claude\plans\` paths it cites are local to one machine.

**Why it is in the repo at all.** The audit records *what was wrong*. This
records *what was going to be done about it, in what order, and what was
deliberately not done* — and the refusals are the half that leaves no trace
in the tree. No wholesale README reorder (grep navigation makes it churn on
a contested file; a TOC buys the same for 3% of the diff); no
`Reports/archive/` (the recommendation reversed mid-plan — with per-report
status in the index, archiving buys an agent nothing, and the `git mv`
breaks `Reports/` paths held in other sessions' uncommitted worktrees). Both
had already been chosen the other way earlier in the same session. A later
review that does not know this will re-propose them.

**What landed:** `033add8` the audit · `6d669ce` `Reports/README.md` +
`scripts/docscheck.sh` · `4f20721` README (architecture map 14→33 modules,
five missing status sections, three self-contradictions) · `11f1adf` wiki
(smoke denial, the fire page's flammability ladder, dated notes) ·
`59ceef5` the PLAN/`PLAN-log.md` split · `add3fe3` `Reports/dead-ends.md`
(542 entries) · `0efeb24` CLAUDE.md routing.

**What did not:** CLAUDE.md recommendations 5, 6, 7 and 12 — the two
narration move-outs and the Conventions re-clustering — deferred because
`load-share`, `plant-branch-angle` and `perf-lock` each edit a region those
moves would touch. They are approved, not dropped; the full text is in
`claude-md-recommendations.md` beside this file, with the current
landed/pending status of each verified against `main`.

**Not recovered, still local-only:** the ten-agent census raw output and its
journal (~800 KB), and the fifteen subagent transcripts (~5.5 MB). The
census *synthesis* is `dead-ends.md` and is complete; the raw is provenance
only. The harness that produced it is
`.claude/workflows/doc-audit-agent-framing.js`, so it can be re-run rather
than restored.

*Recovered as written: 2026-08-19 (v2). Nothing below this line was edited.*

---

# Documentation overhaul — plan v2, agent-consumer framing

## Context

This project is developed entirely by Claude. The primary consumer of its
documentation is a **fresh agent session with no context**, arriving
repeatedly, paying tokens to read, and acting *literally* on what it finds.
The plan therefore optimizes four things, in order: **routing** (which file
do I open), **recall** (has this been tried), **literal accuracy** (an agent
does not discount a wrong sentence the way a human does), and **read cost**
(bounded, grep-navigable units). Human-facing polish (section order, controls
tables) is real but subordinate.

Measured read costs: `CLAUDE.md` ~7k tokens (auto-loaded, every session),
`README.md` ~21k, `PLAN.md` ~66k, `Reports/` ~270k (41 files).

## Evidence base (all complete, all preserved)

1. **Audit** — `Reports/documentation-audit.md` (in-tree, uncommitted): 21
   verified findings against `565e4b4`. §R README, §W wiki, §P PLAN, §F
   CLAUDE, §D Reports/docs.
2. **Dead-end census** — workflow `wf_df7aa74f-5c1`, 10 agents over source
   comments + Reports + README: **501 tried-and-reverted entries** with
   per-entry re-test conditions. By area: plants 121, structural 76,
   destruction 73, creatures 53, liquids 45, field 32, rendering 28, weather
   18, powders 13, worldgen 12, scheduler 9, parallelism 8, other 13. Raw
   results: `C:\Users\Scott\.claude\plans\dead-end-census-wf_df7aa74f.txt`
   (+ `dead-end-census-journal.jsonl` beside it). **Outstanding:** the
   PLAN.md census agent hit the session limit (resets 4am) — resume
   `wf_df7aa74f-5c1` with the saved script; completed agents replay from
   cache.
3. **CLAUDE.md organization review** — 13 recommendations, saved at
   `C:\Users\Scott\.claude\plans\claude-md-review.json`. Headline: a
   task-to-rule topic map; ~700–800 tokens/session of true redundancy;
   zero-cost findability fixes (promote five bold leads to `###`, cluster
   the 93-line Conventions list); the routing rows the two new indexes need;
   and a boundary rule so `dead-ends.md` and `open-bugs-handoff.md` never
   claim the same knowledge.
4. **PLAN.md reference census** (done inline after the agent hit the session
   limit): ~120 inbound references from 44 files, of which exactly **2**
   target the progress log (`README.md:668` anchor; a passing mention in
   `rigid.rs:21` that survives a split unchanged). The 2,127-line log is
   write-only archive.
5. **Bounded-read-units measurement**: `CLAUDE.md` and all wiki pages have
   no heading-less span over 120 lines. `README.md` has three (worst 224
   lines, the field-grid section at 207–430); `PLAN.md` has twelve (worst
   602 lines, inside the progress log).

## Deliverables, ranked

### Tier 1 — routing and recall infrastructure (the agent-framing core)

1. **`Reports/README.md`** — index of every report: one line each — subject,
   status taken from the report's own `**Status:**` header (they nearly all
   have one), superseded-by where applicable. Generated **from the directory
   listing at execution time**, never from the audit snapshot, so reports
   that land meanwhile are included by construction. Includes a pointer row
   for `docs/future-directions.md` (historical, D3).
2. **`Reports/dead-ends.md`** — the tried-and-reverted index, seeded from
   the 501-entry census. One `##` section per area; each entry 1–3 lines:
   *what was tried → what happened → the condition under which the rejection
   held → where recorded* (symbol/heading refs, never line numbers). It is
   an **index**: the authoritative record stays in the source comment or
   report it cites. Expect ~250–350 entries after dedup (the same dead end
   is often recorded in code + report + README). Fold in the outstanding
   PLAN.md census and a marker-sweep delta over files changed since the
   census before writing.
3. **`CLAUDE.md` routing** (small, surgical): a knowledge-table row for each
   new index; amend "A revert keeps the knowledge" to name `dead-ends.md` as
   where the record goes; trim the `open-bugs-handoff` row so the two files
   split cleanly ("is this broken?" vs "was this tried?"); replace the two
   hardcoded per-report table rows with the index row (review rec #8).
4. **R1: architecture map rewrite** — the file-routing table. One line per
   real module (27 in `src/sim/`, 6 in `src/worldgen/`, 5 top-level), in the
   existing voice. ~1 MB of source currently invisible.
5. **`scripts/docscheck.sh`** — the mechanical backstop: (a) markdown
   link/path existence; (b) architecture map vs `src/` tree diff;
   (c) duplicated freshness-note / doubled-heading detector (the `bb20167`
   damage class); (d) report missing from `Reports/README.md`; (e) undated
   "this build" freshness notes. Run by hand; optionally informational in CI
   like `cargo fmt --check`.

### Tier 2 — truth fixes (agents act on these literally)

6. The two README self-contradictions: **R3** (M14 "nothing reads it yet")
   and **R4** (M17 does cover `Plant` — `structural.rs:1513`).
7. **W2** smoke (two wiki pages deny a producer that `backfill_smoke` and
   its guard test prove); **W3** `the-world.md` self-contradiction (rain,
   evaporation, starting plant cover all shipped); **W1** the two damaged
   pages (doubled freshness notes / lost heading in `world-cycles.md`,
   stray trailer in `structural-collapse.md`).
8. **R5** material count (7 → 21), **R6** `kind` list (+`Plant`,
   `Creature`), **R8** link `wiki/` from README.
9. **W5** dated freshness notes on all wiki pages
   (`*Current as of: 2026-08-19.*`), **W4** fire-page coverage (leaf 0.75 is
   the most flammable material in the game and unmentioned; creatures burn;
   snow melts), **W6** snow/seed cross-reference on the powders page.
10. While editing any passage, convert load-bearing `file.rs:NNN` references
    to symbol references (line numbers rot; symbols grep). Opportunistic,
    not a sweep.

### Tier 3 — read cost and remaining accuracy

11. **R7** five missing README status sections (M9 gnome, M10 worldgen,
    weather, ants, M19 sky/visuals), written compactly, plus a table of
    contents. **Changed from plan v1: no wholesale reorder** of existing
    milestone sections — agents navigate by grep, the reorder is a huge diff
    on a contested file worked by another session, and a TOC buys the same
    navigation for 3% of the churn.
12. Subheadings in README's three >120-line spans (the 224-line field-grid
    section first).
13. **R2** Controls table: three corrections (F2/F3/F4 mapping, Enter→`S`
    save, arrow-key fallthrough) and six missing keys (`Y` F6 F7 F8 F9 `L`).
    Human-facing, hence Tier 3.
14. **P1** Stack table: in-use / planned / declined column (rapier2d, mlua,
    earcutr, glam, puffin) and the two wrong milestone labels; **P2** status
    markers carried past M4; **P3** one-line current state atop each
    trailing handoff section + PLAN TOC. **F1** drop CLAUDE.md's stale
    "some older comments still say otherwise"; **F2** add
    `scripts/acceptance.sh` and the seed-sweep invocation to the Commands
    block (review rec #10: the file's strongest convention is currently not
    executable by a cold agent).

### Tier 4 — owner decisions (proposed, not assumed — asked below)

15. **Split PLAN.md's progress log** into `PLAN-log.md` (sibling file; PLAN
    keeps a stub heading + link so `PLAN.md#progress-log` anchors and the
    `rigid.rs` comment stay valid; repoint `README.md:668`). Evidence: 2 of
    ~120 inbound references target the log; PLAN.md shrinks ~40% and its
    live half becomes cheap to read whole. **Recommend: yes.**
16. **CLAUDE.md efficiency pass**, from the 13-rec review, in two clearly
    separated commits: (a) *zero-content-risk*: promote the five bold leads
    to `###`, cluster Conventions under four sub-leads without rewording,
    merge the three true duplications (image-vs-number ×3, liquid `aux` ×2,
    abscission incident ×2), add the ~12-line task-to-rule topic map;
    (b) *content moves*: git-reset forensics, the oscillator rationale, and
    the amputation/liquid-latent gotchas compressed inline with full
    tellings moved to `open-bugs-handoff.md` / a concurrent-sessions note,
    per recs #5–#7. Voice preserved throughout — no sentence rewritten, only
    moved, merged, or pointed. **Recommend: (a) yes, (b) yes.**
17. **`Reports/archive/`** — **recommendation reversed from plan v1: skip.**
    With per-report status in the index, archiving buys an agent nothing (it
    routes by index, not by directory browsing), and the `git mv` breaks
    `Reports/` paths held in other sessions' uncommitted worktrees
    (`.claude/worktrees/` holds eight). Supersession becomes an index
    annotation instead. Owner previously chose archive, so this is asked,
    not assumed.

## Requirement A — the docs will change before execution; nothing new may be lost

The audit is pinned to `565e4b4`; the tree is being worked concurrently
(`ecological-lod-design.md` landed mid-audit; `wiki/the-gnome.md` is dirty
from another session). At execution start, before any edit:

1. **Delta pass**: `git log --oneline 565e4b4..HEAD -- '*.md' wiki/ Reports/
   docs/` plus `git status` — list every doc changed since the audit.
2. **Re-verify before applying**: any finding touching a changed file is
   re-checked against current text; a finding already fixed or superseded by
   the other session is dropped, not re-applied.
3. **Surgical edits only** on shared files — targeted `Edit` diffs, never
   whole-file regeneration, so concurrent additions survive. The one bounded
   exception is the architecture-map section replace, re-derived from the
   file tree at execution time anyway.
4. **Indexes generated from the tree at execution time**, never from the
   audit snapshot.
5. **Census delta**: re-run the marker sweep over files changed since the
   census, and resume the workflow for the outstanding PLAN.md agent, before
   writing `dead-ends.md`.
6. Fresh worktree from current `origin/master` at execution time; rebase
   before push; land each contested file fast.

## Requirement B — a regime so the docs don't rot again

Prevention over cleanup, in the repo's own idiom — cheap mechanical
backstops and event-driven conventions, not scheduled chores:

1. **`scripts/docscheck.sh`** (Tier 1 item 5) — runnable, catches the three
   defect classes this audit found recurring (dead refs, map drift, the
   doubled-note damage).
2. **Conventions added to `CLAUDE.md`**, each a one-to-two-line extension of
   one that already exists:
   - Freshness notes are dated; a change touching a wiki page's behaviour
     updates the date in the same change (extends the existing wiki rule).
   - A new report gets its line in `Reports/README.md` in the same commit;
     a superseding report updates the superseded one's status line (same
     shape as the MEMORY.md index rule).
   - A revert adds its entry to `Reports/dead-ends.md` in the same change,
     with the condition its rejection depends on (gives "a revert keeps the
     knowledge" a *where*).
   - A shipped milestone/feature gets its README status section before the
     work is called done.
3. **Boundary rule** (review rec #13): `open-bugs-handoff.md` owns "is this
   broken?"; `dead-ends.md` owns "was this tried?"; each header points at
   the other for cases that are both.
4. **Structural**: the append-only surface (progress log) separated from the
   read surfaces, so growth stops degrading reading.

## Execution mechanics

Worktree from `origin/master` (another session is active in the shared
checkout; its `src/render.rs` and `wiki/the-gnome.md` are dirty and must not
be swept up). One commit per contested file (`README.md`, `PLAN.md`,
`CLAUDE.md`), landed immediately; explicit paths staged; never `git add -A`.
Commit messages carry the measurement per house style (e.g. "map: 14 of 33
modules listed → 33 of 33; docscheck clean").

Commit order: audit report first (it is the work order), then Tier 1 items,
then Tiers 2–3 grouped by file, then approved Tier 4 items.

## Verification

1. **`scripts/docscheck.sh` clean** after every commit that touches docs —
   this is the check that catches a bad link, a map omission, or a doubled
   note the moment it happens.
2. **Map completeness**: the README module list diffs empty against
   `src/**/*.rs`.
3. **Cold-agent orientation test** (the agent-framing acceptance test):
   spawn a fresh agent with no context beyond the repo and ask it (a) which
   file owns structural failure and what to read before touching it, (b)
   whether per-cell horizontal-first liquid transfer has been tried, (c)
   where to find the status of the tree-architecture work. It must answer
   correctly from the docs alone, citing the index files. Wrong or
   unfindable answers are failures of the deliverable, not the agent.
4. **No source or asset changes**: `git diff --stat` on every doc commit
   shows only `.md` (plus the one new script). `rigid.rs:21` explicitly
   needs no edit under the split design.
5. Commands named in `CLAUDE.md` run in the worktree (`cargo test`,
   clippy, ascii) — unchanged claim, re-verified once.

## Out of scope

Rust doc comments in `src/` (load-bearing, own project — the census *read*
them but changes nothing); `research/` (verified in good order); the
`genome-sweep-*.txt` scratch files at the repo root (transient artifacts,
not documentation — flag to owner separately if wanted).
