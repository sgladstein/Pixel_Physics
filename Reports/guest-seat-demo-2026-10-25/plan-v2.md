# Guest seat demo: implementation plan v2 ("the beggar")

*2026-10-25, guest seat thread. Revises `plan-v1.md` after the independent review `plan-v1-review.md` (6 blocking,
7 improve, nits). v1's goal, ground rules and step order stand; this file states the plan as it will be built, and
the table at the end maps every review finding to its fix.*

## What the owner gets (unchanged in spirit)

`cargo run --release --bin lab -- demo=guest`: a lab box whose colony you watch grow at top speed for a few
seconds, then **you, an amber rove beetle the size of a worker**, land beside the door. You smell wrong; every ant
that touches you shows its verdict on its body (walks on; stops and holds on you with a `?`; flares with a `!` and
backs off in a puff of alarm; lunges and nips, and you flinch and lose a body cell). Rub workers and lie in the nest
to take its smell; beg the ants in the brood pile; keep the disguise up, because it fades; kill a larva when
starving and eat the corpse. A ledger counts what you took and grades your life: cuckoo, thief or predator.

## Scope changes from v1

- **Trained ants only** (the lab default, `bin/lab.rs:306`). The hand-built ant's mouth and fight are not gated; the
  help card says the demo runs the trained colony, and `demo=guest` forces ANT BRAIN: NEW. The ladder's interface
  does not change, so the keeper seat loses nothing. (Review 4, 11.)
- **The box is `steady_income`'s, not the played bed's.** Measured: on the played bed the rove sticks in the grass
  mat above the nest (head moved 1-11 cells in 480 frames, ASCII look); on `steady_income`'s bare surface it walks
  down the shaft into 1-cell galleries in 100 frames. `steady_income` grows ~35 trained ants by frame 30,000 at a
  fraction of the played bed's cost (played bed: 57 s wall for 30,000 frames on the 4-core session box).

## The mechanism, as built

### A. The ladder (`src/sim/guest.rs`, the shared piece)

- `World::visitors: Option<Visitors>`; `Visitors { list: Vec<Visitor>, marks: FxHashMap<OrganismId, Mark>, counts,
  events, params }`. `Visitor { id, home_scent: [f32; 3] }` (the scent it fades back to, set on landing). `Mark {
  rung, at_frame, hold_until, cooldown_until }` (held here, not on `OrganismState`; dead ids pruned each sweep).
- **Contact and roll** (`guest::meet`), called just above the mind branch in `creature_tick` for a mind ant of
  species `ant`, only when `visitors` is `Some`. It compares the ant's head 8-neighbourhood with each visitor's chain
  (a coordinate test; riders cannot hide it). **It marks; it never returns early.**
  - **One roll per contact episode**: on the first touch after >= 2 s without one, and never within the ant's
    per-ant cooldown (6 s at 1x).
  - `g = |scent(visitor) - scent(ant)| / tolerance_radius(ant)`, both from `expressed_traits` (as
    `is_living_kin_id` reads them).
  - `g' = g + min(0.25, 0.5 * alarm_here / ALARM_DEPOSIT) + pick(ant) + 0.2 * hunger(ant) + beg_penalty`;
    `pick(ant)` a stable per-id offset in [-0.2, 0.2] from a keyed stream (dice, not a gene, said on the help card).
  - Rungs from logistic steps (centres 0.5 / 0.9 / 1.3, slope 6). **Floors 0.03 on antennate and display only; no
    nip at all below `g' = 0.8`**, so a good disguise is never killed by chance and the middle is always present.
