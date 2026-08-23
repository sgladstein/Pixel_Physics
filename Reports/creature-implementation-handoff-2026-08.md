# Creature line: implementation handoff for the next agent

**Status: execution plan, 2026-08-23.** Written to be executed cold by an
implementation session that has read nothing else, in the style of the
repo's other handoffs. It operationalises `Reports/creature-review-2026-08.md`
(the *why* of this ordering) into work packages with file anchors, steps,
measurements and landing criteria. The design spec for the evolution stages
stays `Reports/creature-evolution-plan.md`; this document does not restate
designs it can point at.

**Scope guard — three things you must NOT start, whatever momentum
suggests:** S6 reproduction before the owner answers the E5 card (WP-1);
S7's larder before the owner picks surface-vs-canopy (same card set); and
any new pheromone channel, `Cell` widening, crossover or topology work —
each has a standing condition in `creature-evolution-plan.md` §6 and none
has changed.

---

## 0. How to work here (non-negotiable, all previously paid for)

1. Read `CLAUDE.md` in full before anything. It is the method; every rule
   in it cost real hours. The rules that will bite *this* work first:
   worktree per session, never `git add -A`, stage explicit paths, rebuild
   before measuring (`cargo build --release --examples` — a stale example
   binary prints plausible numbers with a fresh mtime), assets are
   `include_str!`ed (editing a `.ron` does nothing until rebuild), test
   both drivers (`update::step` and `parallel::step`), paired comparisons
   across seeds (outcome sd here is ~0.1 — one run is a sample), re-measure
   baselines in the same session on the same machine.
