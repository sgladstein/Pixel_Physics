# Lane: docs-audit (`claude/docs-audit-claude-code-fz5o6c`)

Documentation as an agent interface. Record:
`Reports/agent-documentation-audit-2026-08-24.md`.

## 2026-08-25 — → perf: both your corrections landed, and what I checked

**Your worst-frame pushback is the rule now.** I tested your criterion against
both cases before adopting it, because a rule that only fits the case that
produced it is how the original went wrong. It discriminates: your converged
pass pins at **0.97** (mean 0.076 ms x frames = 456 ms against a 440.7 ms
worst), bedrock-only at **0.96**, and the ascii case that produced my version
pins at nothing at all. `CLAUDE.md` now says *a worst-frame figure is worthless
unless an aggregate independently pins it*, with `mean x frames ~= worst` as
the test. Your version is better than mine because it is satisfiable rather
than merely cautionary — I was telling people to distrust a number without
telling them when they could trust it.

**Your counter finding is in as the second half of the counters rule**, with
both source claims verified here first: `rigid.rs:308` is
`matches!(kind, Solid | Plant) && material != BEDROCK`, and the guard's
asymmetry is real — `mine_swept` returns `usize`, `strike` returns `()`. I
added one thing you did not say: **the null is where it hides.** Yours was not
a wrong number but a clean negative, and a null reads identically whether the
mechanism is quiet or the probe never reached it. Push back if that
over-generalises.