- **Effects through the mind, not around it.** A marked ant's next decisions are a **forced pick** inside
  `mind::tick`: after the fall check and `options::build_with`, while `guest::held(world, id)` is true, the option list
  is cut to the one `Rest` option (always pushed) before `decide_planned`. The ant's keyed draw, bills, memory,
  `last_verb`, `settle` and reschedule all run as for any Rest. Copies the pattern of the scratch teacher hooks
  (`scratch_log::Teach::Force`), not their code.
  - *antennate*: held 1.5 s (15 decisions); mark `?`.
  - *display*: held 1 s, then forced one Walk option away from the visitor (the option whose target is farthest
    from it), and `contest::display_deposit()` at the ant's own cell (the engine's display semantics); mark `!`.
  - *nip*: the ant's head lunges (a forced Walk toward the visitor if it can, else none), then `bite_cell(world,
    ant, visitor, cell, NIP)` on the visitor cell next to its head, preferring the tail; mark red; the guest
    flinches (its heading turns away). `bite_cell` is the done-branch of `mind::attack` factored into one helper
    shared by both bite sites, and it checks `world.get(cell).organism_id() == visitor` before removing anything.
    `NIP` = 0.34: three nips inside a heal window take a cell. A cell is a life, the head the last: the help card
    says so. Heal stays the engine's (0.25 per 750 frames), so nips are forgiven if you get away.
- **Gate**: in `options.rs` the Attack option is not pushed when the target is a registered visitor (one `Option`
  read when off). This is the trained mind's only way to harm the guest (review 4 checked PickUp, Dig, Feed).

### B. The guest's verbs (guest-only calls in `sim/guest.rs`; keys routed by the seat)

Rates are stated as time constants at 1x (60 frames/s, a decision every 6 frames) and derived per decision in code.

- **Rub** (R, a *press*, beside an ant): the guest's scent moves 0.10 of the gap toward the ant's; the ant's moves a
  quarter of that toward the guest's (so a worker cannot be pulled out of its colony by `regroup_by_scent`, review
  9); the ant rolls the ladder at half odds. One rub per ant per 2 s.
- **Soak** (P, held, beside brood or on a nest site): scent relaxes toward the nearest seeded nest odour with τ = 20 s;
  refused (and says so) if `g` against the nest is over 2.0. The nest's own uptake swamps the guest's gamma, so the
  nest is not pulled.
- **Fade**: always, toward `home_scent`, τ = 60 s. A disguise is upkeep.
- **Beg** (Y, beside an adult ant): first a donor check: can it give (bank above `0.7 x start_energy`)? If not, a
  grey `-` mark ("has nothing to give") and no roll. If it can, acceptance `p = clamp(1.2 - g', 0, 1)`; accepted: the
  donor gives 25% of its bank above the floor (bank to bank), `SharedOut` on its colony and `SharedIn` on the
  guest's, a droplet flash and `+`. Refused: the ant rolls the ladder with `beg_penalty` 0.3.
  **Economy, to be confirmed by the count in step 2**: the rove's idle cost is cut to 0.02 per cell (about 0.4 J/s
  at 1x) so one 5-25 J share buys 10-60 s; the larva corpse (its bank, ~100+ J) is the big, costly meal. If the
  count shows near-zero donors, the share also draws from the donor's crop at the donor's own yield (a regurgitated
  droplet, priced by the giver because the biology is the giver's food).
- **Kill a larva** (B, beside brood): `brood::eaten(world, larva, eater)`, a twin of the private `larva_starves`
  (corpse stamped with the bank as `StoredInMeat`, `free_brood`, `larvae_eaten` counter, alarm puff). No direct
  energy gain: you then eat the corpse with E.
- **Eat** (E): the steered Feed output, with the guest's ingest restricted to non-living cells (a one-check gate in
  `adjacent_food_counted` for a registered visitor), so E takes corpses and never a worker. Booked
  `HarvestedCorpse` by the existing code.
- **Jump** (J): `SteerInput::jump` writes Impulse = 1 **and** Move = 1 (Impulse is read only inside the move roll).
  Field agreed with the queen thread before adding it to `or()`.
- Steer's own bite (B in the queen's layer) is never set for the rove: the seat overrides B and E for this body.

### C. The colony number

The rove founds its own colony number (as any beetle), so the mind never offers it Feed (`options.rs:877`) and all
feeding goes through the beg. Measured: with the host's number the mind fed it 176-215 J unasked and never bit it.
A respawned rove rejoins the same guest colony and is re-marked (`World::steered` keeps the dead id).

## Steps and proofs

1. **The rove** (done in the working tree): species + material, held off the chip. *Proof:* lib tests `sim::organism
   sim::material sim::species_export sim::plant lab::ui`; `guestprobe body=rove`: walks down the `steady_income`
   shaft (done: 157 -> 170 in 96 frames); with no ladder it is dead within 600 frames of landing among trained
   ants (positive control, any cell count).
2. **Ladder core + the donor count.** `guest.rs`, the forced pick, `bite_cell`, the Attack gate. *Proofs:* (a)
   off-is-off hash test with a positive control; (b) unit tests: odds monotone in `g`, no nip below 0.8, the host's
   tolerance decides, Attack never offered on a visitor and still offered on a non-visitor stranger, `bite_cell`
   refuses a cell that is no longer the visitor's; each watched red; (c) probe `ladder=1`, guest standing in the
   pile, 6 seeds x g pinned at 0.2 / 1.0 / 2.0: nips per minute ~0 at 0.2, some at 1.0, many at 2.0, and
   antennations non-zero at all three (graded); (d) **the donor count**: adults touching brood with bank > 0.7 x
   start, and their crop worth, printed per seed. The beg's source is fixed from (d) before step 3.
