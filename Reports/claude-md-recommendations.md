# CLAUDE.md organization review — the thirteen recommendations

**Status: nine landed in `0efeb24`; recommendations 5, 6, 7 and 12 are
approved and still open.** Produced 2026-08-19 by the documentation
overhaul's review agent, recovered into the repo 2026-08-24 from the
session's local plans directory — where it was the only copy, cited by the
audit's delta section as "recorded as a queued follow-up so the approval is
not silently dropped" but not actually reachable from anything in the tree.
Landed/pending status below was re-verified against `main` on 2026-08-24,
not taken from the session's own account.

**What the four open ones have in common:** each moves text across a region
that an unmerged branch also edits — `load-share` beside the oscillator
passage, `plant-branch-angle` beside the git-reset passage, `perf-lock`
inside Conventions. The block is a merge-conflict argument, not a
disagreement with the recommendation. When those three branches land, this
list is directly executable.

**The line numbers are rotted, deliberately.** Every `LNNN` below points
into the pre-overhaul `CLAUDE.md` and none of them survives `0efeb24`. They
are left as written because the *issue* text is the evidence — what was
duplicated, and where — and rewriting them into symbol references after the
fact would be inventing a record. Grep the quoted sentence instead; that is
what the repo's own convention says to do.

**On the token figures:** the review's savings estimates are its own, and
the one word-count pair quoted in `0efeb24`'s commit message was measured
against a stale base — direction right, figures unreliable. Treat every
"~N tokens saved" here as an ordering signal, not a measurement.

*Recovered: 2026-08-24. Review performed: 2026-08-19.*

---

## 1. A task-to-rule topic map

**Landed.** `0efeb24` — "Which rules apply to what you are doing right now".

**The issue.** Rules for one task are scattered across three sections. An agent about to run a parameter sweep needs 'When every setting of a sweep fails the same way, suspect the sweep' (Method, L251), 'A change that moves *nothing* is different evidence' (Conventions, L401), and 'Editing an asset `.ron` does nothing until the next build' (Gotchas, L500); a guard-test author needs L376, L381, L443 (Conventions), L497 and L517 (Gotchas), and L246 (Method). The cross-section link is even explicit in the prose: L253 calls a Conventions bullet 'The sibling of' a Method section two hundred lines away.

**The change.** Add a ~12-line task-to-rule map immediately after 'Where knowledge already lives': one line each for 'running a parameter sweep -> L251, L401, L500-gotcha', 'writing or trusting a guard test -> L376, L381, L443, L497, L517, L246', 'adding per-cell work -> L299, L420', 'measuring liquids/powders -> Metric traps, chunk decomposition', 'touching organism code -> L272, L507', plus rows for the two new Reports indexes. Use section names, not line numbers.

**The benefit.** ~150-200 tokens buys the single highest-value thing an every-session file can do: let a mid-task agent find the rule that applies to what it is doing without re-reading all 5,100 words. Also the natural home for routing to Reports/README.md and dead-ends.md.

## 2. The image-vs-number rule is stated three times

**Landed.** `0efeb24` — duplicate section deleted, both unique payloads folded into the Method preamble.

**The issue.** The image-vs-number rule is stated three times: 'An image tells you *what* and *where*. A metric tells you *how much*' (L167), '"Did it fire at all" needs a counter, not a picture' opening with 'An image shows *what* and *where*; it cannot show...' (L171), and a full section titled 'An image says *what and where*; only a number says *how much*' (L228).

**The change.** Fold the L228 section's two unique payloads (the `examples/plant_probe.rs` pointer and the 40%-of-scale misread anecdote) into the L167 paragraph, and delete the duplicate section. Keep L171 intact - the counter rule is genuinely distinct.

**The benefit.** ~70-80 tokens saved per session, and the rule reads once with full force instead of three times with diminishing returns.

## 3. The liquid `aux` convention is stated twice

**Landed.** `0efeb24` — now one bullet, "Two conventions for `Cell::aux` point opposite ways."

**The issue.** The `aux == 0` liquid convention appears twice in the same Gotchas list: 'On a `Liquid`, `aux == 0` means **full**' (L453) and '`liquid_fill`: `aux == 0` on a `Liquid` cell means **full**, not empty' (L469).

**The change.** Merge the L469 bullet into the L453 bullet (its only unique content, 'Writing a literal 0 fill manufactures a full cell out of nothing', is already implied by 'manufactures water out of nothing' at L457).

**The benefit.** ~40 tokens saved; one authoritative statement of a gotcha where having two slightly different phrasings invites the exact confusion it guards against. Keep the merged bullet inline - it is a pre-file-knowledge trap and must stay always-loaded.

## 4. The abscission incident is narrated twice in full

**Landed.** `0efeb24` — the sweep section carries a one-line pointer to the gotcha.

