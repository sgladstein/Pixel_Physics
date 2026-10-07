# How we test — the lab playbook

*Living playbook, edited in place (started 2026-10-05). For anyone measuring
the lab colony: ants, the nest, brood, foraging. It collects the methods the
owner's project settled on between 2026-10-03 and 2026-10-05, which until now
lived only in project memory and individual threads. It **links** to the
tools and traps already written up elsewhere rather than repeating them;
[`instruments.md`](instruments.md) stays the authority on what each binary
does, and `.claude/rules/measuring-the-world.md` on world-wide metric traps.*

**The short version.** Freeze mutation, land the evolved founder in the dry
goal box, run four seeds side by side, read 100k–300k means. Judge a fix by
the number it was built to move, look at a picture, then trace what it cost
ant by ant. Colony size is never the verdict on its own. Deep-trace every
test until the result is understood, and propose no new fix before then.

## 1. The standard setup

| What | Why |
|---|---|
| **Mutation off.** Every measuring example calls `creature::mutation_off_for_measuring()` first in `main()`; `PIXEL_PHYSICS_MUTATION=on` turns it back on | Mutation roughly doubles the spread between seeds, and once two arms differ by one frame each one's newborns draw different mutations. A colony then dies in one arm because a mutation (the pile-loafer, slot 4126) took over there, not because of the rule under test. Owner, 2026-10-04: *default to off for any testing*. A **new** measuring tool must add the call; one that does not is measuring evolution |
| **The dry goal box** — `assets/lab_scenarios/nest_goal.ron` | The owner's goal bed: no plants, mister off, one endless heap of player food 30 columns east of the nest. Nothing but the colony moves the ground, so a nest picture is the colony's own work |
| **The evolved founder** — `deeptrace founder=evolved` | All deep traces and goal-box A/Bs since 2026-10-04 use it (it holds 11 of 12 seeds to 300k). The game-wide switch to land it in every box is in flight with the Laying lane |
| **`RAYON_NUM_THREADS=1`, four runs at once** | A lab run is no faster on four threads than one, so four single-thread runs is ~4x throughput. Pinned threads also keep counters comparable |
| **Seeds are deterministic per build** | Same build + same seed = same world. So arms compare **paired, seed by seed**, and the baseline is re-run on the same commit, never quoted from memory |
| **Read 100k–300k means**, not one frame | The colony's first 100k is founding. Adjacent census stops of one run can differ by an order of magnitude |
| **Stamp the `main` commit** on every number | Lab numbers do not compare across certain merges: before PR 611 (mutation switch), before `6e42f0fa` (HUNGRY_OUT on), before `06202f793` (WATER_FOOTING on, misted beds). A number without its commit cannot be checked against one with |

**How many seeds.** 1–4 seeds to iterate, labelled as partial and reported
early; **12 seeds only to settle a default**. Before a winner ships, run it
once with mutation **on** (2 seeds to 150k is the usual check) — a fix that
only holds in a frozen colony is not finished.

## 2. Judging a fix

**Trace every test until the problem is understood, before the next fix**
(owner, 2026-10-06): *"You run your test and you deep trace it to understand
the problem in full. If you don't fully understand what's going on, stop
suggesting new fixes and implementing and testing them."* So every test run
is followed by a deep trace (§3) of what happened in it. Until that trace
explains the result — what each ant did and why, traced rather than inferred
— the next step is more tracing, not a new switch, a new arm or a re-tune.
A result you cannot explain is reported as unexplained, with what the trace
has ruled out so far.

Owner rulings, 2026-10-05:

1. **Direct metric first.** Every switch is built to change one behaviour. Measure
   *that* — soil set back down inside the nest, hungry nest steps with no
   pull, how far a digger's next cut is from its last — against the shipped
   game, per seed. This is the pass/fail.
2. **Then a picture.** Render the nest (`nestdoor shots=`, `deeptrace`'s
   `nest_f*.txt`) and look. A metric that moved while the nest looks the same
   is a question, not a win.
3. **Then trace the cost.** If the colony got smaller or died, find out **who
   died, of what, and where** — never revert on the colony number alone.
4. **The default.** A fix that works on its own metric ships **ON**, even if
   the colony is smaller; the cost goes to tracing. The one hold: a fix that
   kills colonies outright stays built but **OFF** until the deaths are
   understood. Features default on unless there is a measured harm (owner,
   2026-09-27).

