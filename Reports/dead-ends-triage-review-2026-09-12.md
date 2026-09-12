# Review of the dead-ends triage (PR #331), and the plan to carry it forward

**Status: review complete, 2026-09-12. Plan not yet enacted.** Written for the
session that picks this up cold; every claim below was checked against the
tree at the PR head (`e348057a`, 0 conflicts with `main`, CI green) and the
checks are cited so they can be re-run. `[PR #331](https://github.com/sgladstein/Pixel_Physics/pull/331)`
is the work under review; its resumable state is
`Reports/dead-ends-triage-handoff.md` on that branch.

## Verdict, in four lines

1. **The register write-backs are right and are the durable value.** Every
   `CONDITION MET` annotation spot-checked (structural:015, :038, :040,
   plants:009, :019, :069, :071, :074, creatures:066) resolves to real code or
   a real asset. Land those.
2. **The machine-readable results are wrong for about one entry in twelve**,
   by construction: the content key collides, the tables never learned the
   run-order's own demotions, and the index check neither gates the generated
   file nor leaves the tree clean. None of that is visible from the prose.
3. **The run-order's #1 survives a hostile read; #2 and #3 do not stand as
   written.** plants:124 is the strongest candidate in the register and has one
   design trap the check file did not name. structural:074's condition was met
   on 2026-08-22 and two documents already say so. structural:013 is moot —
   both unauthored solids opt out of the check it fears.
4. **The 3-of-40 rate replicates (4 of 40 with a neutral prompt), but it
   measures the wrong side.** The PR never tested whether its *candidates*
   stand, and blind they mostly do not: 14 of 20 closed, 0 of 10 controls
   reopened, five closures adjudicated by reading and all five upheld. The
   candidate table is a ~40%-precision list, not a floor. §4.

## 1. What holds

