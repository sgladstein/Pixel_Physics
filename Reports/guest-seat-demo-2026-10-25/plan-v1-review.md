# Review of `guest/plan-v1.md` ("the beggar")

*2026-10-25, independent reviewer (did not write the plan or the feasibility review). Checked against `main`
9e32979e plus the uncommitted step-1 work already sitting in `/home/user/Pixel_Physics` (`assets/species/rove.ron`,
`assets/materials/rove.ron`, the two `EMBEDDED` appends, `NOT_STOCKABLE_IN_LAB`). No repo file was changed.*

## Verdict: right shape, not buildable as written. Six blocking corrections, all local.

The core idea holds: a scent-reading body rule that runs before the mind, on a worker-sized rove, with both bites
taken off a registered visitor. What fails is the plumbing and the arithmetic. (1) The early return skips the
mind's whole body tick. (2) The nip relies on wound arithmetic that does not exist. (3) The roll rate times the floor
kills a perfectly disguised guest in the brood pile. (4) Two more ways to bite the guest, and the guest's own
mouth, were missed. (5) "Eat a larva" has no existing path and its energy is counted twice. (6) The beg has, on the
evidence, nobody with anything to give. Fix these six and the plan can be built.

## Findings

### 1. BLOCKING: an early return before `mind::tick` skips the body, not just the decision
`mind::tick` (`creature/mind.rs:2234-2613`) is the mind ant's whole body tick, not only its choice. It covers:
- every standing bill booked to the ledger (`:2248-2288`: idle, brain, jaw, plate, exposure);
- `blend_with_nest` (`:2289-2296`);
- **the fall** (`:2300`);
- `counts.decisions`, `mind.state.mem`, `last_verb` and `last` (`:2519-2535`);
- `still_ticks`, `since_nest` and `since_trip` (`:2581-2589`);
- `settle` (`:2605`): digestion, starvation, birth, **and the returned `ActiveSite`, which is the reschedule**.

Returning early at `creature.rs:8022` for three decisions has four effects. A held ant on undermined ground
**hovers**. It thinks and stands for free. Its stillness input never rises. And the code must invent its own
reschedule. Charging "the idle bill as a Rest would" is one of five bills. The hand-built path has the same
problem: an early return there skips `act`, the walk and `settle`.

**Fix: one forced pick per brain, and no early return.**
- **Mind.** Inside `mind::tick`, after the fall check and `options::build_with` (`:2309`), when
  `guest::held(world, organism)` is true, cut the option list down to the `Rest` option (`Rest` is always pushed,
  `options.rs:819`) before `decide_planned`. The ant's own keyed draw (`RNG_SLOT_MIND`) is still taken, memory and
  `last_verb` stay consistent (verb = Rest), `act` returns true, and the bills and `settle` run unchanged. Strip
  Attack-on-visitor in the same place, or at the push (`options.rs:905`). That is the forced pick. The only
  existing ones are SCRATCH teacher hooks (`scratch_log::Teach::Force/Veto`, `:2471-2491`): copy their pattern, not
  their code.
- **Hand-built ant.** Do what steer hook 1 does (`creature.rs:8171-8183`): after the look-ahead merge, overwrite
  the outputs (Move, Attack, Feed and Tumble to 0) for a held ant, then let the tick run. On the serial thread this
  is safe under every `ParMode`.
- **Where to roll.** The contact check and roll can stay above the mind branch (`:7993`). They only *mark* the ant
  (`Visitors.marks`). They must not return.