Worked example: the nest-plan switches (SOIL_WAY, WAY_GAPS, FACE_TRIP,
CROP_DOWN), judged one by one on their own targets with
`scripts/deeptrace_plan.py`. Three worked, CROP_DOWN failed its own target and
was retired; the three together cost 4–12% of the colony, traced to fewer
eggs, not deaths.

## 3. The deep trace

**For "why does the colony do that"** — the method behind every finding of
2026-10-04/05. It is the `funnel` skill (`.claude/skills/funnel/SKILL.md`)
taken to the whole colony.

1. **Write down how it *should* work first**, from the biology, before
   looking at data — a table of rules, the evidence for each, and what our
   ants do today. Then say what measurement would show each rule working.
2. **Record everything.** `examples/deeptrace.rs` records every decision of
   every ant in the goal box. The cheap whole-colony form is
   `ants=0 dig=1 founder=evolved` (add `walk=1 census=1` for walking
   decisions and every death). A full `ants=all` 300k run is ~45 min and
   ~3 GB: keep it in scratch. Recording changes nothing in the world.
3. **Coarse, then deep.** Colony windows and budgets first
   (`scripts/deeptrace_world.py`, `scripts/deeptrace.py`), then follow
   single ants through the decisions that matter
   (`scripts/deeptrace_dig.py dig|soil|rooms|brood|journeys|face|fed`,
   `scripts/deeptrace_plan.py` for switch verdicts; `only=id,...` replays
   chosen ants). [`instruments.md`](instruments.md) §`deeptrace` says what
   each reader answers.
4. **Baseline arms.** The shipped game on the same seeds and build beside
   every arm.
5. **Per-seed numbers**, never only a pooled one, with a result page.
6. **Mark each claim traced or inferred.** "Eggs track brood falls (r 0.81);
   causal link inferred, not intervened" is the shape.

## 4. Numbers that have misled us

Check this list before quoting a number. World-wide traps (liquids, cascades,
frame cost) are in `.claude/rules/measuring-the-world.md`; method traps
(paired runs, `n` across arms, tidy results) are in `CLAUDE.md` §Method.

- **Plain `deliveries`** counts every put-down at home, including crumbs lifted
  inside the nest and set straight back (4–12x the real figure). Use
  **`trip_deliveries`** (`labforage trip_deliveries=`, census `tripD`) or pile
  refills.
- **Raw larvae starved** falls whenever eggs fall. Use **larvae starved per egg**.
- **Total cuts.** 80–85% of nest cuts are re-digs of soil already moved. Count
  **new-ground cuts**.
- **A share of ant-time deep in the nest, read as a gain.** 0.5% against
  0.6–1.6% of ant-time reads as "raised on all four seeds"; it is about 3
  against 4–9 ants of ~550, still basically nobody (owner, 2026-10-06). Give
  it as ants, beside the baseline: the scorecard's **ANTS DEEP** line, door
  column apart from the rest, because most deep ants sit on the brood pile in
  the shaft under the door. Then read **ANTS LIVING DEEP** under it: a jam
  shows as deep ants too. Homing seed 2's 12 deep ants were a crowd of hungry
  ants dipping below row 10 for a few frames at a time, not ants living
  there; the line counts the fed ones, and `--stays` on a `dig=1` run counts
  the fed ones in stays of 50+ frames and gives the median stay.
- **An ant standing on brood hides it in every picture.** The brood cell is lifted
  out from under the walker, so the grid, the maps (`a`) and the lab's own drawing
  all show the ant. On the shipped game 63-99% of the ants in each row of the
  column over and under the door stand on brood (2026-10-06), so a solid column
  of ants reads as ants that will not walk into it. Read `walk=1`'s `nb` column
  (`B`) or `broodstep.py`, not the picture.
- **`starvewhere.py` "encased" before 2026-10-07** counted depth, not soil.
  Its flood stopped at row 175, so every starver deeper than 15 rows read
  encased whatever was round it: 50 of 51 on LAY_BAR=body seed 1 with the
  smell store, 0 after the fix (all 50 "door open"). Re-run the tool before
  quoting an older "encased". And "door open" says a path joins the ant to
  the door, not that it can climb it: a body with nothing beside it falls.
- **`digbox` `SCORE`** — read its `n=` first.
- **Room censuses** on a wet floor (water splits one room into "chambers"),
  or split by a brood pile; brood pockets under 30 cells are not rooms;
  tunnels in the spoil mound are not the nest. A nest is roofed void.
- **"Home"** has meant different cells in different builds; do not compare it
  across them.