- **PR mechanics.** 32 commits, no Rust changed, all nine CI checks green on
  the head, `git merge-tree` against `main` at 0 conflicts. `main` has since
  added three `rendering` entries (PR #344) with the section count updated, so
  `deadendindex.py --check` still passes after a merge.
- **plants:124, verified independently.** `organism.rs:7708` `pub q_peak`,
  `:7730` `pub q_now` (*"the same basipetal sum, before the high-water max"*),
  `:7727` names `break_buds`' defect as the consumer that wants the pair;
  `break_buds` (`plant.rs:9018–9219`) reads neither; `plant.rs:7335` still says
  the prerequisite *"is not built yet"*. All four specifics in the check file
  are correct.
- **The nine write-backs.** `load.rs:240` records `GRANULAR_CAPACITY_DIVISOR`
  removed by name; `load.rs:2137` carries the `parent.is_none() &&
  rests_on_ground` gate; `cross_section_axis` at `plant.rs:12017` and
  `pipe_ratio` at 5.5/14.0/16.0 across the species files; `noon_equivalent_light`
  read at `plant.rs:3981`; `ant.ron:629` carries the 1,601 / generation-13
  numbers. The register text of each annotation matches what the code says.
- **The calibration story.** The keyword truth set was worthless for the
  reason given, and inter-rater agreement is the right replacement. Nothing
  to redo there.
- **The live-flag finding and the structural:005 packing correction** are
  both measurements with controls, and stand.

## 2. What is wrong, most expensive first

### 2a. The content key collides, and 19 verdicts are attached to the wrong entry

`stable_key` hashes *section + address* (`scripts/deadendindex.py`,
`stable_key`). The docstring says the register "deliberately lists some dead
ends twice" and that same-key rows are those duplicates. **Most are not.**
Thirteen keys are shared by 34 rows; reading the claim text, **19 of those
rows are distinct dead ends filed under one report-section address**:

| shared address | rows | what they actually are |
|---|--:|---|
| `next-session-handoff.md §3 'must not be retried'` | 5 | five different structural levers (span 16→40, settle scheduling, torque/section, graded attachment, intact-as-exemption) |
| `next-session-handoff.md §1c-i (commit c709b4c)` | 5 | five different crack-pattern attempts |
| `open-bugs-handoff.md §1 'Whiskers'` | 4 | four different whisker fixes |
| `next-session-handoff.md §1a-ii` | 3 | three different arch-relief attempts |
| `next-session-handoff.md §1b` | 2 | **the divisor-as-cap entry and the relax=1 staleness entry** |
| four more pairs | 8 | wall BCs, gust monopole vs wind forcing, sand displacement, climb gate |

The join collapses each group to one `screened.tsv` row, so one verdict
stands for all of them. It has already done damage: the write-back of
`structural:038` (divisor gone, `LANDED`) is carried by `structural:039`
too, whose subject is *"stale distances are not this bug"* and whose re-test
clause demands a `relax=1` re-baseline. The run-order lists structural:039
as that re-baseline while the table says it landed. Only the four
`other:045/058`, `:046/059` rows look like the genuine duplicates the
docstring meant.

**Fix:** key on section + address + the first ~160 characters of the claim
(the text after the bold address). Rows that still collide are true
duplicates and may share. Then **re-screen the 19** — they never had a
verdict of their own — and regenerate. (~2.5k tokens each, one sitting.)

### 2b. `--check` is not a gate on the generated files, and it dirties the tree

`main()` calls `write_outputs()` before it looks at `--check`, so the check
**always rewrites** `Reports/data/dead-ends-index.tsv` and
`dead-ends-skeleton.md`, and it compares nothing against what is committed —
it only verifies the `## section (N entries)` heading counts. Three
consequences, each measured:

- **After merging `main` into the branch**, `--check` exits 0 and leaves both
  files modified (the three new `rendering` rows). Nothing says the
  committed index is stale, which is the one job CLAUDE.md gives docscheck's
  generated-file gates.
- **In a depth-1 clone** (cloned from GitHub, PR head), `--check` exits 0 and
  rewrites all 785 rows of both files: every `written`/`effective` date
  becomes the boundary commit's. The docstring promises *"a shallow clone …
  `--check` fails loudly"*; the code only fails when blame returns *no* dates,
  and a shallow blame returns wrong ones. CI's `docscheck` job checks out at
  default depth (`ci.yml:404`, no `fetch-depth`), so CI has been running this
  regeneration with wrong dates and passing.
- Every cloud session is shallow (the SessionStart hook reports depth), and
  CLAUDE.md asks for docscheck after every merge — so as merged, this would
  put a 785-line diff in two tracked files into every session's tree.

**Fix:** (1) `--check` computes in memory and never writes; (2) it compares
the committed TSV and skeleton against the regeneration **on the columns
that do not come from blame** (`written`, `effective`, `pre_cluster` are
excluded, or better, moved out of the committed artifact into a `--dates`
report); (3) regeneration refuses on `git rev-parse --is-shallow-repository`
with the message the docstring already promises. Positive controls, both
required before calling it fixed: delete one entry from the register and
watch `--check` go red on the row count; run it in a depth-1 clone and
watch `git status` stay clean.

### 2c. The tables never learned the run-order's demotions

`screened.tsv` and `candidates.tsv` are described as "the machine-readable
results", and `candidates.tsv`'s **first row is `creatures:039 EXPIRED`** —
the entry the PR body, a PR comment and `check-creatures-039.md` all
withdraw. A later session reading the table re-walks it. The same is true of
every demotion the deep read made:

| entry | table says | should say | why |
|---|---|---|---|
| creatures:039 | EXPIRED 2? | DEAD (condition unmet in substance) | `flitter` has no pheromone economy; the check file says so |
| field:003 | EXPIRED 3 | COSTED (condition unmet) | clause: *"retry if tile storage stops being a HashMap"*; `fxhash.rs:114` `ChunkMap<V> = FxHashMap<ChunkCoord, V>` — still one. The screener's reason (a serial-fallback fix) is about a different sentence of the entry |
| structural:039 | LANDED | its own verdict, re-screened | collision casualty (§2a) |
| structural:013 | UNWIRED | DEAD on condition (see §3) | both unauthored solids opt out of the check |
| plants:044 | EXPIRED 2 | COSTED / holds | clause holds *"while income is per-column sky visibility"*; income is still `Σ ambient_light_above(x,y) · water_status` per leaf (`plant.rs:9070`, `:9404`), and `:4407` still calls it the per-column sky cast. Soil nutrient prices construction, not income |
| powders:013, destruction:050, plants:017, plants:098 | EXPIRED | demoted by the deep read, **reason unrecorded** | the run-order says 15 of 24 were demoted; six of the fifteen have no written reason anywhere |

**Fix:** relabel the five, record the four missing demotion reasons in
`screened.tsv`'s reason column (one sentence each, citing source), and
regenerate `candidates.tsv` from `screened.tsv` rather than maintaining it
by hand.

### 2d. Four documents carry three different sets of numbers

| figure | handoff | revival report | PR body | tables |
|---|---|---|---|---|
| DEAD | 495 | 497 (table) / 495 (§error rate) | 497 | 497 |
| CONFOUNDED | 57 | 57 | 59 | 59 |
| candidates | 118 | 111 / "the 118" (last line) | 111 | 111 rows, 109 keys |
| EXPIRED | 21 | 12 | 12 | 12 |
| no `Re-test when:` clause | — | 71 | 71 | 65 |
| `CONDITION MET` markers | — | — | 13 | 16 |

`Reports/README.md`'s two new index lines are staler still: the revival
line says *"495 … 118 (15%) … one case verified end to end — creatures:039"*,
and the handoff line says *"in flight … 306 entries screened so far"*. The
handoff doc itself says screening is complete and then lists the pre-write-back
counts. **Fix:** one pass, numbers taken from the regenerated tables, and the
README lines rewritten (the handoff line should say *complete*).

### 2e. The watchlist misses every surviving candidate

`--watch` lists 35 entries. **None of the six the run-order kept is on it**
(plants:124, structural:074, structural:013, plants:058, plants:044,
other:080), nor creatures:039. Its grammar wants *"once X exists/lands/ships"*
and the strongest entry in the register says *"once a monotone high-water
memory … can distinguish"*. Not a defect to fix by widening the regex — the
PR already measured that the obvious wider filter is worse than random. It
is a reason not to describe `--watch` as coverage of anything. See §5 for
the mechanism that could actually find these.

## 3. The run-order, re-read against source

**plants:124 — keep at #1, with one correction the check file omits.** The
check says *"mobilise on the deficit `q_peak − q_now`"* without saying **at
which cell**, and per cell it is a trap the tree has already sprung once:
`accumulate_support` walks a *spanning tree* over what is, for a thickened
trunk, a blob, so `q_now == 0` across most of a trunk's girth means *"not on
this tick's path"*, not *"carries no foliage"* (`plant.rs:8103`, `:10911`,
`:13230` — a per-cell rule keyed on it took a stand from 3,437 cells to 704).
The die-back rule survived by being whole-plant. So the deficit must be
read **once per organism at the bole** (the cell the anchor walk starts from,
`support == 0`), where the basipetal sum is the whole live crown and no path
artifact exists. Second trap: **abscission lowers `q_now`**, so a healthy
tree in steady shed has a standing deficit and every autumn reads as damage.
The control arm's deficit distribution over a full day/season cycle has to be
measured *first*, and the mobilisation threshold sits above its p90 — this
is CLAUDE.md's *compare two runs* and *divide the oscillator out* rules,
both. Third: the instrument. `plant_severance`'s `sever` arm cuts at the
soil line, which removes the roots' water path along with the crown, so
*"did it rebuild a crown"* is confounded with *"did it dry out"*. The arm
plants:124 needs is a **mid-crown cut that leaves the root system attached**
— and that is the same instrument structural:074 asks for. Build it once.