**The issue.** The abscission/structural-check incident is narrated twice in full: '772 cells against 20,213 at the same setting, from that one line' (L258, sweep section) and '26x outcome difference from the one line, and it masqueraded as "the mechanism is wrong" through eight settings' (L511, Gotchas). Same event, two lessons, ~150 tokens each.

**The change.** Keep the full telling in the Gotcha (which carries the operative do-not-do: 'do not add `schedule_structural_check_around` to a new organism path'); in the sweep section, compress the example to one sentence with a pointer to that gotcha.

**The benefit.** ~70-90 tokens saved with both lessons preserved; the sweep section keeps its rule ('anything that rode along with the mechanism is part of every data point') without re-deriving the evidence.

## 5. The git-reset forensics passage is always-loaded narrative

****Pending**.** Deferred: `plant-branch-angle` rewrites the region beside this passage. The full narrative is still inline in `CLAUDE.md`.

**The issue.** The git-reset forensics passage (L133-146, ~200 tokens) - '**That reset strands stale files whenever the main tree was *behind*.**' through the recovery procedure - describes a rare failure mode of one specific maneuver, in full narrative detail, in every session's context.

**The change.** Keep the recipe (L126-131) plus a two-sentence warning: note which files are dirty before the reset; afterwards, diff anything newly modified against the commits you were behind by, and `git checkout --` any exact inverse. Move the `src/sim/structural.rs` case history to a short Reports note (e.g. Reports/concurrent-sessions.md) and cite it.

**The benefit.** ~120-150 tokens saved while the operative protection stays inline. This passes the always-loaded test only in compressed form: the agent needs the warning at reset time, not the forensic story.

## 6. The oscillator section is subsystem-specific detail

****Pending**.** Deferred: `load-share` inserts sections beside the oscillator passage. Still inline in full.

**The issue.** 'A channel that oscillates by design must be divided out of decisions' (L272-283, ~150 tokens) is subsystem-specific: it matters only when writing a threshold on light or temperature in organism code, and its detail (71 vs 28 tip counts, the temperature forecast) is design-report material.

**The change.** Compress to a two-line gotcha - 'Any threshold on light or temperature must divide out the day/night oscillation; use `field::noon_equivalent_light`, see <report>' - and move the full rationale to the relevant plant/field design report (indexed in Reports/README.md). An agent gating on light already knows which subsystem it is in, so the pointer suffices.

**The benefit.** ~100 tokens saved per session; the load-bearing function name and rule stay inline where a cold agent will still hit them.

## 7. The amputation gotcha is an open bug, and belongs with the open bugs

****Pending**.** Deferred with 5 and 6 as the second narration move-out. Both entries — the amputation gotcha and the liquid-heightfield latency note — are still inline in `CLAUDE.md`.

**The issue.** 'A structural check scheduled mid-organism amputates it' (L507-516) is, by its own words, an open bug with a workaround: 'Until the support search anchors properly, do not add `schedule_structural_check_around`...'. The knowledge table defines Reports/open-bugs-handoff.md as exactly this: 'Working reproductions, what has been ruled out... Read this before touching a listed area.'

**The change.** Move the full entry to Reports/open-bugs-handoff.md; leave a one-line gotcha: 'Do not add `schedule_structural_check_around` to any organism path, and treat Phase 3 damage results as contaminated - open-bugs-handoff has the details.' Same treatment for 'The liquid heightfield bodies in `liquid.rs` are **test-only today**' (L525) - a latent-bug note that belongs in open-bugs-handoff and as a source comment in liquid.rs.

**The benefit.** ~130 tokens saved combined, and the entries live where they will be maintained: open bugs get resolved, and a CLAUDE.md gotcha that outlives its bug becomes exactly the stale 'this file said cargo test still works; it does not' problem the file already documents about itself (L483).

## 8. The knowledge table hardcodes individual report rows

**Landed.** `0efeb24` — the two per-report rows are gone; `Reports/README.md` has the row.

**The issue.** The knowledge table hardcodes individual report rows - '`Reports/fracture-mechanics-design.md` | Why rock breaks the way it does...' and '`Reports/load-model-handoff.md` | **The next step on destruction**' - which cannot scale to 40+ reports and goes stale the moment a handoff is picked up (the load-model row may already be: MEMORY says the plan awaits sign-off).

**The change.** When Reports/README.md lands, replace the generic '`Reports/*.md`' row and the two per-report rows with one row: '`Reports/README.md` | Index of every design report with per-report status - check status before trusting a report or writing a new one.' Keep open-bugs-handoff.md and design-philosophy.md as inline rows (they are read-before-you-know-your-files). Add one Conventions clause: a session that supersedes or adds a report updates its status line in the index.