### 2. BLOCKING: `gnawed += 0.25` removes nothing, and it heals between nips
`gnawed` is progress and nothing more. The *biter's* done-branch removes the cell: it sets `gnawed = 0`,
`world.set(t, EMPTY)`, `reconcile_chain`, tallies the kill and cries the alarm (`creature.rs:20700-20733`; the
mind's twin is `mind.rs:2680-2700`). Nothing reads `gnawed >= 1` on its own. A ladder that only increments it never
takes a cell. It also heals at `interval/3000` per tick (`creature.rs:7966-7972`, `WOUND_HEAL_FRAMES` 3000), so
one nip of 0.25 is gone in 750 frames (12.5 s). Four nips kill only if they land inside about 2,250 frames, and a
tail nipped first leaves a living one-cell rove (the head lost is death, per `reconcile_chain`'s note at
`:7903-7909`). "Four nips kill" is wrong both ways.

**Fix:**
- Factor the done-branch into one helper that both bite sites and the nip call: `bite_cell(world, biter, victim,
  (tx, ty), damage)`.
- **Check grid ownership before removing.** `chain` can be stale until the victim's own reconcile, so test
  `world.get(t).organism_id() == visitor`.
- Make the nip target the visitor cell next to the ant's head, and say on the help card that a cell is a life and
  the head is the last.
- Choose the heal for the guest deliberately. If nips should accumulate over a stay in the pile, slow the heal for
  the visitor only (one `Option` read), or count nips in the ledger as their own number.

### 3. BLOCKING: roll cadence x the floor x the alarm term makes the pile lethal, and the outcome binary
The arithmetic:
- **Roll rate.** An ant re-rolls every 30 frames while it stays in contact, so 2 rolls/s for each ant touching
  you.
- **Contacts in the pile.** In the brood pile the guest touches roughly 3-6 ants at once (the probe's `touch` of
  20-600 per 1,000 frames was at the *door*). That is 6-12 rolls/s.
- **Damage at the floor.** At the nip floor of 0.03 that is 0.18-0.36 nips/s, or 0.045-0.09 gnaw/s, against a heal
  of 0.02/s.
- **Result.** A guest with a *perfect* disguise loses a cell about every 15-40 s exactly where the beg loop happens.

Then each nip cries a full wound alarm, `ALARM_DEPOSIT` 240 (`creature.rs:11222`, `pheromone.rs:347`), which adds
up to +0.5 to every nearby `g'`. One nip feeds the next, and the run tips into either calm or a death spiral.
That is the "dissolve or nothing" defect, and it breaks CLAUDE.md's law 1.

**Fix:**
- **Roll once per contact episode**: the first touch after more than about 2 s apart, not every cooldown while
  touching.
- **Per-ant cooldown in seconds** (5-10 s).
- **No nip floor below a `g'` threshold.** Keep the floor on antennate and display, so the middle is always
  present and death is never random for a good disguise.
- **The display rung deposits `contest::display_deposit()` at the displaying ant's own cell**, the engine's
  existing display semantics (`creature.rs:20660-20680`), not `cry_alarm` at the visitor.
- **The nip's alarm term saturates**: cap the alarm contribution at about +0.25.
- **Proof addition.** Step 2's probe prints nips per minute for a guest pinned at `g = 0.2` in the pile. It must
  be about 0 over 6,000 frames.

### 4. BLOCKING: two more bite paths, and the guest's own mouth
- **The hand-built ant's mouth.** `act`'s ingest gnaws any non-kin living cell it scores as food, through the
  *whole body's* ring for an attached organism. The scan is `adjacent_food_counted` (`creature.rs:12163-12215`;
  the `is_living_kin` skip at `:12205` is the only guard). The gnaw site is `:20945-20993` and cries an alarm. The
  rove is `kind: Creature` with `food_energy 480` and `food_class 1.0`, so the old ant eats it. The plan gates only
  `nearest_foe`. The oldant arm of the probe ("bites 5-11, the old fight") cannot tell the fight from the mouth.
  Gate visitor cells out of this scan as well (one `Option` read), or drop the brain-agnostic claim (finding 11).
