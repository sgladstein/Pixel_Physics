# Review of the enemy seat plan, v1

*2026-10-25. Independent reviewer; I did not write the plan, the feasibility review or the A1 code. Read against the
working tree of `claude/project-thread-1ekeut` at `9e32979e` plus the uncommitted A1 diff (`src/sim/steer.rs`,
`src/sim/creature/mind.rs`, `src/sim/world.rs`), and against the plan as revised at 09:26 (the `trip_deliveries` note
is already in). Nothing was built or run; every claim below is from the source, with file:line.*

## Verdict

**Ready with changes.** The scope (the Raider, honestly labelled "fights back, not learns") and the A1 code are sound, but
the core loop the plan promises (length as health, hunger as the clock, verb feedback) rests on three engine facts that
are not as the plan states them, and Part B's gate is too weak and breeds against the wrong threat.

## Must-fix

**M1. "Your length is your health" is not how a chain body dies.** Damage is one number per body
(`OrganismState::gnawed`, `mind.rs:2714-2716`): when it crosses 1.0 the cell that took *that* bite comes off. If that
cell is the head, the body dies at once (`reconcile_chain`, `creature.rs:1740-1761`). If it is a middle segment,
everything behind it is severed and becomes meat (`creature.rs:1767-1792`). So a 5/5 centipede can die on one bite,
and a neck bite takes four cells at once. Head-first in a tunnel, the head and neck are the cells the ants can reach.
The HUD line `LENGTH 5/5 (plate 3/4)` would then be a lie, and death would feel binary, which is the defect the ethos
names first.
**Fix:** in A5, log which chain index each lost cell had, over paired seeds. Show the distribution in the report. Make
the HUD say what is true: a wound meter (`gnawed`, quarter by quarter) and "the next cell lost is the one they bite:
the head kills, the neck takes everything behind it". This also gives the player a real tactic. Turn round
(TurnAround) and leave tail-first toward the ants, so the bites land on the tail and cost one cell at a time. The
severed tail is meat the ants carry home, which is a visible consequence. Put that tactic on the help card.