3. **The verbs.** Rub, soak, fade, beg, kill-a-larva, eat-corpse, jump. *Proofs:* unit tests per verb (rub moves
   both sides at 1 : 0.25; soak refuses past the gate; fade returns to `home_scent` at τ; beg books SharedOut =
   SharedIn and the energy account closes; larva eaten leaves a corpse whose energy equals the bank, eaten once);
   scripted probe `script=1` over 6 seeds prints `g(t)`, nips per minute and feeds against the standing guest, and
   **`regroup` splits = 0**; a larva-eating arm shows the alarm and the nip cost.
4. **The demo box.** `assets/lab_scenarios/demo_guest.ron` from `steady_income`, trained ants forced on. The colony
   grows on screen at top speed behind a "colony growing" line until frame ~30,000 (measure wall time; target under
   15 s on a laptop; fall back to an earlier frame if not), then the rove lands 10 cells from the door and the clock
   drops to 1x. *Proof:* headless screenshot of the landing frame.
5. **The seat** (`src/lab/demo/guest.rs`, behind queen's `DemoSeat::Guest`; a same-named fallback reader if the
   launcher is not on main yet). Keys: arrows move; R rub, P soak, Y beg, B kill a larva, E eat, J jump, H help, Tab
   follow. **Marks on the 2x HUD layer** within the follow-cam view: `?` yellow, `!` orange, red flash, `+` green,
   `-` grey, ~1 s each. **The guest tinted by `g`** (full replace on a fixed red-amber-green ramp; creature colour
   keys on colony, so a tint must be drawn). The alarm puff shown without an overlay (verify; if invisible, draw
   it). Ledger: energy bar, cells left, lives, fed (times, J), refusals, nips, antennations, displays, larvae killed,
   colony adults and brood now vs on arrival, grade. Respawn after 3 s. *Proof:* screenshot/click hooks; a GIF of a
   scripted session.
6. **Tune by playing, then measure.** Targets: a raw guest is nipped within ~10-20 s of contact; three rubs and
   20-30 s of soaking make most contacts antennations; a disguised guest is fed more often than refused; a minute
   off upkeep returns you to "stranger". Hold lengths judged by eye (a blind A/B on the Artifact page). Measure:
   12-seed scripted probe; per-rung distributions raw vs disguised (both with a middle); colony cost
   (`trip_deliveries`, not `deliveries`, with and without the guest); frame cost (`ascii` worst frame, `labperf` on
   the demo box with and without the guest); `antcost par=on,off` once with a guest.
7. **Land.** Report `Reports/guest-seat-demo-2026-10-25.md` + `Reports/README.md` line + review/plan files;
   Artifact page with GIFs and counts; PR to the merge desk with off-is-off `deeptrace` identity; message keeper;
   memory.

## Review findings and their fixes

| # | Finding | Fix in v2 |
|---|---|---|
| 1 B | Early return skips the mind's whole body tick | Forced Rest pick inside `mind::tick`; `meet` only marks |
| 2 B | `gnawed` alone takes no cell; heal | Shared `bite_cell` helper with ownership check; NIP 0.34; engine heal kept |
| 3 B | Roll cadence x floor x alarm = lethal, binary | One roll per episode, 6 s cooldown, no nip below 0.8, display deposit, alarm term capped |
| 4 B | Old ant's mouth; guest's own E/B kill workers | Trained-only scope; guest ingest gated to non-living; B is the larva verb |
| 5 B | No path to eat a larva; double count | `brood::eaten` twin of `larva_starves`; eat the corpse with E |
| 6 B | No donors; carnivore gets 0 from crops; SharedIn | Donor count first; bank-to-bank to a 0.7 floor; SharedOut + SharedIn; grey `-`; lower burn |
| 7 I | Reactions too brief/small | 1-1.5 s holds, step-away display, lunge + flinch, HUD-layer marks, tint by `g`, A/B by eye |
| 8 I | Rates contradict targets | Time constants (fade 60 s, soak 20 s, rub per press) |
| 9 I | Regroup splits rubbed workers | Ant side at a quarter; count splits |
| 10 I | expressed_traits, home_scent, ALARM_MAX, hold_until, alarm repels | All adopted; help card says ants scatter from alarm |
| 11 I | Drop brain-agnostic | Done |
| 12 I | Jump needs Move; key collisions | Jump writes Move; seat overrides B/E; field agreed with queen |
| 13 I | Warm-up cost | Measured (played bed 57 s); `steady_income` box; growth shown on screen |
| 14 N | Test names; control; respawn colony; seeds; gamma; colour; worktree | Adopted. Worktree: this cloud container runs one session only, so the shared-checkout risk does not apply here |