**The benefit.** ~50 tokens saved now, and the table stops being a maintenance liability; per-report status in the index also directly serves the existing warning 'not something to take from a report on faith' (L53).

## 9. Nothing routes to a dead-ends index

**Landed.** `add3fe3`/`0efeb24` — table row, and "A revert keeps the knowledge — and gets an address."

**The issue.** Do-not-retry knowledge currently routes only to source comments ('approaches that were tried and reverted and must not be retried', L77) and diffusely to open-bugs-handoff. Meanwhile the file itself establishes that rejections are conditional: 're-test any do-not-retry entry of that shape after something changes its condition' (L409). Reports/dead-ends.md is arriving to hold exactly this, and CLAUDE.md has no routing to it.

**The change.** Three edits: (a) add a knowledge-table row: '`Reports/dead-ends.md` | Tried-and-reverted approaches, each with the condition its rejection depended on - check it before proposing or retrying anything in a listed area, and check whether the condition still holds.' (b) Amend 'A revert keeps the knowledge' (L361) to name dead-ends.md as where the record goes, alongside the #[ignore]d reproduction. (c) Cross-reference it from L77's source-comments paragraph.

**The benefit.** Closes both directions of the loop for ~60 tokens: agents write reverts to a findable place, and agents read it before burning a session re-deriving a known dead end - the exact failure mode the file says the project 'keeps re-learning the expensive way' (L6).

## 10. The seed sweep is mandated but never named

**Landed.** `0efeb24` — `scripts/seedsweep.sh` is in the Commands block with the convention as its comment.

**The issue.** The seed sweep is mandated but never named. 'A seed sweep caught each in one command. So build the sweep *before* changing a model that governs procedural content' (L393) - yet the Commands section (L84-88) lists four commands and none of them is that one; a cold agent told to run it must reverse-engineer which invocation the passage means.

**The change.** Add the actual seed-sweep invocation (or the script path, if it is `scripts/acceptance.sh` with a seeds argument) to the Commands block with a comment tying it to the convention: '# order-statistic seed sweep; run BEFORE changing any model over procedural content'.

**The benefit.** Makes the file's own strongest convention executable by a cold agent in one copy-paste; a rule whose command cannot be found is a rule that gets skipped under deadline, which is how it 'happened twice in one session' (L390).

## 11. Five Method rules are invisible to a heading skim

**Landed.** `0efeb24` — all five promoted to `###`.

**The issue.** The Method section's first five post-list rules (L171-215) are bold-lead paragraphs while the later eleven are ### headings, so 'Resolve an ambiguous complaint before building anything' (L198) and 'Ask what a metric counts when nothing is wrong' (L205) are invisible to a heading skim or an outline view, unlike their peers.

**The change.** Promote the five bold lead sentences (L171, L181, L198, L205, L211) to ### headings matching the later sections. No wording changes.

**The benefit.** Zero token cost; the whole Method section becomes scannable and greppable by heading, which is how a mid-task agent actually consults a 200-line section.

## 12. Conventions is a 93-line undifferentiated list

****Pending**.** Deferred: the re-clustering moves bullets across the whole section, and `perf-lock` edits inside it. Conventions is still a flat list.

**The issue.** 'Conventions' is a 93-line undifferentiated bullet list (L355-449) mixing test design ('A guard test must be able to fail for the *replacement* artifact'), tuning epistemology ('A constant nobody can tune in either direction may be a counterweight'), performance ('Measure a cost against the state the optimisation exists for'), and process ('Prefer an independent review before significant commits').

**The change.** Reorder the bullets into clusters under four bold sub-leads - Tests and guards / Tuning and sweeps / Performance / Process and records - without rewording any bullet. This also puts 'Two fixes failing the same way' adjacent to its declared sibling from the Method section via the topic map.

**The benefit.** Findability at essentially zero token cost (~15 tokens of sub-leads); the voice survives untouched because no sentence changes, only order.

## 13. Two indexes would claim the same knowledge

**Landed.** `0efeb24` — the row now reads "`dead-ends.md` owns \"was this tried?\"; this owns \"is this broken?\"".

**The issue.** For the fourth question's 'anything else missing': the file's opening self-description of open-bugs-handoff.md still claims it holds 'what was tried and reverted' (L72), which will overlap and compete with dead-ends.md the day that file exists - two indexes claiming the same content is how an agent checks one and misses the other.

**The change.** When dead-ends.md lands, trim the open-bugs-handoff row to 'Open bugs: working reproductions and what has been ruled out by measurement', letting dead-ends.md own tried-and-reverted; have each file's header point at the other for the boundary cases (a revert that is also an open bug).

**The benefit.** Prevents the two new indexes from silently splitting the same knowledge; a cold agent gets one unambiguous place per question ('is this broken?' vs 'was this tried?'), which is the entire value of adding the indexes.