**M2. Satiation is not a clock: Q defeats it, and E does nothing for this gut.** The bite refuses only when the crop has
no room for the cell (`mind.rs:2700-2703`). Put Down also puts a crop cell on the ground (`Verb::Drop` doc,
`options.rs:91-94`; the nest-raider report's "a raider could farm its own score"). So B, Q, B, Q kills without limit,
and "you cannot simply eat the colony" is false. Meanwhile E (Eat) only sets `eat_lunch_now`, which is ignored when
`handling` is on (`creature.rs:9039`), so for the centipede E while full is a verb with no consequence. Nobody has
measured the digest wait. An ant cell is 480 J (`assets/materials/ant.ron:36`), and the crop holds three cells
(`centipede.ron`, 1,440 J). If `digest_rate: 3.3` is J per body tick, one cell takes about 730 frames (12 s at 1x).
That is my estimate, not a measurement.
**Fix:** measure the per-cell digest time at 1x in A0. Then put the surplus-kill question to the owner: spitting a
cell lets you kill again, and the flesh stays where the ants can take it. That is graded and visible, and may be a
better verb than a lock. Either way, the "come and go" sentence must not claim a ceiling the engine does not enforce.
Give E an honest HUD answer for a handling gut ("YOUR GUT SETS THE PACE: 9 s").

**M3. The verb feedback cannot come from `choose` falling through.** Attack is offered for any stranger in reach,
sated or not (`options.rs:905-907`). The refusal happens inside the act: sated at `mind.rs:2701`, armour too hard at
`mind.rs:2707`. So a full centipede pressing B gets an Attack option, `choose` picks it, `act` returns false, and
`last_forced_fell_back` would read "no fallback". The opposite case is worse. With a crumb pellet in the jaws, Dig is
not offered unless the body is walled in (`options.rs:80-85`, `pack_on`), so G falls through to the direction rule and
the centipede *walks* (`steer.rs:389-397`), which the player will read as a broken key.
**Fix:** record the steered body's outcome after `act` (`mind.rs:2573`): verb asked, verb chosen, `did`. One write,
guarded by `forced.is_some()`. Derive the three messages from that record, and add `JAWS: CRUMB (Q PUTS IT DOWN)` as a
standing HUD state.

**M4. `Steered::colony = u32::MAX` is unsafe, and the budding gate must not be "steered means never breeds".**
`steer::grant` books to `Steered::colony` (`steer.rs:163-175`). `World::book` then indexes `colony_books` by colony
and resizes the Vec to `colony + 1` (`world.rs:7995-7999`, `8049-8054`). That is about 4 billion `ColonyBooks`: an
out-of-memory abort the first time anything (the queen launcher, a lab tool) grants the player energy. It is also
unnecessary. The only other reader is `regime_of_colony` (`creature.rs:23030-23035`), and the queen rule on a colony
of one body that does not bud changes nothing. Separately, `try_bud` is also where eggs are laid
(`creature.rs:5956`, inside `try_bud` at `5630`). An unconditional "the steered body does not reproduce" hook would
stop the queen seat's queen from laying.
**Fix:** leave `colony` alone, or use the queen seat's agreed `SteerRole::Worker` (`shared/steered-ant.md`). Put the
gate behind a field on `Steered` (for example `may_breed: bool`, default `true`), read at the top of `try_bud` beside
`births_paused` (`creature.rs:5637`). Say in the plan that this is an engine hook in `creature.rs`, not an `enemy.rs`
rule.

**M5. The demo may have no danger, and nobody has measured it.** The feasibility table's one run has the trained colony
killing 4 raiders in 34,000 frames, against 14 for the hand-built ant. The nest-raider report found the trained nest is
a bowl of packed crumbs with ants in one-cell pockets (`Reports/nest-raider-2026-10-25.md:28-35`). A player who shoves
in and finds ants that cannot reach them, and do not bite, has a buffet. Within two minutes that is boring, and length
as health never matters. The alarm trace (A0.2) decides HUD wording. It does not decide whether the game has a fight.
**Fix:** add A0.3: bites landed on a centipede inside the nest, per minute at 1x, for both brains, on paired seeds.
Use a proxy before A5 exists (below). Put the number in front of the owner with question 2. If the trained colony
lands under about one bite a minute, recommend opening on the hand-built colony, or a launch-time choice, rather than
trained-with-a-switch.

**M6. `U` as a live switch breaks the player's body and confounds the comparison.** `set_new_ant(false)` sets
`world.minds = None` (`lab/mod.rs:1041`). That drops the centipede's mind as well. For up to `NEW_ANT_SWEEP` = 16
frames (`lab/mod.rs:4637`) the player is on the hand-built path, with slice-0's ant hooks on a chain body. Then
`give_predator_minds` hands it member `id % len` with fresh memory (`lab/mod.rs:1106-1135`), not member 0. The swap
also puts hand-built ants into a nest the trained colony dug, mid-hunt. That is not the paired comparison `CLAUDE.md`'s
runtime selector is for.
**Fix:** make `U` rebuild the box with the other brain from the same seed, then warm-start it again. Or take the brain
as a launch argument (`demo=enemy oldant`). Name the active brain on the HUD.

**M7. A0 cannot run on day 0 as written, and its boxes are not shipped.** A0.1's raider arm is "A5's autopilot", which
needs A1, A2 and the launcher. The candidate boxes come from the pool header (`assets/minds/std_pool.txt:4`). They
were measured with `labsurvive grant=1`, at 15,000 frames, on 2 seeds. The demo needs 20,000 warm frames plus a
session of 20,000-60,000. No shipped scenario grants a tank on landing. "Provisions 100" and "fruit 100" are not in
`assets/lab_scenarios/`, and `scratch/inrun/labscen/` holds only three boxes.
**Fix:**
- Run A0 with `spiderlab species=centipede ant=std spiders=1` and budding off, or `pool=blank far=0`: the raiders that
  actually went in (nest-raider report, 30 ants to 8). Re-run it with A5 later.
- Make the box an `assets/lab_scenarios/demo_enemy.ron` that reproduces the grant. If the scenario format cannot grant
  a tank, add the grant to the demo's load step and say so.
- Measure survival to 80,000 frames on at least 6 seeds, not 3 of 4.

**M8. Two of the "already visible" consequences are not drawn.** The alarm plane is drawn only under
`FieldOverlay::Alarm` (`render.rs:1012-1047`), so in the normal view a bite's "alarm flash" is invisible. `KillRecord`
has no position (`world.rs:563-576`), so "the kills_log position" for the red ring does not exist.
**Fix:**
- Draw a bite pulse at the attacker's head when a bite event involving the player appears. Set `encounter_log` to
  `Some` in demo mode, and drain it every frame, keeping only rows where `a` or `b` is the player (see S4).
- Place the kill ring at the player's head on the frame the new `KillRecord` with `detail.attacker == player` appears.
  `detail.attacker` is filled (`mind.rs:2744`, `kill_detail(world, verb, organism, victim, ...)`).

**M9. Part B's gate is too weak, and its bed predator is not the player's threat.**
- **Too weak.** A sign test of 6 of 8 passes about 14% of the time when there is no effect. The arms are one line each,
  but the lone-predator report found that replicate lines swing more than arms do (garden food 7,141 against 2,656 for
  two plain replicates). It concluded "16 colonies cannot rank these arms. A defence case needs the 64-colony scale"
  (`Reports/lone-predator-2026-10-24.md:111-114`). Eight paired *judge* seeds sample the judge's noise, not the
  between-line noise that sank the last attempt.
- **Wrong threat.** The lab's trained raider pool "makes a population by keeping its distance from the nest". None was
  in the shaft or a chamber in any frame (`Reports/nest-raider-2026-10-25.md:117-122`). A line bred against it is bred
  against surface predation at the far edge of the foraging range, and is then judged on a nest raid.
- **Judge not proven sensitive.** If the autopilot leaves when its crop is full, its kill count is capped by its gut, so
  "kills per 1,000 loads" may move only through the denominator, that is, through foraging.
- **Confounded comparison.** Season 1 is `std_pool` and Season 2 is a line bred for N more rounds, so the side-by-side
  card mixes training rounds with the raider.
- **Lab survival not selected for.** `std_pool` was selected with a `+lab` case (`std_pool.txt:1`). A line selected
  without it may starve in the box whatever raiders do (`labsurvive`, `Reports/instruments.md:229`).

**Fix:**
- Breed two replicate lines per arm, using std_pool's case set including `+lab`.
- Use a raider that goes in: `far=0` with the blank pool, or the autopilot itself as the bed's predator. A scripted
  *predator* breaks no colony rule.
- Before reading the gate, show the judge's positive control: it must rank the hand-built colony below the trained one
  on price, on the same seeds. Also show its null: replicate against replicate within an arm.
- Gate on both raided replicates beating both controls, at 10 or more of 12 paired seeds each, with `trip_deliveries`
  and ants alive not lower.
- Count "autopilot reached the nest interior" on every judged seed, so a stuck bot cannot read as a defended nest.
- Make Season 1 the control line, or show all three (shipped, control, raided) on the card.

**Compute estimate (my arithmetic; the plan has none).** Assume the feasibility run's rate of about 1,000 frames a
second per core for a 100-160-ant box, and the lone-predator scale of 16 colonies × 10 rounds at about 150,000 frames
a round. One line is then about 24 million frames, or 7 core-hours. Four lines (two arms × two replicates) are about 27
core-hours: roughly 7 hours wall on this 4-core box with nothing else running. At the 64-colony scale the last report
asked for, it is about 110 core-hours. That is not "a small batch overnight". Time one round first and put the number
in the plan.

## Should-fix

**S1. The warm start blocks the window for 15-25 s.** The `PIXEL_PHYSICS_LAB_TICKS` loop runs before the window exists
(`src/bin/lab.rs:315-321`). At the measured 1,000-1,200 frames a second, 20,000 frames is 17-20 s on this box, and
there is no world save to load instead. A blank start reads as a hang. Open the window and run the warm-up visibly at
the top dial, with a progress line ("THE COLONY IS DIGGING IN: 12,000 / 20,000"). Watching the nest form in twenty
seconds is also the best opening shot the demo has.

**S2. The first five minutes need a goal on screen.** The plan says "find a door", but nothing marks one, and the
trained colony's door may be a crumb mound. Underground legibility was already a problem once: `train::predator::picture`
had to darken tunnels because "the first pictures drew a tunnel the colour of the sky" (`src/train/predator.rs:214-218`).
- Mark each `nest_sites` door.
- Use the queen launcher's ring on the player's body.
- Check, in a screenshot of the real lab renderer with the cutaway on, that a 5-cell centipede in a crumb bowl can be
  told apart from the crumbs.
- Show raw counts on the HUD until there are at least 200 loads. `PRICE` over a small denominator jumps around
  (1 kill in 12 loads reads 83).

**S3. Define the end states.** Say what happens when the colony is wiped out (feasibility question 4). Say what happens
above the steering cap: `Steered::input = None` is "her genome drives her" (`steer.rs:110-112`), so at 16x the raider's
own mind takes over. That could be a feature ("let instinct hunt"), but it must be shown on the HUD. Allowing 4x while
the player rests to digest would remove most of the waiting.

**S4. Bites taken.** `gnawed` resets when a cell comes off and falls as wounds heal, so it is not a counter.
`encounter_log` is world-wide and bounded at 2 million events (`world.rs:641-664`). Drain it every frame, keep only the
player's rows, and count `ENCOUNTER_BITE` rows with `b == player`.

**S5. "Which lines you ate" is feasible but nearly empty.** `MindGenome` carries no member id (`mind.rs:283-298`). The
identity is the `Arc` pointer against the lab's member list. That works, because a newborn shares its parent's `Arc`
(`mind.rs:1730-1736`), and a dead ant's mind survives up to `PRUNE_EVERY` = 600 frames (`mind.rs:1867-1872`). But the
30 members are near-clones (feasibility finding 2), and a founder's member is fixed by `id % 30`. Cut it from A4, or
keep it for Season 2, where "line" means something.

**S6. Frame cost is measured in the wrong place.** A6 quotes µs per tick, but the demo's cost is in the draw.
- Marks in the box are the one thing that forces the full redraw on the layered path (`lab/mod.rs:2316-2330`; the
  `Lab::draw_layered` doc).
- A HUD that changes every tick repaints the layer every frame.
- Camera follow moves the view every step.

Use `labdraw`, which exists for this (`Reports/instruments.md:142`), with follow on and a ring live. Quote it beside
`labperf` and the `ascii` worst frame.

**S7. A1 code issues** (none breaks the off path):
- (a) `choose` filters only on `Verb::acts()` (`steer.rs:373`), not the genome's switch (`mind.rs:2056`). F therefore
  lets a raider Feed, a verb `RAIDER_VERBS` does not have (`train/predator.rs:22`). The doc's "only a legal option ...
  nothing a mind of its kind could not" is false. Filter on `g[SWITCH + v] > 0.0` (pass the genome), or correct the
  doc.
- (b) The emits are still the mind's own (`mind.rs:2559`, laid at `2606`), computed from memory after the mind's verb.
  A led ant lays the scent its mind wanted, not scent that matches what it did. This matters for the scout seat's
  trails; say so in `shared/steer-mind.md`.
- (c) When `redrawn` is set, the memory is left from the redraw's verb (`mind.rs:2536`). This happens only under
  SCRATCH vetoes; note it.
- (d) The shipped hook calls `scratch_log::remember` (`mind.rs:5426`). Move it out of the scratch module, or a scratch
  cleanup will break the player.
- (e) `scratch_log::decision` and `copy_decision` log the mind's pick before the override (`mind.rs:2351-2360`), so a
  trace of a led body records what it did not do.
- (f) Reversing a chain: with the body heading east and W held, a diagonal Walk within 90° (NW or SW) wins over
  TurnAround (`steer.rs:389`). The centipede will curl over its own neck rather than turn. Check it with a GIF, and
  consider preferring TurnAround when the tail lies within about 45° of the held direction.
- (g) No test shows a forced verb working in a world. Add one: a steered centipede beside an ant with B held shows
  `attacks` up, against a control without B. Bite is the demo's main verb, and the synthetic `choose` test cannot see
  `act`.
- (h) `MindCounts` (`chose`, `failed`) now include the player's decisions; a harness measuring the colony should know.

Determinism looks right. `take_forced` draws nothing, the `BTreeMap` is only point-read, and the mind's own draw is
keyed on (seed, organism, frame, slot) as before (`mind.rs:2347`, `2572`). The `Decision` built for `d == None` is
complete and read only for `pick` and `emits`. The climb fallback is limited to a level hold, and a fall skips the hook
without spending the latch.

**S8. The autopilot is more work than budgeted.** It has to navigate a crumb bowl with G and Q, and a pellet in the jaws
changes what G means. Drive it through `Steered::press` and `SteerInput`, the player's own path, not through
`steer::force`, so the proofs exercise the keys the owner uses. Add a stuck counter. The 1.5-2 day estimate for all of
A looks short by the autopilot alone.

**S9. Get the owner's verdict the repo's way.** Post a blind A/B with `review.py`: the autopilot's hunt against the
trained colony and against the hand-built one. Put kills, bites taken, the cell index lost and loads home in `meta`.
Do this before asking question 2, and before calling A3 "satisfying". Then ask the owner to play it. A GIF of a bot is
not the hand.

**S10. A0.2's alarm trace** is useful, but it only decides one line of HUD text. Run it after A0.3 (M5), which decides
the game.

## What the plan got right

- It builds the Raider, not the Pit, and states plainly on screen that the colony does not learn.
- It measures the box and the alarm before building.
- It keeps a no-raider control in Part B and has a rule against hiders.
- It picked up the `trip_deliveries` correction on its own.
- It defines the price per delivery rather than per kill.
- It drives headless proofs with an autopilot on paired seeds.
- The A1 hook is well made. Off is two reads (`steer.rs:337-350`), the identity check uses `born_frame`, a forced pick
  is always an option on the list, memory and pull follow the forced verb, and all three tests carry positive controls
  that were watched go red.

## Questions

1. **Surplus killing.** Should spitting a crop cell (Q) to bite again stay? It is the engine's physics, but it removes
   the hunger ceiling the loop is written around.
2. **Budding under the queen rule.** With budding on and `Steered::colony` left as `mark` sets it, the player's buds
   would be raiders that cannot breed while the player lives (`regime_of_colony` applies in `try_bud` at
   `creature.rs:5830`). Is that the graded middle the owner would prefer to "off"? It needs a measurement before it is
   offered.
3. **The hand-built colony mutates at birth** (feasibility finding 2). Over a long session against it, is that slow
   but real in-session selection worth a counter on the card, rather than saying "learns nothing" for both brains?
4. **When the colony falls,** does the demo stop with the card, refound the colony, or let the player hunt AI raiders?
5. **The steering cap.** Which dial setting is it in the launcher, and does the player see the hand-over to the
   raider's mind?