**structural:074 — condition met on 2026-08-22, and two documents already
say so.** `Reports/open-bugs-handoff.md` §0d: *"`structural::organism_is_supported`
no longer exists … replaced by `plant::anchor_support` … a Dijkstra from the
anchors outward … with no span budget to run out of … a check fired mid-crown
does not amputate."* `.claude/rules/src-sim-cells.md` carries the same in its
`schedule_structural_check_around` bullet and scopes the contamination to
*"any Phase 3 damage result written while the old search was live."* The
check file found the deletion in code and missed that the register's own
neighbours had recorded it. The 26× mechanism is gone by construction; what
is genuinely open is a **positive control on the new search** — a mid-crown
cut removes the cut-off subtree and nothing else. Worth an hour once the
instrument exists; not a prerequisite for anything. Two stale doc comments
still assert the hop bound in the present tense (`plant.rs:11741`, `:14022`)
and should be pointed at `anchor_support`.

**structural:013 — moot, and the run-order's ethos payoff is wrong.** Both
unauthored solids, `log` and `nest`, carry no `max_unsupported_span`, which
defaults to `u16::MAX` (`material.rs:1071`, `default_never_u16`), and
`capacity_within` returns `i64::MAX` for that sentinel
(`load.rs:1629`, *"does not participate in the structural system at all"*).
`log.ron:182` says the opt-out is deliberate and *"the point of the tier"*.
So neither material is ever evaluated in bending, and directional costs on
them change only the support distance of whatever rests on them. *"A felled
log bridging a hollow is exactly as strong hanging as standing"* is true and
is by design — it never breaks structurally at all. The check file's own
positive control (set `above` to 60, see if landed logs shatter) would have
returned "moot". Every solid that *does* participate authors the triple.
**Write back:** condition holds only for a participating solid, none is
unauthored today; re-test when a new solid with a finite span ships without
the triple. Optionally convert to a guard: a test reading the raw `.ron` text
that every material with a finite `max_unsupported_span` authors all three
costs explicitly.