- **A 12-seed end count** on the played bed can mislead either way (seeds 1–12
  and 13–24 once pointed opposite ways). Use means over a window, paired.
- **A single-seed result** for a rule that helps some seeds and kills others
  is usually mutation, not the rule — see §1.
- **labforage `aloft`** is food held off the ground, not ants.

## 5. Tools, by question

| Question | Tool |
|---|---|
| Why does every ant do what it does? | `deeptrace` + its readers (§3) |
| Does a switch hit its own target? | `deeptrace walk=1 census=1` + `scripts/deeptrace_plan.py` |
| Is my arm's off run the shipped game, and how does the arm compare with the baseline? | `scripts/deeptrace_tools/identity.py` first, then `scripts/deeptrace_tools/scorecard.py` (one column per run). Read its ANTS DEEP line (about N of M ants, door column / off it) and ANTS LIVING DEEP under it, not the headline alone: most of the time in the dug nest is the knot at the door |
| Who starved where, and were they shut in? | `scripts/deeptrace_tools/starvewhere.py`, `spells.py` and `doorseal.py`, on runs made with `hungry=1 mapevery=1000` |
| Where did the digging go, and how many chambers? | `scripts/deeptrace_tools/digwhere.py` and `chambers.py` (the owner's chamber rule) |
| Did a dropped pellet have a choice of cell? | `deeptrace drops=1` + `scripts/deeptrace_tools/dropchoice.py` |
| Is the nest full, and which part? | `scripts/deeptrace_tools/bandfill.py`: free cells, ants and brood per depth band, map by map, beside the ants there (`--show F1-F2` for each map) |
| Is there a clear way into the nest through the mound? | `scripts/deeptrace_tools/moundway.py`: the shortest way in from outside, its narrowest neck, how full of ants it is, the mound's dead ends, and maps with no way in at all (`--png` draws it with the ants hidden) |
| Do ants step into the brood, or does something refuse them? | `deeptrace walk=1 digfrom=F` + `scripts/deeptrace_tools/broodstep.py` (steps offered against taken, brood against open ground by direction, beside what the chooser's scores predict; what was refused and why) and `fedboundary.py` (fed and hungry ants by what is straight below them, row band by row band) |
| Does a dig rule's input reach the ants that dig? | `deeptrace dig=1` + `scripts/deeptrace_tools/digtrace.py`: each dig decision by where the ant stood and by the Crowding it read |
| Digging in a simple box | `digbox` |
| The nest door and its pictures | `nestdoor` (`shots=` renders with the game's renderer) |
| Foraging and trails | `labforage`, `trailprofile`, `trailfollow` |
| What did a *played* session's nest do: rooms, brood, who starved where? | the lab's chronicle: the nest block at the end of `census.csv` and the indented lines under each CENSUS row (`lab::nestcensus`; README *Chronicle status*). `chronicle scenario=nest_goal export=DIR` writes the same files headlessly |
| Did a fix that works on a simple bed also hold in the played game? | `scripts/labbench.py BASE NEW` (two worktrees, pinned binaries, interleaved seeds, `labpair.py` table) — the **last** check, not a diagnosis |

Before building anything new, grep [`instruments.md`](instruments.md).

## 6. Showing results

- **One chart page per comparison**, published as an Artifact. The
  templates and how to publish pictures with them are in the project's shared
  folder, `charts/README.md` (arm tabs over per-seed panels; the nest variant
  puts a nest picture per seed at the slider's frame). Open space pale, ants
  dark, water blue.
- Lead with the finding in world terms; put the commit and instrument in the
  footnote. Per-seed numbers, not only means.
- Judge-by-eye questions go to the owner as a card (`review` skill).

## 7. Before you push

- `rustfmt` the files you touched (never a whole-project `cargo fmt` riding
  along), then
  `cargo clippy --all-targets --release --locked -- -D warnings`.
- The touched module's tests only (`cargo test --release --lib <module>`, plus
  the `tests/*.rs` file for any registry you added to). **Do not run the full
  suite locally**: CI runs it on every push (the owner caught three lanes
  running it locally, 2026-10-05).
- Merge `main` in right before hand-off; `bash scripts/docscheck.sh` after
  every merge.
- `bash scripts/branchcheck.sh --who-touched <path>` before writing into a
  file another lane owns.
- Change an ant mechanism → update [`how-the-ant-works.md`](how-the-ant-works.md)
  in the same commit. A switch that is tried and left off → a
  [`dead-ends.md`](dead-ends.md) entry with the condition it depends on.
- Stage explicit paths; never `git add -A`.
