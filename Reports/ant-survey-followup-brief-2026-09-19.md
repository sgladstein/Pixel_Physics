# Ant-survey follow-up round — brief for three lanes, 2026-09-19

*Written by the coordinator, session `session_01HTNLNphUPgpg5GqCwCQvmW`,
after the owner read
[`ant-sim-research-review-2026-09-19.md`](ant-sim-research-review-2026-09-19.md)
(PR #475, branch `claude/ant-sim-research-review-eyuol2`). Three lanes,
each its own cloud session. **Read this file, the review's §0, §3, §4 and §8,
and the one dead-end entry named for each candidate — never
`dead-ends.md` whole.***

The owner's instruction, verbatim, because it sets the posture of two of the
three lanes:

> "spin up another agent or two to re-evaluate anything else from the report
> that seems like a good idea and is supported by research but was rejected
> as a dead end or just declined. **Don't trust past results.** Past agents may
> have made mistakes and these are all very complex systems so it may have
> rejected it based on a bad test or a failure coming from an interconnected
> system."

And for the performance lane: *"I will test the parallelism on my own
machine"* — so the creature parallel switch is **not** yours.

## What binds on every lane

- **Get the review off the branch**, since PR #475 may not have merged when
  you start:
  `git fetch origin claude/ant-sim-research-review-eyuol2 && git show origin/claude/ant-sim-research-review-eyuol2:Reports/ant-sim-research-review-2026-09-19.md`
  (and `…-external-…` for the survey itself).
- **Your coordinator is `session_01HTNLNphUPgpg5GqCwCQvmW`, and the human is
  not the postbox.** You cannot message anyone. **The return path is files**:
  commit, push, open a PR (`CLAUDE.md` is the owner's standing authorisation),
  and put the head SHA and the PR number in your lane note under
  `Reports/lanes/<your-lane>.md`. Write only your own note.
- **`CLAUDE.md`'s method, the parts that have cost this line most**: rebuild
  the examples in the same command as the run (a stale binary cost a day on
  the trail line); pin `RAYON_NUM_THREADS`; twelve seeds, not six; compare
  two arms inside one run; the positive control before the treatment; read
  the quantity, not the exit code; a tidy first result is an artifact until
  shown otherwise. `examples/trailfollow` echoes `refill=` and `stop=` — read
  them off the header before comparing to any archived log.
- **Before writing into `src/sim/creature.rs`, `world.rs` or `ant.ron`, run
  `bash scripts/branchcheck.sh --who-touched <path>`** and read the head it
  prints. Those three files took dozens of landings this week. Quote a SHA in
  your note, not your memory.
- **A rejection you overturn is a write-back, not a deletion.** Add to the
  `dead-ends.md` entry: the condition its rejection depended on, what the
  re-test changed, and the new number with its seeds. `python3
  scripts/deadendindex.py --touching` before the PR; `python3
  scripts/bugindex.py --branches` before filing any bug letter.
- **Anything visual goes on a review card** (`.claude/skills/review/SKILL.md`),
  with the discrete event count in `meta`. The queue is for visual judgements
  only; numbers go in the report.
- **Stop a candidate when its falsifier fires.** The budget is one honest
  sweep per candidate, not a search for the setting that works.

## Two unlanded branches you must know about

| branch | ahead | PR | what it holds |
|---|---|---|---|
| `claude/upbeat-shannon-cez0w4` | 20 | **none** | the trail-lifetime split: channel A at `rho 0`, channel B keeping 0.03; `src/sim/pheromone.rs` and `creature.rs` changed; round trips up >10x at gap 90 over 36 seeds. **The largest recent change to the planes, and invisible to the PR list** |
| `claude/nest-biology-research` | 5 | #472 | docs only: the digging-signals report and a build plan, both partly superseded by `nest-digging-plan-2026-09-19.md`, which is on `main` |

`claude/sweet-tesla-ommknn` (the `digbox` instrument, the Stage 0 census
fix, two default-off switches) **is merged**; `examples/digbox.rs` is on
`main`.

---

## Lane P — review and build the performance proposals (`claude-opus-5`)

**Subject:** the review's §8, written after the owner said the field is a
large part of the cost and creatures and the field are interlinked. He was
right at the one clean comparison a harness here could make: fifty-two ants
doubled the field's cost while their own decisions cost less than the
increase. **Your job is to check §8's mechanism in code, build the two
cheap things it proposes behind switches, and score them with the
falsifiers it names.** Not the parallel switch, and not the GPU field.

Read first: review §8; `evolution-lab-playtest-2026-09-13.md` §1 (his log:
awake chunks 12 → 52 at 39 → 2,473 ants, and no per-phase split);
`evolution-lab-creature-cost-2026-09-13.md` §2 and §5;
`evolution-lab-frame-cost-2026-09-01.md` §2; `CLAUDE.md`'s measurement
rules (*a cost that vanishes may be work that vanished*; *measure a cost
against the state the optimisation exists for*).

1. **Verify §8.2 in code.** Does a chunk dirtied *only* by creature steps
   keep its field tile solving? Name the lines (`World::set` →
   `Chunk::mark_dirty`, `field::step`'s skip condition, the per-tile
   `settled` rule at `field.rs:430-445`, `rebuild_blocked` at `:821`). Is
   "a creature cell neither blocks nor sources anything the field solves"
   true for **every** channel — light (does a body shade?), glow, heat,
   moisture, pressure? If any channel reads a creature cell, say which and
   the proposal narrows to the others.
2. **A per-phase stopwatch in the live app, behind a switch.** Off by
   default, provably free when off (bit-identical, no measurable frame
   cost). `#374` refused stopwatches in the live loop; a gated one is the
   answer to that objection, and the owner will run it on his own bed,
   which no harness here can build (stocking tops out at 224 ants). Write
   the phase times where his session log already goes, so `awake chunks`
   and the phase split land in the same row.
3. **Creature-only chunk activity does not wake the field**, behind a
   switch, default off. **Falsifiers, both required**: (a) the field hash
   under a walking colony must not change with the switch on — if it does,
   a channel reads creature cells and item 1 missed it; (b) on `antcost
   ants=0,300,600,1000 width=1024 colony_species=ant par=off rounds=60
   frames=200`, awake chunks at ~200 ants (27.9–31.1 with the switch off,
   21.4 with no ants) fall back toward the no-ant figure, and the per-ant
   slope falls. Also `lab_cost colonies=0 frames=1500 every=500 phases=1`
   against `colonies=1`: the field row (0.090 → 0.188 ms with 52 ants) is
   the number to move. Watch for the cost relocating rather than vanishing
   — the sweep may pick up what the field puts down.
4. **The GPU field — a design note only, do not build.** What it would need
   (readback every tick since the ants read on the CPU; same-build
   determinism across drivers; the sleeping-tile economy), and the number
   from item 2 above which the survey's recommendation would become worth
   taking. `PLAN.md`'s decision table row *Simulation device* is the
   standing decision; say what would change it.

Owns: `src/sim/field.rs` (the wake path only), `src/sim/world.rs`
(`mark_dirty`/settled only), `src/lab/*` for the stopwatch, `examples/antcost.rs`,
`examples/lab_cost.rs`. Does **not** touch creature behaviour or `ant.ron`.
Deliverable: a short report with the commands and the two tables, the
switches, the lane note, a PR. Model: Opus — a wrong number here is loud,
because both falsifiers are counters.

---

## Lane T — re-evaluate the trail line's rejections (`claude-fable-5-1`)

**Subject:** the survey's core recommendation is mass recruitment on a
scent trail with negative feedback, and this engine has rejected or
declined four mechanisms in exactly that space. The owner's instruction is
to distrust those rejections. **For each candidate: name what the original
test could not see, re-run it under the entry's own re-test conditions with
a positive control and twelve seeds, and write the result back.**

Read first: review §0, §2.2, §2.3, §3 items 1–3, §4 items 1, 3, 4;
`pheromone-master-2026-09-17.md` §1, §3.2, §5, §5b, §8 (the traps);
`pheromone-lifetime-and-wiring-2026-09-14.md` (why decay is inert on a
one-cell line); `wiki/ants.md` "Coming home" and "They leave smells behind".
**Every pre-2026-09-15 measurement is on `u8` planes**, where the far half
of every trail read exactly zero. Treat any rejection dated before that as
untested on the current engine.

Candidates, ranked by how likely the rejection was the test's fault:

1. **The food-charged odometer on `EmitB`** — `dead-ends.md:1925`, rejected
   2026-09-17. The bed had **no working return leg**: no laden ant got home
   until `849d6d06`/`de27be4d` on the 18th. A trail strongest at the food
   and fading toward the nest (Beckers 1992; Czaczkes 2024's 22x; the
   survey's *Trigona* row) is readable only by an ant walking *out* from a
   nest it can get back to. The `trailfollow` riders are archived. Score on
   ants reaching food, never deliveries; the `mute` arm is the falsifier.
2. **Reading the food trail at full gain** — §Z7's units-2/3 re-gate,
   `dead-ends.md:1860/1869`, *"correct, and it makes the animal decisively
   worse"*, 25% against 15% in a mirrored race. The entry itself scopes the
   rejection to *what a food trail is worth in this bed* — a uniform larder
   — and names the re-tests nobody ran: a **patchy** larder (`labforage
   plant=` / `creature_arena plant=`), and a **faster channel-B decay**
   (`labforage bdecay=`, `Pheromones::set_channel_rho`). Add the survey's
   two negative-feedback terms as arms: `(Crowding, EmitB, −w)` (Czaczkes
   2013, one wire) and the **no-entry mark** as a negative deposit on B
   (Robinson 2005; needs an emitter condition — a hidden unit). Derive `w`
   from the `Crowding` band actually occupied; the nest plan measured its
   gate compressing 26x.
3. **`DECAY_RHO` in the literature band** — `dead-ends.md:1205`, *"kills
   trails before ants can use them"*, retuned to 0.03. Measured on `u8`
   planes at `PHEROMONE_INTERVAL 4` with the strict-decrease LUT. The
   lifetime branch since found `rho 0` right for channel A and 0.03 right
   for B. Re-evaluate the band **for B only**, on `u16`, with `pherolife
   sweep=rho` in round-trip units, and against candidate 2's patchy larder
   where a faster fade is the thing the survey says stops a trail outliving
   its patch.
4. **Trophallaxis measured as harmful** — `wiki/ants.md` "Feeding each
   other": fewer survivors on two runs of three, in *the harness's colony*
   dropped on seedlings, and the page itself says the runs are too few to
   sign. The survey's Sendova-Franks *famine relief* is the opposite claim.
   Re-measure on the played bed at session length, order statistic over
   twelve seeds, with the founders' reserve spread as the second readout.
5. **Pass-through kin** — `passes_through_kin`, cut blocked moves 90% and
   *reduced* intake 35%. The survey's rule is right-of-way **to the laden**
   (Dussutour 2009), not to everyone. One predicate on `try_swap_with_kin`:
   a laden ant passes an unladen nestmate, never the reverse. Cheap; last.

**The branch question is yours to decide, not to ignore.**
`claude/upbeat-shannon-cez0w4` (20 ahead, no PR) carries the lifetime split
that changed what channel A is. Candidates 1–3 measured on `main` alone are
measured on a homing plane that dies in 144 frames. Either branch from it,
or run each candidate on both, and say which in the note.

Owns: `assets/species/ant.ron`, `src/sim/pheromone.rs`, `examples/trailfollow.rs`,
`examples/pherolife.rs`, `examples/labforage.rs`, and in `src/sim/creature.rs`
**the emitter and reader sites only** (`EmitB`, the `PheroAAlong`/`PheroBAlong`
senses, `tumble`/`home_weighted_pick`). Lane N owns the dig and drop verbs in
the same file; read `Reports/lanes/ant-survey-nest.md` before touching
anything near them. Model: Fable — a wrong re-evaluation here re-enters the
record as fact and is the premise of the next three lanes; the owner has
said the past results are not to be trusted, which is a call for the
reviewer that finds errors in its own brief.

---

## Lane N — re-evaluate the nest line's rejections (`claude-opus-5`)

**Subject:** the survey's §7 (Toffin, Buhl, Bardunias & Su, Khuong,
Tschinkel) against a line that has rejected density-dependent digging,
curvature-attracted deposition and two spoil-placement rules, and has just
found that **every census it scored those on undercounted the nest three
times** (`nest-digging-plan-2026-09-19.md` §1, Stage 0 — now fixed on
`main`). Re-score first; re-evaluate second.

Read first: review §2.6, §3 item 6, §4 item 8; `nest-digging-plan-2026-09-19.md`
whole (it is short, and its status note reorders its own stages);
`nest-shape-three-negatives-2026-09-19.md`; `nest-biology-2026-09-19.md` §0
and §10 (the owner's ruling that the nest has no purpose yet — it bounds
what "better" can mean); `lanes/nest-digging-handoff-2026-09-19.md` (the
`roofed` trap and the `trace` trap). PR #472's two docs are partly
superseded; read them for the seven digging-signal answers only.

Candidates:

1. **`(Crowding, Dig, 0.6)` and the two later `Crowding` interventions** —
   `dead-ends.md:1788` and the plan's §0: *"a correct mechanism with nowhere
   to act"*. Two things changed since the null: the local reading exists
   (`PIXEL_PHYSICS_CROWDING_LOCAL=near|wide`, default off) and the census now
   counts bodies. **Re-score all three on `room total`** (roofed + open +
   bodies), on `digbox`, before believing any of them was a null. Then the
   plan's Stage 2: give density an aggregation point (a nest site with real
   depth) and check widening is *local* to it.
2. **Deposition attracted to existing pellets** (Khuong 2016 — the survey's
   headline for construction). The engine's proxy is
   `(SurfaceCurvature, DropSpoil, 0.169)`, measured *a tenth steeper* — and
   note **how** that was measured: against where the same laden ants stood
   and chose not to drop, after an earlier threefold figure was found to be
   measuring that a surface is uneven. Build the direct rule — a `DropSpoil`
   preference for cells adjacent to `spoil` — as a wired instinct on its own
   output (`dead-ends.md:1095` says why not a coefficient in `act`), and
   **judge the mound by eye on a review card**, with the pellet count in
   `meta`. The owner has reported the anthill three times; he is the
   instrument here.
3. **The two spoil-placement rejections** — `dead-ends.md:1087` (first empty
   neighbour refills the tunnel), `:1093` (loose spoil closes the gallery),
   `:1095` (gating on `LightHere` failed *on the sensor*, and the entry says
   to retry with a real in-the-open reading). The census now has one:
   cover overhead / roofed void. Retry `:1095` with it.
4. **The downward dig bias, "refuted from the code"** —
   `nest-shape-three-negatives`: the dig target and the step target are the
   same cell, so an override severs the coupling. Was that a test or an
   argument? The survey's gravity-and-repose result (PNAS 2021, validated:
   *ants dig piecewise linearly downward*) and the load-based rule (*dig the
   grain carrying least*, free in this engine via `load.rs`) both give a
   direction. `home_weighted_pick` shows the pattern that works on flat
   ground: bias the **heading at the re-roll**, not the target. Try the
   direction there.
5. **The dig-face pheromone** — rejected on Bruce 2015, n = 5 groups, scoped
   by its own authors to the face. Not a build; a paragraph on whether the
   rejection is broader than its evidence, since Khuong's marker is on the
   *pellet*, not the face.

Owns: `examples/digbox.rs`, `examples/burrow_probe.rs`, the lab census, and
in `src/sim/creature.rs` **the dig and drop verbs and `adjacent_nest` only**;
`assets/species/ant.ron` for the dig and drop wires — coordinate with lane
T through your notes before any commit that touches a line the other lane
owns. Score everything on `room total`; `digbox`'s `trace` spread follows
the reach dial by construction and is not a shaft. Model: Opus — the
candidates come with counters and a plan of record, and a wrong answer
shows in the census.

---

## Closing

The coordinator lands each lane's PR on green, runs `docscheck` after each
merge, and writes the next brief. **A lane that finishes with no PR is
invisible**; push and open one even if the answer is "the rejection stands",
because that write-back is the deliverable the owner asked for.