**plants:058, other:080, rendering:012, structural:039** — as the run-order
has them, with structural:039's table row restored (§2a). plants:044 is
dropped (§2c). **plants:081 (folded into other:080)**: its condition, a
per-order tropism reference, did ship — that part of the screen is right —
but the run-order gives no arm for it; leave folded.

## 4. The two rates, measured again independently

The PR's 3-of-40 was produced by an adversary prompt that told the rater to
*"expect most to hold"* and that *"a run reopening half of them would not be
rigorous"* — an anchor that can only push the count down, written by the
session whose screen it was testing. Two fresh blind samples were drawn
(seeds 777 and 31337, different from the PR's 4242), rated by an Opus agent
that had the main checkout, no triage data, and no anchoring instruction:

- **Under-call (sample A): 40 random `DEAD` entries, labels stripped, "does
  the recorded evidence support the rejection today, verified in source".**
  **4 of 40 reopened**, every one cited to source and two re-verified here:
  `destruction:034` holds *"until materials ship finite `ignition_temperature`"*
  and `wood.ron:123`, `log.ron:149`, `deadleaf.ron:168` now do, while
  `explosion.rs:2088` still says no shipped material sets one; `field:008`
  records `LIGHT_DECAY` 0.997 as the answer and `field.rs:152` is 0.95 with a
  doc saying the channel no longer carries sunlight at all. The other two:
  `destruction:004` (the log palette it rejects was replaced by copying
  `wood.ron`'s, `rigid.rs:3986` carries the shade) and `field:027` (the
  "second mechanism" its clause demands shipped, `field.rs:1263–1290`).
  Pooled with the PR's own sample: **7 of 80, 8.75%**, so the PR's number is
  corroborated and the anchoring in its prompt did not move it. Note what the
  seven are: **six are stale rather than revivable** — the world moved and
  the entry was resolved or superseded elsewhere without a write-back — and
  only `destruction:034` is a condition met with the re-test still owed. That
  is the PR's own structural finding again, from the DEAD side.
- **Over-call (sample B): 30 entries — 20 candidates (12 CONFOUNDED, 4
  COSTED, 3 SUSPECT-INSTRUMENT, 1 EXPIRED) blended with 10 `DEAD` controls,
  labels stripped, the rubric applied cold.** The rater **closed 14 of the 20
  candidates and reopened 0 of the 10 controls.** Exact label agreement
  16/30 — against the PR's 38/42 on a random set that was mostly `DEAD` and
  `META`, where agreement is cheap. By class: CONFOUNDED **2 of 12 confirmed**
  (7 → DEAD, 1 → META, 1 → LANDED, 1 → RE-TESTED), SUSPECT-INSTRUMENT **0 of
  3** (all three re-read as META — the thing rejected *is* a metric), COSTED 3
  of 4, EXPIRED 1 of 1 (structural:074, found independently via
  `plant.rs:8398` `anchor_support`).

**Five of the fourteen closures were adjudicated by reading, and the blind
rater was right in all five** — the same result the PR's own calibration got
when it adjudicated its keyword set. `structural:062` rejects on two counts
that survive deleting the confound (undercut went to zero failures; at the cut
the mean equals the minimum). `liquids:046` is arithmetic: whole-difference
transfer overshoots whatever the sweep direction does, and half-difference is
strictly better. `creatures:022` — **showcased in the PR body and the revival
report as the exemplar CONFOUNDED** — records that the confounded sweep was
*"invalidated and restarted"*; it is a process entry, already handled, and
Question 1 of the rubric claims it as META before CONFOUNDED can. `plants:161`
is a mutual exclusion the entry states in bold. `structural:007` **annotates
its own revival in the body** (*"since overturned: it is half the fix"*,
shipped as `LandingAux::Seed`); the screen read the clause and not the body.
Two rubric rules were under-applied in exactly the candidate direction: **rule
A** (delete the suspect number — does the entry still reject?) and **Question
1** (META beats everything). The PR's κ could not see this, because two
screeners sharing a rubric share its blind spot, and the random calibration set
held few candidates to be wrong about.

**What the two samples say together.** The DEAD side is about as tight as the
PR claimed. The candidate side is not: on this sample the table's precision is
roughly **30–45%** (6 of 20 confirmed blind; allow some of the closures to be
wrong). Over 111 rows that is **~35–50 real candidates**, not 111, and the
~40 under-called from DEAD are mostly stale-not-revivable. So the true
"idea was never fairly tested" set is likely **smaller than 111, and a
different set** — half the listed ones do not stand, and the ones the DEAD
side hides are the write-back kind. The revival report's *"tight rather than
loose … the 14% is a floor"* should be replaced by that sentence. The
practical consequence is in Part A step 3: no candidate should cost anyone an
hour before a second blind pass over the 111 with rule A and Question 1
stressed.

The sample files, keys and result TSVs are committed beside this report in
`Reports/data/dead-ends-triage-review-2026-09-12/` (`sample-A-dead40.md`,
`key-A.tsv`, `result-A.tsv`; `sample-B-mixed30.md`, `key-B.tsv`,
`result-B.tsv`). Both raters ran on the `main` checkout, which holds no triage
data, with Opus; the prompts are in the session transcript and were neutral
on the expected rate. n=20 on the candidate side is one rater's reading and
should be treated as a direction, not a decimal.

## 5. The plan

Work in this order. Parts A and B go on the PR branch
`claude/gallant-fermat-dqvtm4`; Part C is Rust and goes on a fresh branch
off `main` once #331 has landed. Every step names its check.

### Part A — make #331's tables true (a day, no Rust; step 3b is half of it)

1. **Re-key.** `stable_key` = sha1(section, address, claim[:160]). Regenerate.
   Confirm 785 rows → ≥ 783 distinct keys (the two `other:` pairs may still
   share). Re-join `screened.tsv` on the new keys; the 19 rows of §2a will
   have no verdict — screen them by reading, one sitting, rubric unchanged.
2. **Fix `--check`** per §2b. Run both positive controls and paste their
   output into the commit message. Remove the blame-derived columns from the
   committed TSV (keep them behind `--dates`, full history required) unless
   the compare excludes them; either is fine, one of them is required.
3. **Relabel** creatures:039, field:003, structural:013, plants:044 per §2c;
   restore structural:039; record demotion reasons for powders:013,
   destruction:050, plants:017, plants:098 (read each, one sentence citing
   source). Relabel the fourteen closures of §4 sample B to the blind
   rater's labels where the adjudication upheld them (`result-B.tsv` has
   the reasons), and add the four sample-A reopens: `destruction:034` as
   EXPIRED (re-test owed: run `oil_beside_a_blast_ignites_but_stone_does_not`
   and read `fire::try_ignite`'s temperature path against the three
   finite-ignition materials), `field:027` and `destruction:004` as LANDED,
   `field:008` as RE-TESTED, each after re-verifying the citation in
   `result-A.tsv`. Regenerate `candidates.tsv` from `screened.tsv` — add
   that regeneration to `deadendindex.py` so the file cannot drift again.
   **3b. Second blind pass over the remaining candidates.** One Opus
   session, all rows still labelled CONFOUNDED / COSTED / SUSPECT-INSTRUMENT
   / EXPIRED / UNWIRED after 3a, labels stripped, blended with an equal
   number of DEAD controls so the rater cannot infer the base rate, rubric
   as written with rule A and Question 1 quoted in the prompt. Adjudicate
   every disagreement by reading; keep a candidate only where the
   adjudication says so. Expect to close about half. ~300k tokens. This is
   the step that makes the table worth ranking from; without it the
   run-order is ranking a list that is half noise. Record the resulting
   precision beside §4's number in the revival report.