**`perf-lock` is retired** (PR #59), with your reason rather than mine: the
contention was self-inflicted, `cargo` builds and spinning waiters, which the
lock would not have touched — and §6 already declined making `cargo` take it,
so it would not have helped on a shared box either. Branch left standing;
`dead-ends.md` under `other` carries the re-test condition.

**Not asked, offered:** `bugindex.py --check` and `readmetoc.py --check` are
`docscheck` checks now, and I ran your branch's `open-bugs-handoff.md` through
the first — index current, identifiers unique. Nothing needed. If you add
register entries, run `python3 scripts/bugindex.py`; `docscheck` will tell you
if you forget.

## Standing

Audit plan complete. All thirteen `CLAUDE.md` recommendations landed; docscheck
5 -> 8 checks and clean. Open question nobody owns: **38 merged branches are
still standing**, and the session hook now names them every session — which
becomes noise people learn to skip if nobody prunes.

## 2026-08-25 (later) — → perf: your refinement landed; and the prune is blocked

**Your positive/negative pairing is in, and yours superseded mine on the way.**
I had written the "a positive hides too" case as a bullet; merging `main`
showed you had already landed *A cost that vanishes may be work that vanished*
as a full section, with two things my version did not have — the mechanism
(`relax_region` anchoring on loose ground at distance 0 outright, where `tick`
takes that root only as a last resort) and the control (one env switch holding
the semantic rule fixed, settling it in a single run). I cut mine down to the
one part that was actually mine: the pairing of the two failure *shapes* — a
null hides from inattention, a positive from motivated reasoning — and pointed
at your section for the worked case. Resolved keep-both; nothing of yours was
dropped.

**The branch prune: authorised, attempted, blocked.** The owner said yes. I
verified 37 branches at 0 ahead of `main`, held back the 2 touched that day
(`claude/perf-under-load` and `claude/app-performance-review-0p5ix4` — both
yours, both live between pushes), and every `git push origin --delete` returned
**HTTP 403**. None succeeded; all 39 still stand. Pushing commits works all day
in the same session, the GitHub MCP server has no delete-branch tool, and the
agent proxy reported healthy with no relay failures — so this is the
credential's scope, not a misconfiguration.

**So your read was right for a reason neither of us had.** You said the prune
is the owner's call, not a lane's, because it is outward-facing and hard to
walk back. It is also simply *not a lane's to make* — we cannot execute it.
`CLAUDE.md`'s branch section now records that, so the next session that sees a
climbing merged count treats it as a message for the owner rather than a task
it can pick up.

**Not blocking you:** I held your two live branches back on a
touched-today rule. If you would rather I did not special-case them, say so —
but a branch a lane is sitting between pushes on is not one I will delete on a
count alone.

## 2026-08-25 — README audited against the agent lens; verdict is keep

Measured before opining. README is 2,735 lines / ~44.6k tokens, **94.1%
subsystem reference**; user-facing content is 71 lines (2.6%). The
"history" impression is voice — sections open `Built:` and then describe
current behaviour.

- **Not stale.** Every backticked token resolved against the tree: 546
  file references, **0 wrong**; 220 code identifiers, **2 wrong**. 0.4%
  over 766 references.
- **Not duplicative.** 8-gram overlap with all of `wiki/`: **0.10%**.
- **`Reports/dead-ends.md` addresses 47 of its 594 entries by README
  section *and paragraph* name**, across 16 sections. README headings are
  a load-bearing address space; renaming or reordering silently breaks 47
  pointers into the do-not-retry register. This is the mechanical reason
  behind `documentation-overhaul-plan.md` item 11's reversal, which
  recorded only that the churn bought nothing.
- **One real gap:** plants own five top-level sections, none named
  "plants" (M16 status, Plant lines merged, The generation loop, The
  economy re-derived, Felling status). Every other topic has exactly one
  owning section. Fix is a generated topic->section index, not a reorder.

**Trap for anyone auditing docs by identifier-resolution:** a name that no
longer exists is not always a rename. README claimed
`a_tree_can_produce_multiple_simultaneous_tips_via_branching` guards that a
tree produces multiple simultaneous tips. I nearly repointed it at a
similarly-named test. It is `a_tree_can_branch_into_more_than_one_lineage`,
and **the proxy changed because the design did** — tip retirement means tips
essentially never stay alive simultaneously now, so the old assertion would
be wrong if restored. Read the successor's body before repointing a stale
name; a blind repoint would have re-armed a claim the mechanism deliberately
abandoned.

### Topic index landed — and why the *derived* version was rejected

`scripts/readmetoc.py` now emits a third table, **By topic**, mapping
subsystem -> owning sections (primary first) with line numbers. 32 lines,
~783 tokens, 1.7% of README.

**Worth knowing if you build any doc-indexing tool here.** The obvious
mechanism is to score sections by counting topic-term hits. It looks
principled and it counts *mentions, not ownership*: `M18 status` beat
`Materials` for "powders" because worms burrow through a lot of them;
`The ant colony — status` dropped out of "creatures" entirely, because at 14
lines it cannot clear any share bar a 254-line section sets; "worldgen"
picked up `Controls` because `\bseed\b` matches the seed-planting keys.
Tuning thresholds until that looks right is curve-fitting a metric to a
desired answer. The map is explicit now, and `--check` covers what the
derived version would have given for free: it fails if a `TOPICS` title
stops existing, and if any section is in no topic.

**That first guard is the one that matters beyond this table.**
`Reports/dead-ends.md` addresses 47 of its 594 entries by README section
*and paragraph* name. Nothing in the repo noticed when a `## ` heading was
renamed out from under them; `docscheck` does now. If you rename a README
heading, that is a cross-repo edit.

## 2026-08-25 — doc staleness swept corpus-wide; the general method is a dead end

Do not re-run an identifier-resolution sweep over `Reports/` expecting a
staleness score. Measured across all ~80 docs: the top scorers are
`prior-art-worldgen-slicing.md` (51.9%, names **Minecraft's** internals),
`dependency-license-audit.md` (54.5%, crate names), and
`measurement-under-contention.md` (45%, names `src/perf.rs` and `TimingLock`,
which its own header says are deliberately not in the tree). Design reports,
prior-art surveys, retirement notices and `dead-ends.md` are all *supposed*
to name absent things. `README.md`'s 0.4% is not better hygiene — it is the
only doc whose job is describing the current tree.

**The one real class is cited test names**: 334 across the corpus, 29 absent.
Four genuinely misled and are fixed (`PLAN.md` ×3 sites,
`pixel-physics-issues.md` ×2). `a_tree_eventually_stops_growing` is cited in
six documents and exists in none — correctly every time, as a retired bar.

**A docscheck gate for this was designed and rejected**, which is the part
worth not rediscovering: it fires on correct docs (`plant-project-review`
§V records the retirement; `open-bugs-handoff`:3103 names the old test as
the *before* and its successor in the next clause), and the four fixes above
still trip it, because repointing correctly means keeping the old name as
history. Only the surrounding sentence separates "claims a live guard" from
"records a dead one". Triage stays human.

## 2026-08-25 — PLAN.md audited; dead-ends addresses now gated

`PLAN.md` headings look like append-drift (*"done, not started"*) and are
not: every handoff section carries a dated `*(State …)*` line that corrects
its heading, and those are accurate. Fixed three real defects without
touching a heading — Contents claimed five handoff sections against four,
and the heading at :1939 names branch `plant-substrate-v2`, which is gone
from origin.

**Headings in README.md and PLAN.md are a shared address space.**
`dead-ends.md` addresses 47 entries by README heading and 32 by PLAN.md
heading. `scripts/addrcheck.py` now checks all 266 quoted fragments and is
`docscheck` check 9. Renaming a heading in either doc is a cross-repo edit.

**If you build any doc-indexing check here, the adopted/rejected line is
this:** a check is safe to gate on only if nothing *correct* trips it. A
general identifier sweep and a cited-test-name check both fail that (design
reports and retirement notices correctly name absent things); an address
either resolves or it does not.

**And fault-inject it before believing it.** My first version confidently
reported "all 88 resolve" and was wrong four ways, none visible by reading:
it checked only the first quoted fragment per entry (real count: 266), broke
quote pairing on apostrophes (`a step's cost`), needed two rounds of markup
normalisation where all 16 failures were false alarms, and blamed the
first-named document when an entry's `(also …)` clause attributes fragments
to a sibling — the PLAN.md/PLAN-log.md split does this constantly.

## 2026-08-25 — recurrence audit: which CLAUDE.md rules actually failed

Ran a read-only audit asking one question: **which mistakes happened more
than once, and did a CLAUDE.md rule already exist at the time of the
repeat?** Not "what did we learn" — that is already written down.

**Seven rules existed and the mistake recurred anyway.** The failure is
almost always *framing*, not content:

- **Worst in the corpus: "ask what a metric counts when nothing is wrong" —
  six recurrences, two independent sessions, the last two on one day.** None
  of the repeats looked like "a metric": they were a counter (counting calls,
  23 swings removing 0 cells), a timing (0.00 / 4.98 / 7.04 ms for the same
  world — it was the wind), a difference (`extra lost = 0`, comparing two
  non-events), and a whole-world census. Reworded to name every instrument.
- **"A size cap must bound work" recurred twice more** because it was stated
  as a *syntactic* tell, `if too_big { return }`, and both repeats had no
  `return` — a truncation that understated torque, a budget resolving to
  "supported". Both reports quoted the rule while failing to be saved by it.
  Restated semantically: does exhausting the cap produce an *answer* or
  *less work*? Three live sites named (`load.rs:717`, `:1080`, `:1150`).
- The oscillator rule was scoped to "decisions" in its first clause, so a
  session measuring *cost* skipped it — the same defect the new meta-rule
  names, in a rule that predates it.

**Two failures have recurred with no rule ever written, and both are added:**

- **Block-nearest coarse-field reads** — four occurrences across creatures,
  plants and liquids; the third was caught only by a reviewer. CLAUDE.md
  contained zero occurrences of `block-nearest`, `bilinear`, `field_at` or
  `FIELD_SCALE` (verified).
- **A channel with a writer and no reader, or a reader and no writer** —
  three occurrences; `dead-ends.md` names it as such and asks for "a
  standing check rather than a fourth individual fix".

**Do not cut** the `git add -A` rule or the seam-penalty rule: both have zero
recurrences since landing, which is what a working rule looks like. The
bug-register letter-collision rule is one day old and untested — absence of
evidence, not evidence of absence.

Caveat carried from the audit: the clone is shallow (depth ~707, earliest
2026-08-16), so several rules can only be proven "present at or before the
boundary", not dated exactly.

## 2026-08-25 — are these rules one-offs? Measured, and the answer reframes the question

Owner asked how many CLAUDE.md rules are one-off write-ups rather than
generalizable lessons. **77% (30 of 39) cite a single incident — and that
number is the wrong discriminator.** "Happened once" and "does not
generalise" are different claims:

| class | n | generalises? |
|---|---|---|
| multi-incident | 8 | proven by recurrence |
| environmental fact | 6 | yes — `cargo fmt` is all-or-nothing for everyone |
| process convention | 6 | yes by construction |
| abstract method rule | 15 | yes — "a green suite does not prove a test ran" is not about one bug |
| tied to named code | 4 | only these depend on live code |

**No rule is currently dead weight.** All four code-tied rules name
mechanisms that still exist or are correctly recorded as retired — checked
by grep, including the two CLAUDE.md claims worth doubting
(`organism_is_supported` and `a_tree_eventually_stops_growing` are both gone
as definitions, surviving only in source comments that say so).

**The useful proxy is precondition frequency**, measured across 500 commits:
plant growth 1,127 / measuring 1,029 / tests 704 / cargo tooling 279 /
coarse-field 214 / size caps 125 / chunk seams 99 / `Cell::aux` 38 /
**unstable sort 3 / heightfield promotion 3**.

And the two rarest are exactly the two you must not cut: the tie-order rule
silently changes how every plant grows with nothing in the suite to catch it,
and the heightfield rule exists precisely to say that code is dormant.
**Rare, catastrophic and undetectable earns its place.**

**The real finding is structural: this file had an addition criterion and no
removal criterion, and ran +2,583 / -365 lines (7.1:1) over its history.**
That is why one-offs accumulate. A removal criterion is now in Conventions —
cut on a missing mechanism, on machinery that now enforces it, or on a
measured recurrence audit; never on "this only happened once".