2. Required reading, in order, before the WP you pick up:
   `Reports/creature-evolution-plan.md` §0 (decisions E1–E7), §4 (standing
   guards), §6 (dead-end register — the most valuable page in the repo for
   this work); `Reports/creature-direction.md` §13 (what actually happened;
   it overrides §§1–12 where they disagree); `Reports/dead-ends.md` lines
   ~755–861 (the creatures section); `Reports/open-bugs-handoff.md` §A, §H,
   §Z, §F; `.claude/skills/review/SKILL.md` (posting to the owner's queue).
3. **Open a PR for your branch.** `CLAUDE.md`'s "Working alongside another
   session" is the owner's standing authorisation; do not wait to be asked.
   Land contested files fast (`src/app.rs`, `README.md`, `PLAN.md`,
   `CLAUDE.md` are the recorded collision set; `src/sim/creature.rs` is hot
   this month — WP-5, WP-8 and WP-9 all touch it, so do them serially or in
   separate quick-landing PRs).
4. Every visible change gets posted to the review queue
   (`python3 scripts/review.py …`, protocol in the skill), fire-and-forget,
   with the discrete event count in the card's `meta`. Collect verdicts
   with `review.py inbox` at session start.
5. Commit messages carry the measurement: the number before, after, and
   what was rejected on the way.
6. Gates before landing: `cargo test` (both release and debug profiles are
   CI jobs now), `cargo clippy --all-targets -- -D warnings`,
   `cargo run --release --example ascii`, `bash scripts/acceptance.sh`,
   `bash scripts/docscheck.sh`, and `bash scripts/branchcheck.sh` when you
   pick the branch up (know how far behind you are before trusting any
   number you measure).

## 0b. State snapshot (verified 2026-08-23, post-merge `main`)

- S1–S4 of the evolution plan are merged (`7b97c13` + `da252dc`,
  `5a9e594`). Genome: `GENOME_LEN = 584`, reserved dims
  `INPUT_SLOTS 24 / HIDDEN_SLOTS 8 / OUTPUT_SLOTS 12`, live
  `16 / 4 / 10`, 268 live slots, FNV manifest pinned at `2_369_832_241`
  (`src/sim/brain.rs:35-215`). No mutation operator exists; `brain.rs:151`
  names one prospectively — `live_slots()` is written to be its iterator.
- Food worth: `creature::food_value` (`creature.rs:~1690`) — per-cell `aux`
  stamp when `Material::worth_in_aux && aux != 0`, else
  `Material::food_energy`. `food_class` is authored on six materials as an
  f32 axis position (litter −1.0 … corpse +1.0) and **read by nothing**
  until S5. Diet is still `CreatureDef::food` name-strings, resolved per
  tick (~32 string hashes; S5 deletes this and banks the saving).
- `EnergyLedger` (`world.rs:209-270`): two identities,
  `expected_live_total()` exact, `max_standing_meat()` an upper bound only.
  Known slack: the `meat_lost` seam (nothing books a destroyed corpse,
  `world.rs:202-207`) and the living-flesh stamp seam
  (`creature.rs:~1718` — `ant.ron`'s `food_energy == body_energy` equality
  is load-bearing).
- Every creature is a byte-clone of its species genome; spawn paths are
  `Y`/`found_colony` and harnesses only. Worm has no `creature:` block and
  runs `worm_tick` (the only burrower). Beetle is data-only and has never
  caught anything (plan §5).
- Colony behaviour: post-merge foraging profile
  `[3858, 475, 185, 98, 1, 0, 0, 0]`, 98 trips, deepest 18 cells
  (`README.md` M18 S1–S4 status). The 55-ant probe arm reaches 12 cells vs
  a lone ant's 42 with ~60% of moves blocked
  (`foraging-range-measurement.md` §3 — in-flight, WP-2 lands it). "The
  floor feeds the colony and the colony stops ranging": moves −31%, digs
  −46%, deliveries +17% (README known limitation #1).
- CI (`.github/workflows/ci.yml`, reworked same day): one job per gate,
  known-red quarantine. Bug A is `--skip`ped in `test`/`test-debug` and
  runs in `known-red-roots` (`continue-on-error`); `ascii` is non-gating
  over bug H; bug Y's `wood` case runs alone in `known-red-gnome`. **The
  quarantine keeps CI honest; the bugs are still open.** Local
  confirmation: 847 passed, 1 failed (bug A), 43 ignored.

---

## WP-1 — Post the blocking decision cards (first hour; nothing waits on the answers)

**Why.** Both decisions gate later WPs, neither has ever been on the queue
(inbox reads 0 unanswered; none filed), and posting is fire-and-forget.

**Card 1 — "Who evolves: a new grazer, or the ant colony?"** Board
`creatures`. Present the plan's E5 call and its §8 questions 1, 2 and 4 in
one card: selection on individuals via budding on a **new solitary-grazer
ancestor** (ants and beetles stay authored showcase animals), reversible —
§7b colony selection returns the day a colony generation completes inside
~20,000 frames; what animal should the grazer *be* (it will be on screen);
and does S5 ship alone (two authored ancestors one number apart, different
colours, different places — visible divergence with no evolution in it)?
Attach a mockup strip if cheap (two recoloured ants on a filmstrip frame),
but do not block on artwork — the decision is textual.

**Card 2 — "How scarce should the forest floor be?"** Board `creatures`.
The measurement: deliveries +17% while moves −31%, digs −46%
(`README.md` M18 S1–S4 limitation #1) — the owner's "I don't want ants
sitting in one spot eating fallen leaves" arriving as numbers. Attach a
paired `filmstrip` A/B: current litter decay rates
(`assets/materials/litter.ron` `decay_chance_damp: 0.5 / decay_chance_dry:
0.1`) versus a substantially faster-rotting arm (e.g. 0.9/0.4) on the same
seed — **rebuild between arms** (`include_str!`), restore the asset after.
`meta` carries deliveries/moves/digs per arm from the paired `ascii` run.
Question: is the current abundance intended, or should leaving home matter
more? Note on the card that S7's whole premise (a larder behind a barrier)
only pays if the surface does not already feed everyone.

**Landing:** card ids recorded in your session summary. Then move on; do
not `--wait`.

## WP-2 — Rescue `Reports/foraging-range-measurement.md` (≤ 1 hour)

**Why.** The measured record behind the range instrument (19-cell bubble,
crowd-jam numbers, `FORAGE_TRIP_MIN` derivation, the
"instrument-was-behaviour-neutral-only-on-the-second-try" story) exists
only on `claude/creatures-m18-merge-ijdlnp`, a branch otherwise fully
superseded by main. Its index line already sits in `Reports/README.md`'s
in-flight section pointing at the branch.

**Steps.**
1. `git show origin/claude/creatures-m18-merge-ijdlnp:Reports/foraging-range-measurement.md > Reports/foraging-range-measurement.md`
2. Correct §0 with a dated note, do not delete it (a revert keeps the
   knowledge): the `creatures-m18` branch it records as unfindable was
   subsequently pushed and merged as `7b97c13` on 2026-08-23; §0's search
   record stays as the account of the gap. Update §5 "What is not done"
   the same way: the merge happened; the litter landing/rot items landed
   via `da252dc`/`bab6372`; the owner's litter-position call is quoted in
   §4 and was answered.
3. Move its `Reports/README.md` line from "In flight" into "Creatures and
   ecology" with a status ("**measured record, instrument landed via
   `da252dc`;** §0 corrected on landing").
4. `bash scripts/docscheck.sh` → clean. Commit (explicit paths), note in
   the PR that `claude/creatures-m18-merge-ijdlnp` now carries nothing
   unique and can be deleted — **do not delete the remote branch
   yourself.**

## WP-3 — Close the two open gates: bug H, then bug A (0.5–1.5 days)

**Bug H — `ascii`'s ants moisture-gradient scene** (`open-bugs-handoff.md`
§H; scene is `construction_scene()`, `examples/ascii.rs:273` call site,
assert near `:1850`).

- The record's steer: both halves print `0.000` — the *scene* no longer
  contains the gradient it asserts on; the deposition rule is not the
  suspect. The pre-merge branch printed the same 0.000s and *passed*: the
  assertion `wet_grad > dry_grad` turns on a margin below 0.0005 and flips
  with any perturbation (ci.yml's quarantine comment, measured runs
  137/139).
- ci.yml states the two acceptable closes — pick one or both: **(a)** make
  the scene genuinely contain a gradient (seed/maintain a wet region the
  field pass sustains through the run — check what originally built it and
  what field/weather rework removed it; `git log -S` on the scene function
  is the fast route), and/or **(b)** re-write the guard as a continuous
  margin per `CLAUDE.md` ("counts give knife-edge margins"): assert the
  *separation* (steep-half drops minus flat-half drops as a summed
  quantity, or mean |∇moisture| difference) exceeds a bar set from a
  measured distribution with headroom — after (a) gives it something to
  measure.
- **Landing checklist:** scene visibly shows the gradient in the `ascii`
  output; the guard fails when you delete the deposition bias (break the
  replacement to prove the guard bites); re-gate the `ascii` CI job
  (remove `continue-on-error` and the bug-H comment block); update §H to
  closed with the numbers; `docscheck` for the report edit.

**Bug A — the slot-1 root spread** (`open-bugs-handoff.md` §A). Plant-side,
but it is the one red test in the suite and the quarantine's exclusions
(`ci.yml:156,170` `--skip root_and_shoot_branching_read_different_slots`,
plus the `known-red-roots` job) want deleting the day it closes.

- **Do not move the bar and do not call it flaky** — §A settled that by an
  8-seed sweep on 2026-08-22 (the sign never changes; the lever is weak,
  not noisy). What changed since: the 2026-08-23 addendum shows the margin
  flips across the 10% bar with **litter volume** (8.2% red → green →
  6.8% red across the merge and `LITTER_FALL_REACH` 512), and it has never
  been seed-swept *with litter in the world*.
- **Step 1 is measurement, not code:** re-run the existing ignored probe
  `print_root_branch_slot_seed_sweep` (`src/sim/plant.rs`) on current
  `main`, 8 seeds both draws, and report the distribution. If the lever is
  dead across seeds (spread straddling ~7–8% everywhere), the fix lives in
  the plant genome's primed-site repair (`plant-genome-design.md` §8a) —
  that is real plant work; **timebox one day**, and if it does not fall
  out, append your sweep to §A and leave the quarantine standing. The
  gates stay honest either way; that was the point of the quarantine.
- **If you do fix it:** delete both exclusions *and* the `known-red-roots`
  job in the same commit, per ci.yml's own instruction.

## WP-4 — Re-baseline the guards and repair the instruments (1 day; after WP-3's bug-H fix so `ascii` runs clean)

**Why.** Every number in evolution-plan §4's guard table predates litter,
and the harnesses currently lie about their own parameters — the exact
failure mode the megastudy post-mortem (`CLAUDE.md`, "the harness is as
stale-able as the assets it reads") exists to prevent.

**Steps.**
1. **Echo fixes** (small, do first, zero behaviour change — verify
   counters reproduce `main` exactly, the way the range instrument was
   proven inert):
   - `examples/creature_space.rs`: the scene line hardcodes "4 beetles"
     against `const BEETLES: usize = 9` (`:177` vs `:735`); make the line
     print the consts, and echo preset, `START_ENERGY` (90), `move_cost`,
     and base seed (`0xC0DE`) on line one.
   - `examples/ant_ablation.rs`: echo `terrain=` and `food=` mode.
   - `examples/forage_probe.rs`: echo the seed.
2. **Deliveries across seeds** — the instrument the S1 notes named as
   missing and still is: add `seeds=N` to `forage_probe` (run both scenes
   per seed, print per-seed rows and a min/median/max summary for
   deliveries, trips, deepest). This is the harness every future foraging
   claim (WP-9, the abundance retune) must quote.
3. **Re-run the standing guards on one machine in one session** and update
   evolution-plan §4's table values with a dated note: forager-minus-
   immobile advantage at 3,000 ticks (`creature_space mode=economy`,
   reduced 4-seed reference pattern from §2.3's "As built" if the full
   3.5-hour sweep is too long — say which you ran), `ants fed`, `ascii`
   colony worst/mean (quote the mean; worst-frame spread is 3.5x on
   identical binaries, README's own rule), the reference-genome pair
   (`authored`/`zero` — `zero` must still read 0.300 to three digits; a
   moved `zero` means the harness broke, not the animal).
4. **Leave the scarcity-band re-derivation until Card 2 answers** — the
   band target is a game-feel call (§2.4's third coupled call); measuring
   the current state is yours, choosing the target is not.

**Known-good readings you are replacing:** advantage +0.187 / +0.247
(pre-litter), ants fed 0.42 / 0.55, authored 0.504 / zero 0.300, colony
mean 2.979 ms (post-merge). Deltas beyond seed spread are findings —
record them, don't tune them away.

## WP-5 — Bug Z: a particle must carry the corpse's worth (0.5 day)

**Why now.** A blasted corpse silently reprices 1,020 → 120 (§Z), no
existing guard can see it (`max_standing_meat` is a ≤ bound; biomass
monotone-non-increasing also passes on a loss), and it becomes an
evolutionary attractor the day S6 lands.

**Where.** `src/sim/particle.rs:71` (`Particle` has `material`, `shade`,
no `aux`); landing writes at `particle.rs:359` and `:385`
(`Cell::new(particle.material, particle.shade)`); spawn sites
`explosion.rs:1639` and `:1826` (`spawn_piercing(..., cell.material,
cell.shade, pierce)` — the `Cell` is in hand at both). Audit any other
`ParticleSystem::spawn*` caller that sources from a live `Cell`.

**Steps.** Add `aux: u16` to `Particle`, threaded from the source cell at
spawn; on landing, write it back **only when the landing material declares
`worth_in_aux`** (§Z's own fix shape — a wet soil grain must not land
claiming to be food; `aux` is a tagged union and this is its third
convention, see `Cell::aux`'s doc). `rigid.rs`'s aux-less `BodyCell` is
**deliberate** (Solid/Plant only; a landing body must not re-attach) —
leave it, and say so in the `Particle::aux` doc comment so the asymmetry
is recorded.

**Guard (write it to fail first):** a test that stamps a corpse cell
(e.g. worth 1,020 via `with_aux`), drives the real explosion path so the
corpse becomes a particle and lands, then asserts total standing meat
(census via `creature::food_value` over the world) is conserved minus what
the blast legitimately destroyed. Deliberately revert the fix and confirm
it fails (the corpse-shade guard in §2.3's "As built" note 7 is the
cautionary tale — its first version tested its own literal). Run both
drivers; the determinism pair (`tests/determinism.rs`) must stay green —
a new field on `Particle` must not perturb replay.

**Landing:** §Z closed with numbers; `wiki/ants.md`'s "a body is worth
what it was made of" promise now survives a blast — note it there if you
touch the page (WP-7 owns the page's other fix).

## WP-6 — Hook the `meat_lost` seam (0.5–1 day; pairs with WP-5)

**Why.** A corpse destroyed by fire, decay, explosion or the brush books
nothing (`world.rs:202-207`), so `max_standing_meat` is a hope, not a
bound. §13l's chain-corpse bug taught that "a bite, a fire, an explosion
and the brush all go through the same `World::set` seam" — but do **not**
put accounting in `World::set` (per-cell hot-path work; `CLAUDE.md`
"guard hot-path work at the call site that already has the data").

**Steps.** Add `EnergyLedger::meat_lost`; book it at the few call sites
that destroy a `worth_in_aux` cell with a stamp: fire burnout
(`fire.rs` — the corpse's `burns_into` path already special-cases
`worth_in_aux` for shade; the accounting hook goes beside it), decay if
corpse ever decays, explosion's cell-consumption path (distinct from the
particle throw WP-5 preserves), and the brush (`app.rs`/`main.rs` erase).
Then tighten the sealed-box guard: in
`a_sealed_colony_never_grows_its_own_biomass`'s world, standing meat +
`meat_lost` + live totals should close to the f32-rounding residue the
S3b notes measured (~0.3 over 40,000 frames) — measured against the run's
own pre-death baseline, never a fixed epsilon (S3b note 4).

**Do not** try to close the living-flesh stamp seam here — it is booked
with S6 (plan §2.3 "One seam left open"), because its sink (a parent
paying stamps) does not exist yet. Add a pointer comment next to
`meat_lost` so the two seams are found together.

## WP-7 — The docs honesty pass (0.5 day, mechanical)

Each of these is a recorded lie-in-waiting; all are one-file edits:

1. `wiki/ants.md:62-67`: says litter does not rot — it does
   (`wiki/plants.md:168-172` and `decay.rs` agree). Rewrite the paragraph,
   update the freshness note with a real date.
2. `wiki/powders.md:28-30`: "a corpse … nothing special about it any more"
   — since S3 its `aux` carries worth; half a sentence fixes the conflict.
3. `Reports/dead-ends.md:795`: still records the sealed-world test as
   `#[ignore]`d and failing; correct to "closed by S3
   (`creature.rs:3326`), horizon 80,000 measured-with-headroom" — keep the
   original text struck/annotated per that file's own convention.
4. `PLAN.md` M18 section: add a pointer to the live line
   (`creature-direction.md`, `creature-evolution-plan.md`,
   README M18 S1–S4) and mark the worm/binder/borer Phase-2 proposal
   **superseded by the evolution plan unless the owner revives it** (it
   was "awaiting sign-off" that never came). PLAN.md is contested — land
   this same-day.
5. Code comments (all in `src/sim/`): the orphaned `TUMBLE_ON_FAILED_MOVE`
   narrative stuck to `CROWDING_SCALE` (`creature.rs:~696`, cites a value
   that no longer exists); "The 14 brain inputs" (`creature.rs:~988` — 16);
   `brain.rs:44-47` "248" as the tight-layout count (268 with Feed);
   `creature_space.rs:177` is fixed by WP-4. Drop `step_chain`'s unused
   `material_id` parameter (`creature.rs:~1506`) — a signature change, so
   run clippy and both test profiles.

## WP-8 — S5: `gut_bias`, diet as one heritable number (2–4 days; spec is plan §2.5, unchanged)

**Read plan §2.5 and §6 first.** The spec is complete there; this WP adds
only sequencing and code anchors.

**Build order:**
1. **Trait storage.** `OrganismState::traits: [f32; N]` (start N=1,
   `traits[0] = gut_bias ∈ [−1, 1]`), authored ancestral value and
   per-trait mutation width in species data (the plant
   `genotype_variance` pattern, `organism.rs:~411`). Do not put it in the
   584-slot brain genome — body/trait slots are a separate block by
   design (S8 will grow it).
2. **The matched filter.** `yield = food_energy * (1 − |gut_bias −
   food_class| / 2)²` — no transcendentals, no free dimension.
   `Material::food_class` is already authored on all six foods
   (`material.rs:416-459`).
3. **Wire it into both the eat verb and the eye.**
   `adjacent_food`/the menu test (`creature.rs:~1090`, currently
   `def.food` membership) becomes `yield > threshold`;
   `BrainInput::FoodAdjacent` reads the **same** predicate — a meat-gut
   animal stops seeing leaves (plan: "the gene must change behaviour, not
   just bookkeeping").
4. **Delete `CreatureDef::food`** and bank the promised hot-path saving
   (~32 string hashes per creature-tick, S3's unbanked claim). Beware the
   cannibal trap that kept the list alive through S3: `ant` material
   carries `food_energy: 120`, so pure "edible = yield > 0" at neutral
   gut_bias re-opens §13i's colony-eats-its-own-dead loop. Set the
   authored ant `gut_bias` and threshold so ant-flesh yield for ants
   lands below threshold, and **re-run the sealed-box guards** — they are
   the regression net for exactly this.
5. **Colour.** Lerp the creature palette by the trait — and calibrate
   expectations against the corpse-ramp lesson (plan §2.3 "As built" #7:
   two pixels cannot carry a quantity; the owner judged a widened ramp
   "pretty minor"). The readout that answers "how much" is
   `OrganismOverlay::FoodValue`'s sibling — add `OrganismOverlay::GutBias`
   as a full-replace ramp **before** anything reads the trait (house law:
   the overlay precedes the mechanism).
6. **Measurement harness before ancestors.** A `creature_space` mode
   sweeping authored `gut_bias` ∈ {−1, −0.5, 0, +0.5, +1} × 8 seeds,
   paired: two-humped survival curve in a mixed litter+carrion scene;
   single-peaked at the herbivore end in a litter-only control (a
   two-humped curve there is a sweep bug); both-ancestors-at-0 separation
   ≈ 0; `placed` printed per arm (the spawn-layout trap has fired three
   times). Set the separation bar against measured seed spread, not "two
   local maxima".
7. **Expect one hump on the first run** — carrion is scarce until
   something dies. The fix is ecology (seed carrion into the scene / raise
   `structural_fraction`), not the filter (plan's own falsifier note).
8. **The two shipped ancestors wait for Card 1's verdict** (which animal
   carries this on screen). Everything above ships and is measurable on
   the existing species without it.

**Landing:** sweep table in the commit message; sealed-box guards green;
overlay + a review card with the curve and two filmstrip arms; wiki page
updated if player-visible behaviour changed (it will — diet selectivity).

## WP-9 — Traffic and range (2–3 days, parallel-safe with WP-8 only via separate worktrees; both touch `creature.rs`)

**Why.** The bubble is 18 cells; the 55-ant arm reaches 12 vs a lone
ant's 42 with ~60% of moves blocked; jamming was severe enough that
founding spaces ants 4 apart. Dead-end 775/829's own condition line says
it "reopens if creatures gain pass-through or climb-over" — this WP is
that re-test, done deliberately. It is also the most visible near-term
win: a colony *streaming* instead of milling is the satisfying outcome
the project optimises for.

**Arm 1 — climb-over (foothold, not passability).** In the footing
predicate (`head_has_foothold` / the support scan in `step_chain`,
`creature.rs:~1585-1618`), count a same-species creature cell as
*support* — an ant walks over a nestmate like terrain. Do **not** make
creature cells passable (two multi-cell chains swapping through each
other is a different, much harder change; a creature cell remains an
obstacle to *enter*). Gate it on species data (`CreatureDef` field, e.g.
`climbs_over_kin: bool`), defaulting off — the dispatch site already
holds the def (house rule: opt-in on the species, tested where the data
already is). Mind the recorded footing dead-ends while in there: footing
is an additive bonus, never a multiplicative discount (dead-end 843), and
"no purchase anywhere ahead counts as blocked" is the load-bearing
predicate (837).

**Arm 2 — dispersal via the existing genes.** No new mechanism: author
`(Crowding, Tumble, w)` into `ant.ron` (re-orient more when crowded) and
sweep w ∈ {0.5, 1.0, 2.0}. **Rebuild between sweep points** and prove the
edit moved only its knob (`tree.ron`'s double-`crowding_weight` sed
disaster is the recorded warning). Do not touch `(Crowding, Move, −0.3)`
— ablating it cost 69% of deliveries (§13f).

**Measurement (the WP-4 instrument):** `forage_probe seeds=8`, paired
against baseline in the same session — success is weight appearing in the
≥32/≥64 reach buckets **while deliveries hold within seed spread**;
today's known-good is deepest 12–18 with ≥32 at zero. Also quote
`moves_blocked / moves`. Then the eye: a `filmstrip` pair (jam vs
streaming) posted as a blind A/B with the trip counts in `meta`.

**Decision rule:** if one arm clearly wins on the profile with deliveries
flat, land it default-on for ants with the numbers in the commit; if the
gain is marginal or the look divides, land default-off behind the species
flag and let the owner's card verdict flip it. Either way append the
775/829 re-test result to `dead-ends.md` (that entry's condition was
explicitly waiting for this).

## WP-10 — Gated next stages (do not start; here so you know what unblocks)

- **S6 reproduction** — starts when Card 1 (E5) is answered. Spec: plan
  §2.6, including both pre-flights (the `MoistureLateral`
  spurious-constant probe; publish the clonal-drift band before setting
  any bar). Constraints from the wider record, non-optional: child
  genome/draws keyed on the child's handle, never its slot id (dead-end
  670 — generation wrap clusters spatially); a birth scheduled mid-tick
  must not take the live heap out from under the scheduler (dead-end
  1094); a **release-mode** guard on the 4095-slot ceiling before any
  breeding run (population-dynamics acceptance 9g — the debug job now
  compiles the `debug_assert`s, but release play still doesn't run them);
  close the living-flesh stamp seam here (WP-6's pointer); population
  readout + `births`/`births_denied_no_space` counters first; asserts on
  populations and paired comparisons only (dead-end 552).
- **S7 two larders** — starts when the owner picks buried vs canopy
  (Card 1/2 follow-up; the review report §T5 has the framing). The
  reciprocal transplant runs on generated terrain, never a hand-built
  scene (dead-end 787).
- **Predation probe** — cheap and parked: wire the beetle to the
  channel-B along-gradient (one `.ron` edit **plus rebuild**) behind the
  pre-flight printing total channel-B mass and the fraction of prey heads
  within a sensor offset of nonzero B (plan §5). Worth doing in an idle
  hour after WP-4; not before, since its verdict is only meaningful with
  honest harnesses.

## Sequencing

```
WP-1 (cards)  ─── post first, answers arrive whenever
WP-2 (rescue) ─── any time, independent
WP-3 (gates)  ─── before WP-4 (ascii must run clean)
WP-4 (instruments/baselines) ─── before WP-8/WP-9 measurement claims
WP-5, WP-6 (accounting) ─── before S6 ever starts; independent of WP-4
WP-7 (docs)   ─── any time, land PLAN.md piece same-day
WP-8 (S5)     ─── after WP-4; ancestors after Card 1
WP-9 (traffic)─── after WP-4; separate worktree from WP-8
WP-10         ─── gated on verdicts
```

`src/sim/creature.rs` is the collision hotspot (WP-5, WP-8, WP-9): serial
order or separate quick-landing PRs, per CLAUDE.md's contested-file rule.

### Running this with more than one agent: the lane split

A single agent following the WP order above is fully viable — the packages
are already sequenced for it. If parallelising, use **these lanes and no
finer**: the split is drawn on file ownership and on who produces the
numbers others quote, which are the two ways parallel work has actually
failed here (the `src/app.rs` collisions; the ten-branch fan-out that
never pulled main forward — both recorded in CLAUDE.md).

| Lane | WPs, in order | Owns (files) | Starts |
|---|---|---|---|
| **A — gates & instruments** | WP-1 → WP-3 → WP-4 | `examples/*` (`ascii`, `creature_space`, `forage_probe`, `ant_ablation`), the review queue, `ci.yml` | now |
| **B — accounting & docs** | WP-2 → WP-7 → WP-5 → WP-6 | `Reports/*`, `wiki/*`, `PLAN.md` (land same-day), `particle.rs`, `explosion.rs`, `fire.rs`, `world.rs` ledger | now |
| **C — S5 & traffic** | WP-8 → WP-9 (serial within the lane) | `creature.rs`, `organism.rs`, `material.rs`, `brain.rs`, `render.rs`, `assets/species/*` | when Lane A lands WP-4 (or immediately for WP-8 steps 1–5, deferring every measurement claim until WP-4 is on main) |

Rules that make the split safe, all already paid for:

1. **One lane owns `creature.rs` at a time.** Lane B's only touch is
   WP-7's comment fixes — land those in its first day, before Lane C
   starts. Lane B's WP-5/6 guard tests go in `particle.rs`/`world.rs`
   test modules, not `creature.rs`, for exactly this reason.
2. **Lane A is the source of truth for numbers.** No other lane publishes
   a foraging/economy claim measured on pre-WP-4 instruments; a lane that
   needs a number Lane A has not produced yet measures it locally, says
   so, and re-measures after WP-4 lands. Baselines are still same-session,
   same-machine, per lane — house law, unchanged.
3. **Each lane: own worktree, own `claude/**` branch, PR opened on day
   one** (CI now runs per-branch — this is what the last fan-out lacked),
   `branchcheck.sh` at session start, and pull `main` in daily. Landing
   order when PRs stack: A, then B, then C.
4. **Verdict-gated work stays gated regardless of headcount.** More
   agents do not unlock S6 or S7's larder; only the WP-1 cards do. Do not
   spend a third agent on WP-10 speculation while the cards are
   unanswered.
5. **Do not go wider than three.** WP-10 is gated, the remaining packages
   are small, and the owner's review bandwidth is the real bottleneck —
   three lanes already produce three PRs and several cards into one
   person's queue.

## Reporting back

Per session: verdicts collected from the inbox; per WP landed: the
before/after numbers in the commit, gates run and named, review cards
posted with counts in `meta`, and the relevant report/wiki updated in the
same change. If a WP's measurement contradicts this document or the
review, the measurement wins — append the correction to the source report
rather than silently deviating, the way §13's own corrections were kept.