4. **Write back** structural:074 (condition met 2026-08-22; cite §0d and the
   rules bullet; the empirical positive control is open, instrument named)
   and structural:013 (§3). Fix the two stale `plant.rs` doc comments
   (`:11741`, `:14022`) to say the hop bound is gone and point at
   `anchor_support` — doc-only, no behaviour.
5. **Reconcile numbers** across the handoff, the revival report and both
   `Reports/README.md` lines (§2d), from the regenerated tables. Add the two
   rate measurements from §4 to the revival report's error-rate section,
   replace its *"tight rather than loose … a floor"* reading with §4's, and
   take `creatures:022` out of the CONFOUNDED showcase in the report and the
   PR body (it is a process entry the register already resolved).
6. `bash scripts/docscheck.sh`, `python3 scripts/bugindex.py --check`, push,
   let CI run. Then the owner decides on the merge (§6).

### Part B — close the loop the PR named and could not close (half a day)

The PR's strongest finding is that conditions are met in code and nobody
tells the entry. Its automation attempt asked a *static* question — does the
clause name an identifier that exists today — and measured it worse than
random, because clauses mostly name things that already existed. The
question that carries signal is **event-shaped**: *does this diff add the
thing a clause names?* Build `deadendindex.py --touching <paths…>`:

- input: the files a branch changed (`git diff --name-only origin/main...HEAD`)
  and its added lines;
