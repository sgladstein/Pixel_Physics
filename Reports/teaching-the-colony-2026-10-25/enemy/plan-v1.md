# The enemy seat demo: implementation plan, v1 (for independent review)

*2026-10-25, enemy seat thread. Built on the feasibility review (`review-feasibility.md`, verdict: feasible with
changes, as the Raider and not the Pit). Branch `claude/project-thread-1ekeut`. This plan is v1; it will be reviewed
by an agent that did not write it, then revised.*

## What the owner will play

`cargo run --release --bin lab -- demo=enemy` opens the lab on a box with a living, trained ant colony that has had
time to dig. **You are a centipede** (the shipped nest raider: a 5-cell chain, jaw 0.5, armour plate 2.0, the trained
raider mind under you). The keys drive your body; the colony runs on its own trained brains. The HUD tells you, in
words and numbers, what you have done to the colony and what it has done to you.

- **Arrows** creep (the chain follows its head; held into a wall you climb).
- **B** bites a foe touching your head. **G** shoves through crumbs / cuts what your jaw can. **E** takes flesh or
  a corpse into your crop. **Q** puts down what you hold.
- **Tab** camera follow (queen's launcher). **H** help card. Lab time dial on PageUp/PageDown (launcher).
- **Y** "season card": a summary page (below). **U** switches the colony's brain on screen (trained / hand-built),
  the runtime selector `CLAUDE.md` asks for when "which looks right" is the owner's call.

**The loop:** find a door, creep in, bite, eat, get out before the bites take your length. Your length is your
health: every 4 ant bites take a cell, 5 cells to 1, a bitten head is death. Your hunger is the clock: a full crop
cannot bite until it digests (the game's own satiation), and an empty tank starves you. So you cannot simply eat the
colony: you have to come and go.

**What it honestly is:** a colony that **fights back**, not one that learns you. The help card says so in one line.
If the gated Season 2 (part B) passes, a second season shows a colony bred against raiders.

## Part A: the Raider (playable demo). Steps, each with its proof

### A0. Two measurements before building (half a day, CPU)

1. **The box.** A lab scenario in which the *trained* colony is alive at 60,000 frames on 3 of 4 seeds with no
   raider, and still alive at 30,000 frames after a scripted raider hunt (A5's autopilot) lands at 20,000. Candidates
   from the `std_pool.txt` header (alive at 15k): the "provisions 100" and "fruit 100" boxes. Tool: `labsurvive`,
   then `spiderlab species=centipede ant=std` for the raider arm (the reviewer's one run: trained colony 162 ants at
   60k with raiders breeding to 72). **Proof:** a table of ants alive by seed and frame, both arms. If no box holds,
   the demo opens on a provisioned box (the colony starts on the pile) and says so.
2. **Step 0a, the alarm trace** (design step 0, still owed). Drop alarm (the lab's J tool, `ALARM_DEPOSIT`) next to
   working trained ants; trace every ant within 10 cells for 300 frames (position, verb chosen, alarm read in its
   options). Positive control: a 20x dose. **Proof:** per-ant rows; the answer to "do trained ants go toward a bite,
   away, or ignore it". It decides whether the HUD may say "they come to the bite" (mob) or must not.

### A1. Steer any body with a mind (shared piece; DONE in code, PR pending)

`steer::take_forced` / `steer::choose` / `World::mind_force` in `mind::tick` (shared/steer-mind.md). The player
picks the mind's option: dir to the nearest Walk (else TurnAround, else straight up when held level into a wall,
else Rest), B/G/E/Q/F to Attack/Dig/PickUp(Eat)/Drop/Feed nearest dir. Memory and pull follow the forced verb.
**Proof (in the PR):** a steered trained ant and a steered centipede go 10+ columns east in 300 frames where the
unsteered one does not; an entry for a dead id leaves the box hash-identical; all three go red with the hook removed
(watched). Off is one `Option` read plus one `is_empty`.

### A2. `demo=enemy` on the shared launcher

Depends on queen's launcher PR (`Lab::demo: Option<DemoSeat>`, `assets/lab_scenarios/demo_<seat>.ron`, key layer).
Mine: `src/lab/demo/enemy.rs` + `assets/lab_scenarios/demo_enemy.ron`.

- The scenario: A0's box, colony at frame 0, **warm start**: `PIXEL_PHYSICS_LAB_TICKS`-style pre-run to ~20,000
  frames before the first displayed frame (a nest dug, a store laid), then one centipede placed at the surface
  `far=30` columns from the nearest door (`train::predator::place_predator`, lab raider pool member 0), marked as the
  steered body with input `Some`.
- **Player body rules** (enemy.rs, all scoped to the steered id, nothing else changes):
  - no old age (already: `steer::is_steered` skips the hazard roll);
  - **no budding** for the player: the steered body's `reproduce` gate returns early (a one-line steer hook in the
    birth check, like the old-age one). Owner question 3 in the review; recommended off, because 2 raiders became
    72 in the reviewer's run and the box stops being you against them. AI raiders can be stocked by hand (COLONY chip).
  - the queen breeding rule `mark` applies is harmless (a centipede colony of one) but is skipped for a non-ant
    (`Steered::colony` set to `u32::MAX` for the demo's mark).
- **Proof:** a test that `demo=enemy` loads, a centipede is steered, the box with `demo=None` is byte-identical to
  today's (hash over 3,000 frames, as `a_steer_on_no_living_body_changes_nothing`).

### A3. The HUD and the marks (what makes it satisfying)

Every verb must leave something. What the engine already leaves: a corpse (a powder that decays, 120 J of food), an
alarm flash (3 s, 6 cells), a bitten-off segment (a shorter you), a moved store. What A3 adds is **legibility**:

- **HUD line** (top-left, the lab's own text): `LENGTH 5/5 (plate 3/4)`, `CROP full: digesting 4s` / `CROP empty`,
  `TANK ####----`, `KILLS 3  WOUNDS 2  BITES TAKEN 7`, `ANTS 162  LOADS HOME 412  PRICE 7.3 kills / 1,000 loads`.
  Sources: the body (`chain.len()`, `crop`, `energy`), `World::kills_log` filtered on `detail.attacker == steered id`,
  `CreatureStats::deliveries` since the player landed, the body's own gnawed counter for bites taken (or
  `encounter_log` set to `Some` in demo mode only).
- **Verb feedback:** a pressed verb that had no option (sated, nothing in reach, jaw too weak) prints one HUD line
  for 2 s: `FULL -- DIGESTING`, `NOTHING IN REACH`, `TOO HARD TO CUT`. Read from `steer::choose` falling through
  (exposed as `steer::last_forced_fell_back`).
- **Marks on the ground (demo-only draw, no physics):** a short red ring where you killed (the kills_log position,
  fades over 10 s), a white tick where a bite landed on you. Corpses are already drawn by the world.
- **Death and respawn:** when the player dies, the season card opens (A4); `R` (seat key) puts a new centipede at the
  surface far from the doors.
- **Proof:** a GIF (`labgif`-style capture of `Lab::draw` with the demo HUD) of an autopiloted hunt (A5), the HUD
  numbers matching a printed log of the same run (kills and loads counted twice, independently).

### A4. The season card (Y, or on death)

A page in the lab's panel style: time hunted, kills, wounds left, bites taken, length lost, food the colony carried
home while you hunted, **kills per 1,000 loads** (the lone-predator report's metric; low means the colony foraged
safely around you), ants alive at start and now, and *which lines you ate* (pool member of each victim: recorded at
the kill into a demo-side list, since a dead ant's mind leaves with it). One plain sentence at the bottom chosen from
the numbers ("They kept foraging; you ate 1 in 140 trips" / "They stopped foraging while you hunted").

**Proof:** a screenshot of the card after the autopilot hunt, numbers checked against the run log.

### A5. The autopilot (for proofs only, not a game feature)

`examples/enemyhunt.rs`: loads `demo_enemy.ron`, drives the steered centipede by a scripted policy (walk to the
nearest door, enter, bite whatever touches the head, eat when E is offered, leave when length < 3 or crop full, rest
on the surface to digest), writes the GIF and a per-event log (bite, kill, wound, delivery). Paired seeds. This is
how every proof above is produced headlessly, and how B's judge replays "the player's hunt".

### A6. Frame cost

`examples/ascii.rs` worst-frame is untouched (demo off). With the demo on: `labperf`-style µs/tick of the demo box
with and without the HUD, quoted in the PR. One extra creature and a text line; expected noise.

### A7. Land it

PR 1: steer-mind (A1), now. PR 2: demo=enemy + HUD + card + autopilot + `Reports/enemy-seat-demo-2026-10-25.md`
(the review, this plan, the measurements, the pictures) + `Reports/README.md` line. Artifact page with the GIF and
the card for the owner. Merge desk merges.

## Part B: Season 2 (gated stretch)

**The question:** does a trained line bred against raiders pay the player a lower price per delivery than a line bred
for the same rounds with no raider? (The lone-predator report: the spider-free line improved just as much, 4.6 vs
3.2-3.8 killed per 1,000 deliveries, so the control is not optional.)

- **B1 (start day 1).** Two in-run colony-selection lines from the current Teacher-lane founder pool (the ratchet
  rule: latest best pool, not an old one), same rounds, same seeds: `inrun spider_species=centipede` with N raiders,
  and the same with 0 raiders. Compute: GitHub Actions or a cloud runner thread (owner: no waiting on the VM); this
  4-core box can run a small batch overnight if nothing else is free. Tell the Teacher lane before it runs, since it
  is ant training on their line (project rule).
- **B2. The judge.** The autopilot hunt (A5) on paired seeds against each line's best pool in the demo box: kills per
  1,000 loads, ants alive at the end, loads home. Gate: the raided line beats the control on at least 6 of 8 paired
  seeds and its loads home are not lower (no hiders: the critique's rule).
- **B3. If it passes:** `demo=enemy` gets a Season 2 (seat key P): the same box, founders from the raided line's
  pool, the card showing Season 1 and Season 2 prices side by side, and a line saying where the change came from
  ("bred off-screen against raiders for N rounds; a control bred without raiders did X"). **If it fails:** the demo
  ships without Season 2 and the report says so.

## Rules check

- One physics: no physics change. The only engine change is A1 (decision override, off by default) and A2's
  player-only budding gate.
- One brain design, no castes, no scripted answers: the colony is the trained pool untouched; the autopilot drives
  only the player's body.
- Dig and forage all game: the colony's own; the price-per-delivery readout rewards a colony that keeps foraging.
- Every verb has a visible consequence: corpse, alarm flash, segment, moved store, plus A3's marks and HUD lines.
- Graded: wounds before kills; length lost a quarter-cell at a time; price per delivery is a number, not a win/lose.
- Frame cost: one body, a HUD line; A6 measures it.
- Shipped lab unchanged: everything behind `demo=enemy`; A1/A2's hooks are off-is-off, hash-tested.

## Shared pieces and other seats

- Uses queen's launcher and key layer (shared/launcher.md, shared/steered-ant.md). Seat keys used: Y, U, R, P.
- Provides steer-mind (shared/steer-mind.md) to scout (tandem followers) and anyone with a mind body.
- Guest's scent ladder (shared/scent-ladder.md) is not needed for A: mind ants bite the raider by colony number.
- Queen + enemy "switch sides" is a later combination: Season 2's pool could be the queen game's founding stock.

## Open questions for the owner (asked on the thread card)

1. Learning: gated Season 2 (recommended) / none / mutation at birth in demo mode.
2. The colony brain: trained by default with an on-screen switch to hand-built (recommended), since the trained colony
   barely fights a raider and the hand-built one does.
3. Player budding: off (recommended).

## Estimate

A: 1.5-2 days of agent work after the launcher lands (A0 half a day of CPU in parallel). B: a day of work plus
compute; the judge is cheap once A5 exists.

## Revision notes (collected while the plan is in review)
- "Loads home" must read `creature_stats.trip_deliveries`, never `deliveries` / `MindCounts::delivered`: those count crumbs picked up inside the nest and put straight back (2.4-5.8x inflation; how-we-test.md s4).
