# Creature line — handoff for a coordinator, 2026-08-30

**From `session_01JncNrRQacprPV1tRw3BXoh`**, successor to
`creature-line-handoff-2026-08-29-evening.md`. Written for a coordinator that
will run three lanes.

## 0. How to use this file

**§§1–5 are measured. §§6–7 are my reading, and the split in §7 is a
suggestion — you decide the packages.** One thing in §7 is *not* opinion and
is labelled so: the file-contention constraint, which is counted rather than
argued and which breaks the obvious three-way split.

Read the sources and form your own view: `CLAUDE.md` (method — its *By topic*
table maps subsystem to section; do not read `README.md` whole),
`Reports/creature-evolution-plan.md` §0 for decisions **E1–E15**,
`Reports/session-programs.md` for the coordinator↔lane protocol (**read this
before dispatching anything**), `Reports/README.md` for report standing,
`Reports/dead-ends.md` grepped by *mechanism*, never by subsystem.

## 1. Where the line is

**Reproduction works and the shipped ant still cannot do it.** S6 landed
(#126) and read `births 0` for a while, which looked like a broken birth path.
It is not: the gate control now reports **births 1875, generation 18, 6
lineages, top share 0.517**. The path fires and selection has something to act
on. What blocks the shipped ant is arithmetic — it banks **567** against a
birth bar of **1,860**.

Two things stand between here and a breeding colony, and **neither is built**:

- **`birth_grant`** — decouple what a newborn is *given* from `start_energy`.
  Absent from `main`; grep confirms no such field.
- **E14, the horizon cut** — authorised by the owner 2026-08-29 (*"Yes, let
  them starve"*) and **not shipped**: `ant.ron` still reads
  `start_energy: 900.0`.

The economics report calls `birth_grant` worthless without E14. They are one
package, not two.

## 2. E15 is new, and it re-points the predation line

**Vision comes before predators** — owner, 2026-08-30. Recorded this session
in the plan's §0 and §5. The short form: E13's probe *executed and answered*,
and the answer was neither of the two things E13 expected. Numbers in §4.

**Predation as a milestone stays unauthorised.** It is not blocked on a
fear-scent channel, a `Strike` verb or slot 12; it is blocked on a sense that
reaches. Do not let a lane design the third pheromone plane — that is the
route the measurement rules out.

## 3. Decisions — authorised, and not

**Authorised:** **E15** vision before predators (new) · **E12** S6
reproduction (built, economics unfinished) · **E14** the horizon cut
(**authorised, NOT shipped**) · `birth_grant` + newborn-starts-small · **E9**
water response (decided, unbuilt — and it is the only proposed mechanism that
puts **carrion** in the world, which §2.5 measured as why the diet curve has
one hump) · **E11** gap-crossing (**shipped** — `Impulse` is a live brain
output; the previous handoff listed it as pending).

**Not authorised, and do not let a lane assume otherwise:** the predation
milestone · `birth_size` as a gene · the third pheromone plane · `Strike` ·
the nest-larder *fork* as a hand-built species (see §6).

**Blocked on the owner:** **E10's premise is false in the code.** It asserts
per-cell metabolic cost already prices a longer body; `idle_cost` and
`move_cost` are flat per organism and nothing reads `chain.len()`. Body size
would ratchet to one extreme however priced. This blocks S8 and is a decision,
not a task.

## 4. Measured — do not re-derive

- **The S6 gate.** `creature_probe start_energy=200 body_energy=20
  threshold=241 hunger=0.9 terrain=world frames=24000` → `births 1875`,
  bit-identical across runs. **Read `births` against `live`**: at `seed=7` the
  same line gives `births 0 / live 0` — an empty world, not a failed birth.
  The gate line pins no seed. (`seed=` parses decimal only.)
- **The predation pre-flight** (re-run this session on `main`). Channel B:
  mass **294** over **33 nonzero cells** in an 81,920-cell world. **77%** of
  *ants* sit within a sensor offset of a nonzero cell; **32%** of beetles do.
  Mean beetle→nearest cell **46 cells**, against a **6-cell** sensor span.
  **The two sensor reads differ 1.3% of the time**, `|along|` 0.0067 — no
  gradient anywhere a beetle stands. **The kill works**: under the saturated
  control, prey-within-sensor 0.467 → 1.000 and a hungry beetle beside an ant
  feeds, ant cells 24 → 22 → 21. **The search is what fails.**
- **No sensor reports another organism at a distance.** `FoodAdjacent` and
  `AtNest` are contact-range; the two pheromone planes are the only distal
  sense and are the ones measured failing. That is the whole of E15's case.
- **`creature_probe`'s printed ceiling is not a ceiling** — 616 banked against
  540 printed, and 561 with `mutation_rate=0`. It prices the *founder's* gut
  (`def.traits[TRAIT_GUT_BIAS]`) where `creature.rs:1583` digests with the
  organism's own heritable one; mutation off removes 55 of the 76 excess. A
  residual 21 is unexplained and left open. The readout no longer says
  "proof". **Do not quote a ceiling as a bound.**
- **The nest store is already physical.** A delivery writes the carried cell
  into the world beside the nest — `creature.rs:1812`, *"At the nest it is
  storage and always wanted"* — and the pre-flight logged 532 deliveries.
  Nothing spends from it. This matters for §6.

## 5. Where I was wrong — the pattern is worth more than the list

**All three were the same failure: I compressed a primary source I had not
read, and the compression sounded authoritative.**

| I said | truth |
|---|---|
| CI was green on #136 | It was not — my poll read an empty API response as "no failures". Green looks identical to never-reached. Verify with the check-runs API, not a bare curl |
| "No standing larder exists" | Wrong — I grepped a variable name instead of reading the code. The pile is real cells |
| "The predation decision was don't build it" | Wrong. The **milestone** was never authorised; what ran was a *pre-flight*, and it answered a question rather than closing the line |

## 6. The owner's correction that reframes reproduction

**The two "forks" must be *reachable outcomes*, not two hand-built species.**
The owner, 2026-08-30: the intention was that those behaviours "should be
possible based on our implementation. not that we should manually design two
creatures that do that."

**The economics report contradicts itself here and the owner's reading is the
one to follow.** Its §4 says *build Fork 1, run E3 in parallel*; its §5.3
argues the store's location "should be the gene rather than a choice." Take
§5.3.

What that requires: genes spanning the space — `birth_grant` (how much),
`reproduce_at` (when), `store_in_body` (where the surplus sits). **And the
trap that makes this non-trivial**: a gene whose codomain is degenerate
expresses nothing. This project has already paid for that once — plants'
`light_weight` was authored up to 0.6 while `phototropism_dir` could only
return up-or-nothing, and fixing the codomain took reproduction to zero
because every constant had been calibrated against the broken quantity. So
before `store_in_body` is a gene, **both ends must be reachable and the
constants re-derived as part of the work.** §4 says the pile end already
exists physically; what is missing is a birth that can spend from it.

## 7. A suggested split — and one constraint that is not a suggestion

**THE CONSTRAINT (counted, not argued).** Over the last 80 landings:
`src/sim/organism.rs` **29**, `src/sim/creature.rs` **25**, `src/sim/brain.rs`
**18**. **Vision and reproduction both live in all three files.** A naive
"Lane A = vision, Lane B = reproduction, Lane C = something" puts two or three
lanes in the same hot trio at once — which is the split-authored-in collision
`CLAUDE.md` records, not bad luck. **Run at most one lane in
`creature.rs`/`organism.rs`/`brain.rs` at a time.** If you overrule this,
overrule it deliberately and stagger the landings.

Given that, what I would run — **your call, not mine**:

- **Lane A — vision (the one source lane).** Owns `src/sim/{creature,
  organism,brain}.rs` and `assets/species/*.ron`. E15. Line-of-sight at CA
  resolution, **not** a two-point gradient read on a coarse field — that is
  the degeneracy hit four times on three lines and never once caught by a
  test.
- **Lane B — size the sense before it is built.** Owns `examples/` (name the
  files: `predation_probe.rs`, a new `vision_probe.rs`). **This package is
  answerable today with no vision implemented**: what fraction of beetles have
  an ant in unobstructed line of sight at radius 8/16/32/64, and how does that
  move with occlusion? It is pure geometry, and it *parameterises Lane A's
  design* — the 46-cell mean distance already says a short radius will not do.
  This is the "build the readout before the mechanism" rule.
- **Lane C — reproduction economics, no source.** Owns `Reports/**` and a
  named new `examples/larder_probe.rs`. Two deliverables: **land the economics
  report** — it is on `claude/creature-reproduction-economics` with **no PR
  and not on `main`**, so the document the whole next step depends on is
  reachable only by knowing the branch name — and census the nest pile (does
  it persist, how large, is it eaten) to say whether `store_in_body` has two
  reachable ends.

Then `birth_grant` + E14 as one package for whichever lane frees up, scoped as
a shared-budget reallocation: `idle_cost`, `move_cost`, `eat_energy`, the
synapse tax and `reproduce_threshold` all change meaning and must be
re-derived, never inherited.

**What I would not do:** let two lanes into the hot trio; let a lane build the
third pheromone plane; tune `reproduce_threshold` down to "fix" the shipped
ant (`reproduce_at` floors it at `birth_cost + 1`, so the edit does nothing
and reads exactly like the change having been made); or treat §7 as settled.

## 8. Coordinator mechanics you will need

Full protocol in `Reports/session-programs.md`; the four that cost the most:

- **`SendMessage` does not resolve cloud sessions.** Reach a lane with
  `create_trigger(persistent_session_id=…)` + `fire_trigger`, poke-only (omit
  both `cron_expression` and `run_once_at`).
- **A woken lane cannot reply** — a trigger stamps `allowed_tools` carrying no
  `mcp__*`. Its whole outbound vocabulary is commits and files. **Insist the
  return path is files**: "commit it, push, report the head SHA."
- **The PR list is not the work list.** A lane that cannot reach the GitHub
  API cannot open a PR; you own the merge either way. Read
  `branchcheck.sh --prs` every cycle.
- **Spawn workers on `model: "claude-opus-5"` explicitly** — owner cost
  policy; the default inherits the coordinator's tier and three workers once
  ran $25–71 each in ninety minutes. And **put a cost fork in every brief**
  (build it, or write the finding up and stop — not a half-built fix with no
  writeup).

Environment notes that cost the previous session time — clippy drift, the
review queue's author filter, the measurement-instrument box — are in
`creature-line-handoff-2026-08-29-evening.md` §7 and still hold.