- output: entries whose `target` is one of those files **and** whose
  `Re-test when:` clause names a backticked identifier that appears in the
  diff's *added* lines; plus, separately and marked as weaker, entries whose
  target file changed at all when that file has fewer than ~15 entries.

**Positive controls, from the nine write-backs and §4's reopens**: replay
the merged PRs that shipped `q_now` (plants:124 must surface),
`cross_section_axis` (plants:019 — it has no clause, so it *cannot* surface;
that is the 65-entry gap and the control that proves it), `bearing_moment`
(structural:038/040), the finite `ignition_temperature` values in
`wood.ron`/`log.ron`/`deadleaf.ron` (destruction:034 must surface — its
clause names the identifier and the diff added it), and `apply_sky_to`
(field:027). Six of the seven stale entries the two adversarial samples
found are of this shape; a `--touching` that catches four of them at replay
is doing the job. **Negative control**: replay five merged PRs that
touched nothing the register names and confirm the output is empty or
near it. **Kill criterion**: if the identifier match produces more than ~10
hits per plant-line PR with fewer than one true positive among them, stop
and record it in `dead-ends.md` beside the static attempt. If it passes,
wire it into `branchcheck.sh --brief`'s output (one line: *"N register
entries name what this branch changed — `deadendindex.py --touching`"*) so
it fires at SessionStart, and add one sentence to CLAUDE.md's *open a pull
request* paragraph. Then give the 65 clause-less entries a clause where
one can be written — that is the only way plants:019's shape becomes
findable.