- **The guest's mouth (E) kills workers.** `rove.ron` gives a carnivore gut (`traits.0 = 1.0`). Ant flesh is
  `food_class 1.0` and cuticle 0.25 (`materials/ant.ron:36-42`). The rove's bite (dig_force 1.0) against 0.25 is
  `bite_progress = 1.0` (`contest.rs:148`): **one press of E beside a worker takes a whole cell, 480 J into the
  crop**, with the alarm. B (`steer`'s bite → `nearest_foe`) does the same. That breaks the plan's "E does
  nothing" and "adults are never targets", and it makes begging pointless.
- **Fix.** The seat routes E and B itself:
  - E eats loose food only: corpses (as `rove.ron`'s comment intends).
  - B is the larva verb (finding 5), never `steer`'s bite on adults.
  - If killing workers is wanted, it is a deliberate fourth grade with its cost shown, not an accident of the gut.
- **Checked and harmless.** The mind's PickUp needs `organism_id == 0` (`options.rs:838`, push at `:854`). Dig refuses `Creature`
  cells (`jaw_can_cut`, `creature.rs:18818`). Mind Feed needs the same colony (`options.rs:877`). The mind's
  Attack is the only mind harm (`:905-907`). Spoil and soil falling on the guest is physics.

### 5. BLOCKING: "eat a larva" has no existing path, and the plan counts its energy twice
Brood is `kind: Powder` with no `food_energy` (`materials/brood.ron:14-20`). `is_animal_cell` rejects it
(`creature.rs:11255`), so neither `nearest_foe` nor the mind's `attack` can target it, and no mouth scores it. The
feasibility review's "its B bite took brood" is most likely a misread of `brood0 -> N`, which also falls through
hatching and starvation. The plan also says "kills it to a corpse cell ... **gains its energy**". A larva's bank
goes into the corpse as `StoredInMeat` (`brood.rs:1512-1531`), so taking it as well counts it twice.

**Fix.** Add `brood::eaten(world, larva, eater)`, a twin of the private `larva_starves`: corpse stamped with the
bank, `free_brood`, a new `larvae_eaten` counter, and the alarm puff. The guest then **eats the corpse with E**
through the existing ingest. Corpse is `food_class 1.0`, the carnivore yields 1.0, and it is booked
`HarvestedCorpse`. That is one ledger path and two visible beats: kill, then swallow.

### 6. BLOCKING until measured: the beg has, on the evidence, nobody to give
Three problems:
- **The crop route pays a carnivore nothing.** `crop_feed_from` prices the share by the *taker's* gut
  (`brood.rs:1204-1210`, `yield_of(world, larva, def, ...)`). Ant crops hold plant food (crumbs and provisions,
  `food_class -1`), and quality for gut 1.0 is `(1-1)^2 = 0` (`creature.rs:10820`). The guest gets 0.
- **The bank route needs a donor over its stamp**, as `nurse` does (`brood.rs:1089`). The dead-ends record for
  the same gate near the brood: "ants touching hungry deep larvae are at 0.79-0.81 of their grant, so nothing
  gives" (`Reports/dead-ends.md:1252`).
- **Half the booking is missing.** The plan books only `SharedOut`. `SharedIn` on the guest's colony is required
  (`world.rs:3628-3646`; the identity test is at `creature.rs:52404-52413`).

**Fix:**
- **Count before you build.** In step 2's probe, count over the pile how many adult ants touching brood have
  `energy > start_energy`, and what their crops hold.
- If donors are rare, give from the bank down to a floor (for example `0.7 x start_energy`, which the donor can
  show by slowing), not only above the stamp.
- Book `SharedOut` and `SharedIn`.
- Show "has nothing to give" (a grey `-`) apart from "refused" (which rolls the ladder). Otherwise refusals read
  as random.
- Also check the economy: one beg gives `0.25 x (B - 200)` J, about 10-25 J at the observed banks, against the
  rove's ~1 J/s burn (`rove.ron`: 0.05 x 2 cells per 6 frames). The player must beg successfully about every
  15 s. Decide whether that is the loop you want.

### 7. IMPROVE: the reactions are too brief and too small to read at 1x
- **A three-decision hold is 18 frames, 0.3 s.** A 2-cell chain cannot visibly "turn to face" anything, so the
  stop and a 3-pixel glyph carry the whole rung.
- **The glyphs are drawn at world resolution.** The lab box is 1 cell = 1 pixel under the 2x interface layer
  (`lab/mod.rs:2251-2262`). Over a crowded pile, 3-pixel `?`/`!` glyphs will merge.
- **No scent tint exists.** Creature colour keys on `(species, colony)` (`organism.rs:6729-6735`), so the design's
  "your tint shifts" exists only as the seat's ring.
- **Fix:**
  - Hold antennation for 1-2 s.
  - Make display physical: the ant steps away. Make the nip a lunge plus a one-frame flinch of the guest (a turn
    away, or the jump).
  - Draw marks on the 2x HUD layer (or only within the follow-cam radius).
  - Tint the guest by `g`, as a full replace on a fixed ramp.
  - Verify that the alarm puff is visible in demo mode without opening an overlay.
- Post a blind A/B card of hold lengths: the owner judges this by eye.

### 8. IMPROVE: the starting rates contradict the plan's own targets
Per decision (6 frames), soak 0.02 against fade 0.004:
- **Soaking reaches family in seconds.** The two rates settle at `fade/(soak+fade) = 17%` of the authored gap, with
  a time constant of about 42 decisions (**about 4 s**).
- **The disguise lapses in seconds.** Walking off, with the authored point at distance 2.0, `g` climbs from 0.33
  to the display centre (0.9) in about 10 s, and to the nip centre (1.3) in about 22 s. That is not "about a
  minute".

**Fix:** state each rate as a time constant in seconds at 1x (for example fade τ = 60 s, soak τ = 20 s, rub
0.1 of the gap per *press* rather than per held decision), derive the per-decision value in code, and print `g(t)`
in the scripted probe.

### 9. IMPROVE: three rubbed workers can be relabelled a new colony and attacked
`Lab::tick` runs `World::regroup_by_scent` every tick (`lab/mod.rs:1976`). It splits any mutually kin cluster of at
least `MIN_SPLIT_GROUP` = **3** (`world.rs:739`) off its colony's hub. Rub is symmetric at 0.04 a decision. With R
held on one worker (10 decisions/s), the pair converges toward its midpoint within seconds, so a worker can sit
about 1.0 from its colony. Step 3's script "rubs three workers". The trained mind sorts by colony *number*
(`options.rs:877`), so three relabelled sisters become STRANGERs and get bitten. The nest re-blend usually rescues
them first, but not deep in a gallery.

**Fix:** move the ant's side at a quarter of the guest's step, and count `regroup` splits during the scripted
probe (it must be 0, or shown as a deliberate consequence).

### 10. IMPROVE: smaller correctness points in the ladder
- **Read the host's scent and tolerance from `expressed_traits`**, as `is_living_kin_id` and `kill_side` do
  (`creature.rs:11829-11833`, `:11302-11307`), not raw `traits`. Identical today for unwired lines, and it stays
  identical when they are wired.
- **Fade toward the scent set on landing**, not the species' (0.8, 0.8, 0.8). `rove.ron`'s comment says the demo
  resets scent on landing. Store `home_scent` in `Visitor`. Pick the landing direction toward the farthest corner
  of `[-1,1]^3` and check the clamped distance.
- **`ALARM_MAX` does not exist.** The alarm plane is `u16 x SCALE` and a deposit is `240 * SCALE`. Normalise by
  `ALARM_DEPOSIT`.
- **`hold_until` does not exist on `OrganismState`.** Keep it in `Visitors.marks`. A new state field touches every
  constructor and clone. `OrganismId` carries a generation (`world.rs:8964`), so an id-keyed map does not alias a
  reborn slot. Prune dead ids anyway.
- **Alarm is out of training for the trained minds.** A scratch note says `c::D_ALARM` reads 0 on every bed
  decision (`mind.rs:2329-2332`), and keeper and enemy measured ants *avoiding* a puff. After a larva is eaten,
  expect ants to scatter, not converge. Write the help card to match.

### 11. IMPROVE: drop "brain-agnostic" from the demo's scope
The demo opens on trained ants (`bin/lab.rs:304-307`). Supporting `oldant` needs two more hooks (finding 1) and a
third gate (finding 4), and every proof run twice. Build trained-only and say so. The ladder's interface does not
change, so the keeper seat loses nothing. Also, every probe must pass `brain=std`: `Lab::new` harnesses default to
the hand-built ant, and `guestprobe` defaults to `brain=old` (its doc's "the shipped default" is stale).

### 12. IMPROVE: Jump and the key layer
- **Jump needs Move.** `Impulse` is read only inside the move roll (`creature.rs:8470-8483`), and `p_move` is the
  Move output clamped (`:8374`). `steered_outputs` sets Move from the arrow alone (`:27731`), so J without an arrow
  never fires. J must also write Move = 1. The launch goes along `heading`, and `launch` refuses an unsupported
  body (`:28670-28680`).
- **`SteerInput` is shared.** It is `steer.rs:65`, inside the queen's piece: add `jump` to `or()` and agree the
  field with the queen thread first.
- **Keys collide.** In the queen's layer B is the slice-0 bite and E is eat (`shared/steered-ant.md`). The seat
  must override both for the rove (finding 4) rather than stack a second B.

### 13. IMPROVE: measure the warm-up now, not in step 4
There is no world snapshot facility, and `Scenario` has no warm-up (`lab/scenario.rs:58-85`). 30,000 frames at
about 3.7 µs/ant/tick plus the base frame (`creature.rs` round-33 note) is likely 30-60 s, against the 15 s
target. Every seat shares this cost.

**Fix:** time `guestprobe grow=30000 brain=std` in step 1 and tell the queen thread. Two fallbacks:
- Show the growth: run the lab at its top speed on screen with the guest held off, which is good to watch.
- Start on a younger nest.

### 14. NIT
- **Name the right tests for step 1's registry check.** They are lib tests, not `tests/*.rs`:
  `cargo test --release --lib sim::organism sim::material sim::species_export sim::plant lab::ui`. Integration
  holds only `determinism` and `worldgen`.
- **Loosen step 1's positive control.** "Killed exactly as the beetle was" cannot hold: the rove's cuticle is 0.25
  against the beetle's 1.5, so a mind bite takes a whole cell. Assert "dead within N frames" instead.
- **Give a respawned rove the same colony number** (`plant_creature_seed_in(.., Some(col))`), or each life adds a
  colony to the lab's lines. Re-`steer::mark` it: `World::steered` keeps the dead id (`steer.rs` header).
- **Seed counts.** Six seeds is a probe, not a guard. Step 6's twelve is right.
- **The soak's `gamma` pulls the colony's nest odour.** The colony's own uptake swamps it; say so.
- **The plan says "dark glossy" but `rove.ron` is amber-red.** The file is right (a player must find their body);
  fix the plan.
- **Work in a worktree.** The step-1 files are uncommitted in the shared main checkout, not a worktree. CLAUDE.md
  asks for a worktree, and a shared `target/` breaks other sessions' builds.

## What is right and should stay
- **The core design**: a scent-reading body rule beside the mind (the design's route (c)), with the mind's Attack
  stripped at the option list (`options.rs:905-907`). One place, and `options::shadow` sees the same list.
- **A new worker-sized `rove`** with the ant's `Chain(2)` and walk. **Leaving `nest:` unauthored** keeps
  `adjacent_nest` false (`creature.rs:19707`), so the existing `blend_with_nest` never runs on the guest. Keep it
  that way: authored, it would blend the guest at beta 0.1 a tick once inside its own radius and make soak
  pointless.
- **No species-gate change.** No mind is given to the rove (`lab/mod.rs:1090`), and it is kept off the chip
  (`NOT_STOCKABLE_IN_LAB`).
- **Determinism and parallel safety come free.** Rolls on keyed `rng::stream`s re-phase nobody.
  `World::organism_mut` marks the write-watch (`world.rs:8968`), so `ParMode::Checked` stays sound. Still run
  `antcost par=on,off` once with a guest.
- **The contact check costs almost nothing and riders cannot fool it.** It compares the ticking ant's own head with
  the visitor's chain, which is a coordinate test, so riders (stack cap 4) cannot hide a contact. Off, it is one
  `Option` read per ant tick.
- **Fade as upkeep, the per-ant "dice not a gene" said aloud, and the off-is-off hash test with a positive control.**
- **The fallback for the queen's launcher is sensible.** Steps 1-3 need no launcher at all; only step 5 does. Keep
  the fallback behind the exact names in `shared/launcher.md` so the rebase is mechanical.