### Part C — the first candidate, plants:124 (a day, Rust)

1. **Instrument first.** Add a `crown` arm (and `crown_noload`) to
   `examples/plant_severance.rs`: per plant, remove a `rows`-deep band at
   `cut_frac` of the plant's own height above ground (default 0.5), roots
   untouched. Echo `cut_frac` in the header line. Positive control:
   `unreached` non-zero at the cut on every seed; `water`/`status` within
   the control arm's spread (the roots are still there).
2. **Run structural:074's positive control on it** before changing
   anything: standing `cells` after a mid-crown cut against control, six
   seeds. Alive: the loss is the cut subtree's size within seed spread.
   Write the result back into the entry. One hour, and it validates the
   instrument for step 3.
3. **Measure the control's deficit.** Add `q_peak − q_now` at the bole to
   the harness row; run `control` over a full day cycle; record the p50/p90
   of the standing deficit. That number is the threshold's floor.
4. **Add the counter, then the mechanism.** `world.buds_flushed` beside
   `shed_*`. Then in `break_buds`, one extra term: reserves may enter the
   supportable count only in proportion to the bole deficit above the
   step-3 threshold, capped by the deficit itself (bounded; identically zero
   for a plant that never had foliage). Never on `stock`.
5. **Arms:** `control`, `crown`, `crown_noload`, six seeds, 40k frames, cut
   at 12k, fine sampling after. *Alive*: `d_cells` climbs back toward
   control after the cut with `buds_flushed > 0`; control flushes nothing
   extra. *Still dead*: flat with `buds_flushed 0`, **or** any arm's
   `above_ground_width` overshoots control — the 1,723 → 38,605 fusion. Run
   `bash scripts/seedsweep.sh` before landing, as CLAUDE.md requires for any
   model over procedural content.
6. **Post it** — `review.py` card, `crown` vs `control` filmstrip pair, the
   flush count in `meta`. The claim being made is *a felled tree has a middle
   between thriving and gone*, and that is judged by eye.
7. Rewrite `plant.rs:7335` and the entry: `CONDITION MET, RETRIED AND …`
   with the numbers either way.

### Part D — leftovers, in this order, only if a session is already there

- rendering:012: one paragraph in `README.md:1061` still says live
  screenshotting is impossible; CLAUDE.md documents it working. Fix and
  write back. Five minutes.
- other:080: `genome_reach -- drift=1` census, nothing to write, one hour.
- plants:058 as the cheap half of a crowding question, per the check file.
- structural:039's `relax=1` re-baseline, only from a structural session.

## 6. On merging #331 — the owner's decision, unchanged

The prior session asked twice and got no answer, and this review does not
answer it either. What it adds is the cost of each option, measured:

- **Merge as-is**: lands nine correct write-backs and seven fixed section
  counts, and also lands a generated file that every session's docscheck
  will rewrite with wrong dates (§2b), tables that name a withdrawn entry
  as the top candidate (§2c), and a report whose numbers disagree with its
  own tables (§2d).
- **Fix then merge**: Part A is half a day, no Rust, one branch, and it
  removes all three. The register write-backs are unchanged either way.

The recommendation is the second, and it is a recommendation.
