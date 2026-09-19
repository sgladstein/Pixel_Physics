# Stacking: many creatures of one colony in one cell

*Design of record, 2026-09-17. Owner's brief, and his rulings throughout §3.
Built and landed as PR #465 (merge `c061a245`), off by default; §10 is the
review that followed and §11 closes its four follow-ups. Supersedes nothing —
it is the first design on this axis — but it is the third attempt on colony
traffic and §1 says what the other two were.*

## 1. The complaint, and why the two existing answers do not reach it

The owner's brief: *"Right now, two ants/creatures cannot occupy the same
space. This can create traffic jams or affect behavior especially pheromone
following as a lot of this is happening on a 1D line across the ground.
Brainstorm some ideas for how multiple ants/creatures could occupy the same
space, not just two."*

The last clause is the specification. Two prior mechanisms already address
colony traffic and neither produces depth:

- **Spacing.** `COLONY_ANT_SPACING = 4` (dead ends 1017/1074) fixed the
  founding gridlock by moving founders apart — 27,386 blocked ticks and an
  unbroken wall of ants on screen. It works by giving ants *more room*, not by
  letting them share any.
- **Climb-over.** `CreatureDef::climbs_over_kin`, the deliberate re-test of
  dead ends 775/829, makes a living nestmate count as **ground**. Its own doc
  states the asymmetry as the design: *"Footing only, never passability, and
  the asymmetry is the whole design: a creature cell stays something you
  cannot enter, so two chains never swap through each other."*
- **Pass-through**, the remaining reading `dead-ends.md:1876` names as
  untested, is live on `claude/vibrant-mayer-9o2dbx` (`5a180d99`, *"ants may
  trade places with a nestmate -- the pass-through half of 775/829"*). The
  owner's ruling on it: it *"only works for two ants going opposite
  directions."* That is correct by construction — a swap is a permutation of
  two occupied cells, so it cannot express three-deep and does nothing for
  same-direction traffic, which is the nest-door and trunk-trail case.

**So this is the first attempt that changes occupancy rather than routing.**

## 2. The constraint: exclusivity belongs to the grid, not to creatures

`Cell` carries exactly one `organism_id` (`src/sim/cell.rs:272`), and
`classify_step` reads a cell held by an ant exactly the way it reads rock —
`BlockedWhy::HeadSolid` either way, because the predicate asks *can I stand
here* and not *what is stopping me* (`boxed_by_traffic`'s own doc says so).
Nothing in `creature.rs` forbids co-occupancy; the array does.

So the entire design reduces to one question: **where does the 2nd..Nth ant's
identity live?** There are four answers, and the engine already ships
machinery for two of them.

It is also worth naming that today's rule fails both of this repo's stated
laws at exactly the spot the complaint comes from. Occupancy is **binary** —
solid or empty, with no middle — and an ant meeting an ant has **no verb**: it
re-rolls its heading through `tumble` and walks away.

### 2a. The four candidates, and the one taken

| where identity lives | max N | new storage | blast radius | addressable? |
|---|---|---|---|---|
| in a **delay** (`organism::Crossing`) | unbounded | none, ships today | ~zero | **no** |
| in a **side table** (crowd id in `Cell`) | unbounded | table + id namespace | **292 reads** | yes |
| in a **count** (`aux`, the liquid model) | unbounded | `aux` | medium | no — identity dropped |
| in a **new axis** (depth register) | k | table + surfacing rule | 292 + more | partly |

**`Crossing` is the nearest existing thing and is transit-only.** It is the
owner's own design from 2026-09-06: *"if an ant tries to go through a trunk,
they basically just teleport to the other side with a delay long enough for
however thick the trunk is, so they don't actually overlap with the cells
ever."* An ant mid-crossing has left the grid, so occupancy is **zero** rather
than N and any number may be crossing one cell at once. Its doc names why the
shape is right: *"A real ant goes round the trunk, in the axis this world does
not have; the time that takes is the axis, expressed as a delay. And it is
graded by construction."* What it has no notion of is **dwelling** — ants at a
nest door are milling, not passing through, and a crossing that ends must put
the ant somewhere. It is kept in reserve as the "squeeze past" verb for
through-traffic, not as the answer here.

**A crowd handle inside `Cell` is the expensive one, and the cost is not
speed.** Two things that are easy to assume and are false. `organism_id` is
*already* a handle into a side table — `World::organisms: Vec<OrganismSlot>`
(`world.rs:3093`), a `u32` split 20 bits slot index / 12 bits generation
(`world.rs:64-81`), resolved by `decode_organism_id` → `Vec::get` →
generation compare (`world.rs:114`). An array index, not a hash. And creature
ticks are **serial** — `scheduler::step(world: &mut World)` →
`creature_tick(world: &mut World, …)`, a signature that cannot be held twice —
so there is no parallel contention to design around; only the CA cell sweep is
the four-pass checkerboard, and it already defers organism bookkeeping through
`ChunkOutcome::organism_moves` → `reindex_organism_cell`.

What it actually costs is **correctness surface**. `organism_id()` has **366
call sites across 18 files**, and only **74** are in `creature.rs`. The
remainder: `plant.rs` 158, `structural.rs` 25, `world.rs` 15, `rigid.rs` 12,
`player.rs` 12, `render.rs` 11, `load.rs` 7, `update.rs` 7. Every one reads
that field meaning *the organism in this cell*. A reserved crowd-id range
enrols all 292 non-creature reads in a concept they have never heard of —
plant anchoring, structural collapse, rigid fragments, the player's strike, the
renderer. That is `CLAUDE.md`'s named recurring failure — *adding a member to a
set something sweeps enrols it in every rule over that set, silently* — whose
two scalps are `spoil` breaking five censuses and the `druid` preset holding
`main` red for over two hours.

**The count model is wrong for this repo specifically**, which is a stronger
objection than its cost. `Cell::aux` already carries continuous fill for
`Liquid` (`cell.rs:247`), so sub-cell occupancy is a shipped idea. But
identity is dropped while merged, and the lab's entire purpose is
per-individual genomes and lineage depth; cargo and genome would need parking
somewhere, which reintroduces the side table anyway.

### 2b. The answer taken: riders, on the side table that already exists

The grid is left **exactly as it is** — one cell, one `organism_id`, unchanged
meaning. That cell's organism is the one that is visible and interactive.
Everyone else in the cell is a **rider**, never written into `Cell` at all,
tracked in a separate sparse index that no existing reader consults. Option
two's generality at option one's cost, bought by refusing to put the crowd in
the cell.

`dead-ends.md` was grepped for rider and stacking mechanisms before this was
proposed; there is no entry. (Silence is not evidence, per that file's own
caveat, but nothing has been tried here.)

### 2c. Co-occupancy is a property of a cell, not of an organism

The first draft of this design put `rides: Option<OrganismId>` on
`OrganismState`. **That cannot express the real case.** Bodies are multi-cell
and differ in length — `ant.ron` and `ancestor.ron` are `Chain(2)`,
`ant_long.ron` is `Chain(6)`, `longant.ron` is `Segmented`, and
`ant_block`/`ant_wide`/`beetle`/`hopper`/`chitin_pale` are `Rigid` blocks
(`ant_wide` is 5x2). A 6-cell body moving into a space where a 1-cell ant
already sits would own five of its cells and **ride at the sixth**, so one
field per organism is structurally insufficient.

Three consequences, all of which simplify rather than complicate:

- **No size rule.** Ownership is per-cell and first-come. There is no
  "largest ant on top" comparison, and no re-shuffle — promoting a big body to
  owner would mean rewriting six cells and demoting whoever held them.
- **One body may carry several unrelated riders** at different cells. They are
  not riding *the ant*, they are standing in *those cells*.
- **"The top creature" is a per-cell answer**, which is what makes the attack
  ruling in §3 fall out with no special case written.

### 2d. The engine already separates "my body" from "the grid's opinion"

This is the finding that makes the whole design cheap, and it was nearly
missed. `OrganismState.cells: PosMap<OrganismCell>` (`organism.rs:5230`) is
each organism's **own authoritative body map**, and `reconcile_chain` — the
death detector — validates the chain against **that map**, not against the
grid (`creature.rs:708`):

    let surviving: Vec<(i32,i32)> = chain.iter().copied()
        .filter(|p| owned.contains_key(p)).collect();   // owned = state.cells

So a rider whose body record is intact is **alive by the engine's own test**,
whatever the grid says about that cell. `reindex_organism_cell`
(`world.rs:8617`) confirms it from the other side: it only touches the two
organisms named in a transition, so a third organism co-located at that cell is
never disturbed when ownership changes hands.

An earlier draft of this design warned of a **death cascade** — one ant dying
and taking nineteen riders with it. That does not exist. The risk is the
opposite one, and it is worse; see §4.

## 3. Owner's rulings

Each of these was given directly and settles its question.

| | |
|---|---|
| **Independence** | *"Independent co-located. Every ant thinks for itself; the only thing shared is the cell, with dismounting as an ordinary move."* — agreed as the model. A suspended-passenger reading was rejected. |
| **Cost** | **None.** *"No cost for now. I am ok with a limit. Only 20 creatures can share a cell (not sure if 20 is the right number, we can play around with it). If this becomes a problem, we can revert or add a cost in the future."* |
| **Scope** | **Same colony only.** *"there is no ant hiding under a beetle."* `OrganismState.colony` already exists (`creature.rs:1120`). |
| **Attack** | Hits the **cell owner** — *"you don't want one attack hitting 20 creatures at once."* All stacked ants may attack outward, in tick order, and a dead target stops later attackers. |
| **Fire** | A **cell** property, so it hits everyone in the cell — agreed, and deliberately unlike attack. |
| **Involuntary exit** | A rider is **promoted** into the cell before it is overwritten. |
| **Corpse** | First free neighbour; if there are none, **suppressed**. |
| **Pheromone** | A stack lays up to 20x the trail. Accepted. |

**On the cap.** With a cap and no cost, the stack will sit **at** the cap
wherever there is pressure — the cap is the operating point, not a safety
valve. So 20 is "how deep should a pile look", not "how deep will it usually
get", and it wants picking by eye on a rendered jam.

## 4. The failure mode to design against: freezing, not dying

**A rider does not own its head cell, and the scheduler site sits on the
head.** `creature_tick`'s opening guard (`creature.rs:3779`) reads the grid
directly and, on a mismatch, reconciles and returns **no site at all**. Its own
comment: *"The creature would then never tick again: not dead, not scheduled,
just an orphan standing in the world forever."*

**This has already been paid for once, on the pass-through branch**, and the
account is `5824fd1d` — *"the kin swap orphaned the displaced ant's scheduler
site"*. Its measurement, `ticks` per 1,000 ant-frames against the 167 a 6-frame
`tick_interval` implies:

| arm | off | on (before fix) | on (after) |
|---|---|---|---|
| hand | 172 | — | 172 |
| self | 172 | **42** | 170 |
| mute | 173 | **24** | 171 |

The reason this matters more than a cascade would: **a frozen ant is never
charged, so it never starves, and a frozen colony reads as a thriving one.**
That branch's whole 12-seed sweep was the bug rather than a result — colonies
appearing to survive 24,000 frames on zero food. It is `CLAUDE.md`'s *"a cost
that vanishes may be work that vanished"*, where the vanished work was the
animal's entire existence, and the tell was arithmetic plus tidiness: uniform
12/12 survival across nine arm/gap combinations.

Two things carry over. The mover reschedules itself naturally, because
`creature_tick` builds its next site from the live head *after* the step; it is
the animal that did not act that goes unscheduled. And a stale site left to
evaporate is safe — when it fires, the guard finds a stranger's cell,
`reconcile_chain` reads the **chain**, finds the body intact, and the site is
dropped with nothing else changed.

**So a per-ant tick count is a required instrument here, not an optional one.**
Death is visible; freezing looks like success.

## 5. The build

Steps 1–3 are the pass this report is written for. Steps 4–6 are specified so
the next session does not re-derive them.

**1. Sparse occupancy index.** A new `World` field: a sparse map of cells
holding more than one creature, reusing `sim::fxhash::PosMap<V>`
(`fxhash.rs:116`) — the structure `state.cells` already uses. Value is a
`Vec<OrganismId>` in **insertion order**, which is deterministic because
creature ticks are serial. **Never iterate the `PosMap` itself in an
order-dependent way**: it is an `FxHashMap`, key lookups are fine, iteration
order is not a guarantee. Maintained for **riders only** — cell owners stay on
the existing `World::set` → `reindex_organism_cell` seam, untouched.

*Not a world-sized plane.* The shipped world is **8192x2560**
(`organism.rs:5217`), so a `u8` depth plane is ~21 MB of new allocation for a
feature live in a handful of cells, and M10 streaming would then have to manage
it.

**2. The cap check, and the toggle.** `classify_step` (`creature.rs:11281`) is
the single chokepoint — six live call sites. Where an occupied cell returns
`HeadSolid` today, a living same-colony occupant under the cap becomes legal.

**The toggle is the cap, not a branch.** At `cap = 1`, *may I enter a cell
holding one creature* **is** *is this cell occupied* — today's rule exactly. Old
behaviour is a **value of the parameter**, so there is no forked logic to keep
in sync, which is the usual reason a toggle rots.
`PIXEL_PHYSICS_STACK_DEPTH`, default 1, set to 20 to arm. This is the house
convention — **128 distinct `PIXEL_PHYSICS_*` switches** already exist in
`src/` — and the reason is on the record across `dead-ends.md`: keep the
ablation live in one binary, because a recompile sitting between two arms
becomes the thing that actually changed.

*The cap costs one slot per step, not one per body cell.* `classify_step` skips
cells already in the body's own chain (`chain.contains(&p)`), and a
forward-stepping `Chain(6)` re-enters five of its own six cells. A flip
(`ReverseRule::Flip`) costs **zero**, since no cell moves. Only rigid bodies
translating and births into a crowd pay more than one.

**3. Rider body records, and tolerating non-ownership.** Two surgical changes,
and everything else sits on them.

  a. **Populate a rider's `cells` map without a grid write.** Today it is only
     ever filled by `reindex_organism_cell`, driven from `World::set`. Left
     alone a rider's `cells` is empty, `reconcile_chain` finds zero surviving
     cells, and the ant dies — not because the grid disowned it, but because
     nothing told it that it has a body.
  b. **`creature_tick`'s opening check must tolerate legitimate
     non-ownership** while still catching a genuinely decapitated ant, **and
     must still return a site.** See §4: this is where the freeze lives.

**4. Promotion and the corpse rule.** On involuntary exit — death, fire, player
strike, explosion, the cell dug out, `Flight` launch — promote the **oldest**
rider from the per-cell list before the cell is overwritten.
`stamp_as_corpse` (`creature.rs:903`) is the single chokepoint for corpse
writing, shared by death and severing, so the placement rule lands in one
function: first free neighbour in fixed `NEIGHBOURS_8` order, else suppress.
**Count suppressed corpses and their summed `worth`** — a corpse carries what
the animal was made of, suppression deletes that from the colony ledger, and it
only happens in the crowded case, which is the case under study. An uncounted
leak correlated with the experimental condition is a measurement trap.

**5. Multi-pip render — CANCELLED, 2026-09-17, on a measured finding rather
than a change of mind.**

`MagnifyStyle`'s own doc says the in-cell art mechanism *"costs nothing at
`zoom == 1`, which is every ordinary frame"* — because at play zoom one cell
**is** one screen pixel. There is no sub-cell space to put a pip in, in the
frames the complaint was about.

And colour cannot substitute for it.
[`creature-appearance-design.md`](creature-appearance-design.md) measured that
directly: **"Extent is the only lever, and it has to roughly quadruple."** The
shipped dark ant already achieves the best contrast of the three values tested;
a pale 9-cell body puts *less* luminance on screen than the dark 2-cell one
(251 against 285); and shape at constant extent moves nothing measurable (two
9-cell bodies score within 0.8% of each other on every appearance number). A
stack has **no extra extent by definition** — that is the whole feature — so it
is precisely the case that report says cannot be made findable.

This is `CLAUDE.md`'s *ask which pixels a lever moves before ranking it by
silhouette*, applied before the work instead of after: three plant levers were
built, demonstrably fired, and moved nothing, because they only relabelled
cells. A pip inside a one-pixel cell has no pixels to move.

**So the payoff is judged as flow rather than as visible piles** (owner's
ruling): the point of stacking is ants *moving* where they were stuck, which is
what the complaint was about. The verdict card is a **motion A/B** —
`filmstrip gif=1`, cap 1 against cap 20 on a deliberately jammed scene — and
the measured claim is the flow numbers from `forage_probe spacing=2`, whose
`blocked/moves` ratio is the instrument that settled `climbs_over_kin`.

One precedent makes the jam mandatory rather than optional: on the
`climbs_over_kin` cards, **only the jammed-crowd card divided the arms for the
owner's eye.** Three others came back *"pretty similar"* and *"no major
difference"*, because at `COLONY_ANT_SPACING` there is no jam to dissolve. A
founded-colony card would answer nothing here either.

**6. Teach `neediest_kin` about riders.** **Open, and on the list at the
owner's request, 2026-09-18.** `Share` reaches through
`neediest_kin` (`creature.rs:6070`), which scans `NEIGHBOURS_8` around the
body's own cells and reads `cell.organism_id()`. **Two independent reasons it
cannot see a rider**: `NEIGHBOURS_8` never includes `(0,0)`, and the cell
returns the *owner's* id. So trophallaxis inside a stack would silently never
happen, in exactly the place it is most wanted. Leave `nearest_foe` alone — the
same blindness is what implements the attack-hits-the-top ruling for free.

## 6. Verification

**First in, before any behaviour rides on the toggle:** at `cap = 1` the sim
must be **bit-identical** to the trunk over a fixed run. Determinism is
required here (`PLAN.md`), so this is a checkable assertion rather than a
judgement, and it is the specificity control `CLAUDE.md` asks for. Not
bit-identical at cap 1 means something leaked, and that is the first bug.

- **Sensitivity, not only specificity** — put the fault back. A scene where
  ants demonstrably stack, with the depth counter reading zero at cap 1 and
  non-zero at cap 20. A counter that cannot move is blind, not strong.
- **`ticks` per 1,000 ant-frames**, per §4, against what `tick_interval`
  implies. This is the one that catches the silent failure.
- **Counters beside every picture**: depth histogram, **depth by body size**
  (so the `Chain(2)` / `Chain(6)` / `Rigid` asymmetry is visible rather than
  discovered), promotions fired, corpses suppressed and their worth.
- `cargo test` — **not `--lib`**, which cannot reach `tests/*.rs` where the
  preset and worldgen guards live.
- `cargo clippy --all-targets --release --locked -- -D warnings`.
- `bash scripts/acceptance.sh`, `bash scripts/docscheck.sh`.
- `cargo run --release --example ascii` for worst-frame timing — whole-frame
  figure, paired and alternating, baseline re-measured in the same session.

**The card is a motion A/B, not a still**, and there is no render work at all —
§5 step 5 has the measured reason. A blind pair of gifs, cap 1 against cap 20 on
a jammed scene, with the discrete counts in the card's `meta`. A contact sheet
cannot answer it: the renderer draws only the cell owner, so an armed stack shows
**fewer** visible ants than cap 1 and a still would read as a jam thinning out.
What changes is whether the ants are *moving*, and only motion shows that.

## 7. Known consequences, carried rather than solved

- **A falling stack drains one ant per tick.** Promotion makes it *valid* — the
  owner falls, a rider is promoted, repeat — but twenty ants then hang in the
  air for twenty ticks. The alternative is riders falling with the carrier as a
  group. Settle it with a rendered clip in the review queue, not in the
  abstract.
- **Partial-overlap invulnerability.** A body owning some cells and riding at
  others is attackable where it owns and unreachable where it rides. "You hit
  what's exposed" reads correctly, but it will surface as *why won't my strike
  land* rather than as a failing test.
- **Two feedback loops, both accepted.** A stack lays up to 20x pheromone, and
  corpses placed in neighbouring cells are food that recruits more ants to the
  jam. Watch them; do not pre-emptively tune them.
- **Long bodies pay slightly more to join a crowd** — rigid bodies translating
  need two slots rather than one, and a birth needs the whole footprint. An
  earlier draft of this design claimed a body needed *all* its cells under cap
  and therefore that long ants would be excluded first; that was wrong, for the
  reason in §5 step 2. It is now a counter worth watching rather than an
  expected defect.

## 8. Relationship to the dead-end register

- **775/829, 1017/1074** (shoulder-to-shoulder gridlock, 27,386 blocked ticks).
  The condition line reads *"re-test if creatures gain pass-through or
  climb-over."* Climb-over was the 2026-08-23 re-test; this is the other half,
  and it needs writing back once built.
- **1876** (*"let a body boxed only by kin step onto a kin cell as footing"*)
  names passability as *"the remaining reading of the candidate"*, ruled out in
  `climbs_over_kin`'s doc as *"a different and much harder change (two chains
  swapping through each other)"*. Stacking is not that change — it never swaps
  two chains through each other, it lets both stand.
- `python3 scripts/deadendindex.py --touching` before the PR.

## 9. How to play it, and why the menu looked empty

**Asked by the owner, 2026-09-18: "I don't understand how to playtest this. I
did not see an option for this in the evolution lab menu."** They were right,
and the absence had three separate causes rather than one. All three are worth
recording because each was a design decision made for a good reason that
compounded into a feature nobody could reach.

1. **The cap is a launch-time environment variable**, `PIXEL_PHYSICS_STACK_
   DEPTH`, read once when a `World` is made. That was deliberate — §3 records
   the owner's ruling that the whole thing sit behind a toggle, and an env var
   is the shape this repo's other ablation switches ship in (`PIXEL_PHYSICS_
   LAB_ROOM`, `spoil_kept`, `trophallaxis_enabled`). But an ablation switch is
   for measuring an arm, and the owner asked to *play with* the number — "not
   sure if 20 is the right number, we can play around with it" — which by the
   lab panel's own rule makes it a dial.
2. **There is nothing to see.** The render step was cancelled on measured
   grounds (`c10ea398`): a cell holding three ants draws as one ant. So even
   with the cap armed, a correctly working stack is invisible, and "I can just
   play test" has no artifact to land on.
3. **A founded colony at its shipped spacing does not crowd.** `COLONY_ANT_
   SPACING` is 4, and `HeadBlock`'s own doc says it out loud: *"a colony at
   `COLONY_ANT_SPACING` almost never has a nestmate in any of its own eight
   neighbour cells."* Measured here: a founded colony left alone produced
   **1,329 moves and zero stacks**. So the first thing a playtester would have
   done — arm it, press the colony key, watch — was guaranteed to show nothing
   even with (1) and (2) solved.

**(2) is the one that mattered most**, and it is why this section exists rather
than a one-line answer. `CLAUDE.md`'s rule is that *"did it fire at all" needs
a counter, not a picture*; with no picture available at all, the counter is not
a supplement to the playtest, it **is** the playtest.

### The recipe

```
PIXEL_PHYSICS_STACK_DEPTH=20 cargo run --release --bin lab
```

Then, because of (3), build a bed that actually crowds: on the **BOX** page set
`colony_ants` high (it reaches 120) and keep the bed narrow, then **REBUILD**.
Read the **SHARED CELLS** row in the stats panel.

- **`EVER`, not `NOW`.** A stack is transient by design — dismounting is an
  ordinary move — so the standing count is a snapshot of one frame. Measured
  over 2,000 ants: 15 cells standing against **498 events**, an understatement
  of about 33x. `NOW 0` with `EVER` in the hundreds is the healthy state.
- **Only a *founding* stacks, not a handful of animals placed one at a time.**
  `can_stack_into` refuses `colony == 0` (§3) — the bucket every plant and every
  directly-constructed test animal shares, because treating it as a colony is
  how an ant would come to shelter under a beetle — **and it refuses two
  different colonies just as firmly**, which is the half that matters to a
  player. The single-animal tool routes through `plant_creature_seed` with
  `colony: None`, so each animal calls `claim_colony` and founds a colony of its
  own: ten clicks is ten colonies of one, all strangers, and none of them will
  ever share a cell. Use the colony tool, which hands one label to the whole
  founding.

  (Recorded because the first write-up of this said *"`plant_ant` claims no
  label"*. It does claim one — `next_colony` starts at 1 and 0 is never issued —
  so the refusal was for strangeness, not for namelessness. Same outcome, and
  only the corrected reason tells a playtester what to do differently.)
- **Expect three, not twenty.** The cap has never been approached. The deepest
  ever observed is three creatures in a cell, at 2,000 ants.

### The dial, and why it is on the BOX page

`animals_per_cell`, on **BOX**, live on the next tick. It belongs on **ANTS**
beside `room_each_ant_wants` and `alarm_fades` — the two other world scalars
parked there because they are colony rules rather than properties of an animal —
and it cannot go there: **ANTS and GENOME both stand at exactly 20 rows**
against `no_page_is_longer_than_two_screens`' ceiling of 20, so either costs
relocating somebody else's row, in a file eleven unlanded branches are holding.
`71a81384` left a comment on the ANTS page saying the next lane would have to
move something; that is still true and this is not the change to spend it on.
BOX has seven free rows and already carries `colonies`, `colony_ants` and
`predators` — who is in the box and how many — which is the same kind of fact.

**Another lane reached the same wall hours earlier and stopped at it**, which is
worth recording because its conclusion reads more absolute than it is.
`f4594c45` (*"the home_bias row overflowed its page, and there is no page with
room"*) hit `no_page_is_longer_than_two_screens` twice — GENOME 21, then ANTS 21
— and concluded that the remedy was a new page and that this was "a lab-UI
decision rather than a measurement one". It considered those two pages. BOX was
not full, and it is a defensible home for a colony-crowding rule, so no page
split is needed for this one.

Its sequel, `3beb6e59`, records the owner declining a dial — **for `home_bias`
specifically**, which also had a substantive reason nobody should want one (above
0.25 every colony measured died). That is not a standing ruling against lab
dials, and it points the opposite way here: the owner asked for this number to
be playable (*"not sure if 20 is the right number, we can play around with it"*)
and then reported not being able to find it.

Two consequences worth knowing:

- **A REBUILD resets it** to whatever `PIXEL_PHYSICS_STACK_DEPTH` says, because
  it is a `Knob::Scalar` on the live `World` rather than part of the `LabBox`
  spec a rebuild is made from. `room_each_ant_wants` and `alarm_fades` behave
  the same way; the row's note says so.
- **A `Knob::Scalar` needs four sites, not one** — the row, `write`'s match
  arm, and `Dials`' field plus its `from_world`/`apply_to` pair — and the
  round-trip guard is hand-enumerated, so it is blind to whichever key was
  added last. It now names this one.

### The readout, which is the part that was actually missing

The **SHARED** figures ride the stats page's `LINES` row rather than getting a
row of their own, and that is forced rather than chosen: the page is sized to
its content and clamped to `bar_top() - 6`, which is **258**, and a bed with a
colony in it already comes to **exactly 258**. A row added there is not a row
nobody notices, it is a row that is **never drawn**, and
`the_page_stays_inside_its_own_border` passes straight through that because it
asserts the clamp rather than that the content fits.

**Which means the page already overflows by one row today, whenever the REFUSED
row fires** — 267 against 258, arithmetic from the measured 258 plus a text
row's 9 px. Not this lane's to fix, and the next lane wanting a row on that page
should know it has none.

## 10. Bug and efficiency review, 2026-09-18

Asked for by the owner after the scale run. Read as a diff of every non-comment
line the branch adds to `src/sim/world.rs` and `src/sim/creature.rs`, then
checked against the engine rather than against the diff — which is where all
three real findings came from, since none of them is visible in the added lines
alone.

### Fixed: a rider carried the wrong body forward

**The defect.** `relocate_chain` reads each cell's value off the grid before
moving it — `world.get(from)` — and **a rider owns no grid cell**, so for a body
that is already riding, that read returns *the host's* cell. A rider stepping
from one shared cell to another therefore stored somebody else's body as its
own. The damage lands later, at promotion, which writes the stored cell: the
grid's `organism_id` never changes, so `reindex_organism_cell` early-returns,
the promoted rider never becomes the owner, and it is left holding a
`state.cells` entry for a cell the world still attributes to the animal that
walked away. The rider index is the only place a rider's own appearance exists,
so it is now the authority.

**How it was found, and the part worth keeping: three wrong guesses first.** The
symptom was one assertion failure. I reasoned a cause, fixed it, re-ran, got
*the same message*, reasoned a second, same again, reasoned a third (`try_swap_
with_kin`'s missing symmetric guard), same again. Each hypothesis was plausible
and each was wrong about *this* failure. What ended it was tracing every rider,
`remove_rider` and `reindex` event at the one offending cell: **ant 18 mounts
`(70,119)` on host 19 at frame 756, is promoted at 774, and no reindex fires at
all** — which named the cause in one run and could not have been argued to.

**Two traps inside that, both already in `CLAUDE.md` and both walked into.**

- **A guard that goes red is not a guard that went red for your reason.** The
  first fix was "verified" by reverting it and watching the test fail with the
  expected message. It failed for something else entirely: the invariant was
  `cells.len() == chain.len()`, and that form also trips on a *pre-existing*
  defect unrelated to stacking — an animal left owning a grid cell no longer in
  its chain (measured: organism 1, chain `[(9,119),(9,118)]`, a third cell at
  `(5,117)` whose grid owner is **1 itself**, no riders; filed separately, and
  it reproduces at cap 1). A fault-injection that reads only the *message* and
  not *why* is the pass/fail-of-a-graded-quantity trap wearing different
  clothes.
- **So the invariant was rescoped to the question stacking actually answers**:
  no body may claim a cell the world disowns and it is not riding. That form is
  checked on **both** arms — it cannot fail at cap 1, where every `cells` entry
  arrives through the grid seam, so the unarmed arm passing is what makes it a
  control rather than an assertion.

**Two further fixes, kept although neither was the cause here.** Both are real
gaps found while reading, each an unpaired `remove_rider`:

- `relocate_chain`'s branch (1), a rider stepping off voluntarily, removed the
  registration and left the `state.cells` entry: no cell's `organism_id`
  changes, so the pruning seam never fires.
- `reconcile_chain`'s shortening path drops a position from the chain with
  nothing pruning `cells` when the position was *ridden* — it is in neither
  `attached` nor `severed`, so neither the corpse stamp nor the grid seam
  reaches it. Narrowed to grid-disowned positions only, so the severing and
  predation accounting above it sees exactly what it saw before.
- `try_swap_with_kin` guarded **the mover** against riding and had no
  counterpart for the animal being swapped *with*. The exchange clears both
  bodies to `Cell::EMPTY` before rewriting, so swapping with a rider deletes the
  third animal it was standing on — the exact erasure the mover's guard exists
  to prevent, reached from the other side. Also refuses when anything is riding
  on either body, which the clear would strand.

All four are no-ops at the shipped cap of 1, structurally rather than by luck: a
body owns every cell it stands in there, so the riding branch is unreachable and
`riders_at` is empty everywhere.

**What the fix did to the bed, and why it is the right sign.** The crowded
colony went from **1,273 moves and 24 alive** to **9,036 moves and 49 alive**
over the same 3,000 frames. That is a large change for a bookkeeping repair, and
it is the expected one: before it, a rider moving between shared cells wrote its
*host's* body into the destination, so bodies were being duplicated and
overwritten while the simulation went on believing everything was fine. Nothing
in the suite could see it, because every cell involved still held a plausible
live animal.

**And it broke this test's own "did it fire" assertion, which is the third
instrument lesson in one afternoon.** The assertion read
`stacked_cell_count() > 0` — the **standing** census, at the final frame — and
after the fix that reads 0 on a run that stacks *more*, because stacks are
momentary and disperse. This document already said to read `EVER` and not `NOW`,
and the lab readout already says it to the player; the test was written before
that was understood and nobody went back. It now asserts on
`CreatureStats::stacks_entered`, with the standing figure printed in the failure
message rather than gating it, and the unarmed arm asserts **both** are zero.

### Found, filed, not yet fixed — all four now closed in §11

- **A dying rider leaves no corpse and is not counted as suppressed.**
  `creature_dies` filters the chain to grid-owned cells, so a rider stamps
  nothing. The owner's ruling — *first free neighbour, else suppressed* — is
  implemented for the dying **owner** (via `stamp_as_corpse`'s promotion
  branch) and not for the dying **rider**. The ledger does close: the body
  stamp goes to `meat_lost` and the bank to `Account::Dissipated`, checked
  line by line rather than assumed. But meat that should be on the ground is
  not, and `corpses_suppressed` — the named-hole counter the owner asked for —
  reads 0 the whole time it happens, which is precisely the *unnamed leak
  correlated with the experimental arm* that counter exists to prevent.
- **Severing a ridden cell may overwrite the host.** Unverified by a
  reproduction, so filed rather than claimed. `reconcile_chain`'s `surviving`
  now keeps a ridden cell, so one can reach `severed` and then
  `stamp_as_corpse`, where `riders_at(cell).first()` can be the severing
  animal *itself* — and the promotion branch then writes that animal's cell
  over its host's. It is the one failure this design was built to prevent,
  arriving down a path nobody walked. Needs a multi-cell ant losing a segment
  while riding.

### Checked and found to be fine — recorded so nobody re-derives it

Two of these were on the review's own candidate list as suspected
inefficiencies, and both are wrong. `CLAUDE.md`'s *ask what your number counts
when nothing is wrong* applies to a code review as much as to a measurement.

- **`remove_rider_everywhere` on every organism death, including every
  plant.** Free at the shipped cap: nothing is ever inserted, so the map's
  capacity stays 0 and `retain` visits nothing. Already stated at the call
  site; the review nearly filed it anyway.
- **`cell_still_stands_for` per chain cell per creature tick.** One extra grid
  read, and at cap 1 it is a **structural** no-op rather than an incidental
  one: `reindex_organism_cell` keeps `cells` and the grid in agreement about
  ownership, so any cell reaching the test is still owned and the first branch
  returns. Gating it on `stack_cap() > 1` was the proposed optimisation and is
  the wrong trade — it saves a few thousand grid reads against a 163,840-cell
  sweep and buys a branch that makes the armed and unarmed paths structurally
  different, which is the thing the cap-1 identity argument rests on.
- **The cap arithmetic.** `1 + riders_at(p).len() < stack_cap()` is right at
  the boundaries: entering makes `2 + riders` creatures, so the test is
  `2 + riders <= cap`, and at cap 2 it admits exactly one rider.
- **`corpse.aux()` really is the worth** that `corpse_worth_suppressed` claims
  to sum — `stamp_as_corpse` writes `worth.round()` there — so the counter
  counts what its name says.
- **`place_corpse_beside` uses `World::is_empty`**, which is the managed-aware
  test, and that is the correct one here: the question is *is this position
  available*, not *is there material here* (`.claude/rules/src-sim-cells.md`).
- **One production caller of `add_rider`**, and it is behind `can_stack_into`,
  so "the caller owns the cap" has exactly one caller to be true of.

### A wart, deliberately left

Promotion writes the rider's cell value **as captured when it mounted**, so a
promoted cell carries a stale `temperature`. Harmless — heat re-diffuses on the
next tick — and the alternative is rebuilding the cell from the species def at
promotion time, which is a change to how a body's appearance is derived and
does not belong in a bookkeeping path.

## 11. The four follow-ups, closed 2026-09-19

Owner's list, in his order. **Each fix is paired with the fault put back and
the *cause* scored rather than the message** — §10's own account of three wrong
guesses at one assertion is why, and item 4 below is the case where scoring the
message would have been wrong again.

### 11a. `neediest_kin` can see riders — and the eye had to move with the mouth

The owner asked for the mouth: `neediest_kin` scans `NEIGHBOURS_8`, which never
contains `(0,0)`, and reads `cell.organism_id()`, which is the *owner's*. Both
are now answered by one helper, `fold_ridden_kin`, called at the animal's own
cells and at each neighbour. `nearest_foe` is deliberately left blind, because
the identical blindness is what implements the owner's attack ruling for free.

**Fixing only the mouth is a lever that fires and moves nothing, and that is
measured rather than reasoned.** `BrainInput::KinNeed` comes from
`adjacent_food_counted`'s scan, not from `neediest_kin`, and it was blind to a
rider in the same two ways. `ant.ron` authors `(Energy, Share, 2.5)` against
`(Bias, Share, -2.5)` — an exact cancellation at full energy — so the species
file's own words are that **`KinNeed` is the only input that can open the
`Share` gate from outside the donor's own belly**. With the mouth fixed and the
eye blind, `a_starving_rider_is_visible_to_the_verb_that_would_feed_it`
evaluates the real genome at the inputs the old scan produced and gets `Share`
= **exactly 0.0**: no draw is taken, and the target the mouth found is never
requested. So the eye is part of the mouth's repair, not scope creep.

Put the mouth's half back on its own and the same test fails differently and
usefully: the donor aims at the **host**, which is full, because the host's
second body cell is an ordinary grid neighbour. A share that moves nothing.

**The event count, and why it is a within-run number.** A paired `shares`
reading against the trunk cannot answer this: arming the cap moves a draw on
the first stacked tick, so the two arms are different worlds by the next frame.
Over six seeds on the crowded bed at cap 20, 3,000 frames, `shares` went
**193.3 → 206.5** — inside the 169–213 spread of the unfixed arm alone, and
therefore no evidence of anything. `CreatureStats::shares_in_stack` needs no
second world, because before the repair there was no path to a rider at all:
**43, 47, 51, 51, 54, 55** over those six seeds, about a quarter of all shares,
against **0** at the shipped cap. Both arms are asserted in
`a_crowded_colony_actually_stacks_when_the_cap_is_armed`.

### 11b/11c. A body never writes a cell it does not own — one rule, two bugs

§10 filed these separately and they are the same rule seen from two sides, so
they are fixed in one place: `stamp_as_corpse`, which is already the single
chokepoint for corpse writing.

**The dying rider** (§10's first filed bug) stamped nothing at all, because
`creature_dies` filtered its corpse cells down to the grid-owned ones — which
was right, or a rider's death would bury its host, and is exactly what
`a_dying_rider_does_not_bury_its_host` guards. So the owner's ruling (*first
free neighbour, else suppressed*) was implemented for a dying owner and not for
a dying rider: the meat was deleted in silence with `corpses_suppressed`, the
named-hole counter that exists so a leak correlated with the experimental arm
cannot go unnamed, reading 0 throughout.

**The severed ridden cell** (§10's second, filed *without* a reproduction)
reaches the same function from `reconcile_chain`: `surviving` keeps a ridden
cell, so it can fall out of the 8-connected component, land in `severed`, and
arrive where `riders_at(cell).first()` is **the severing animal itself** — and
the promotion branch would hand it the cell it was only borrowing, writing its
own body over its host's.

The rule that closes both: **a body that does not own this cell in the world
never writes it.** Its flesh was only ever in the rider index, so the meat goes
*beside* — the same ruling promotion already follows — and the host is not
touched. Unreachable at the shipped cap, structurally: a body owns every cell
it stands in there.

Two things ride along:

- **The worth divisor.** The dropped ridden positions were also being booked to
  `meat_lost` and left out of the divisor, so an animal's worth was spread over
  fewer cells than it had and every surviving corpse cell was made *richer* to
  compensate for meat that had been deleted. Both terms now read one list.
  Asserted directly: a half-riding, half-standing `Chain` body's two corpse
  cells must carry the same `aux`, and it must be the two-cell figure.
- **`stamp_as_corpse` returns how many corpses are actually standing**, and
  `creature_dies` books `Account::StoredInMeat` against that rather than
  against the cells it offered a place to. Crediting a suppressed corpse pushes
  `max_standing_meat` *down*, which is the one direction that can take
  `the_standing_meat_never_exceeds_what_was_put_into_it` red. The discrepancy
  existed at cap 1 too, through the promotion branch, and was being absorbed
  into the bound's slack.

**The reproduction §10 asked for** is
`severing_a_ridden_cell_leaves_the_host_holding_it`: a `Chain(6)` in a row with
its tail cell inside a nestmate, bitten two cells behind the head, so the tail
half *and the ridden cell* sever together — which is the only way to reach the
branch, since a body that keeps its ridden cell attached never severs it. With
the fix removed it goes red naming the cause: the tail cell's grid owner
becomes **1**, the severing animal, instead of **2**, the host. The severing
count is asserted *first*, so a scene where nothing severs cannot pass the
host-keeps-its-cell assertion for free.

### 11d. The stranded grid cell: found, and it was already closed

The defect: an animal left owning a grid cell that is not in its chain, with no
rider standing there. `creature_biomass` sums `cells.len()`, so it over-reports
for the rest of that animal's life.

**The mechanism.** `reindex_organism_cell` prunes `cells` only when a cell's
`organism_id` *changes* — §10's own dead-end entry (d) names that early return.
So a promotion that writes a cell **already carrying the outgoing owner's id**
is invisible to it: the owner keeps the entry and its chain walks on. The cell
that made this possible was the pre-fix `carried` read in `relocate_chain`,
which let a rider store its *host's* cell as its own — the bug §10 opens with,
fixed in the same PR. **The two are one bug seen at different times**, and the
owner's note that item 4 "reproduces at cap 1, so it predates this work" is the
part that does not hold up.

**Traced rather than argued, with the probe's sensitivity established first.**
A check at every `end_step` — any creature owning a grid cell outside its chain
— was injected with a hand-made stranding and confirmed to fire at frame 0.
Then, on pristine `main`:

| arm | result |
|---|---|
| crowded bed, cap 1, 40,000 frames | clean |
| crowded bed, cap 20, 3,000 frames (the run §10 quotes: 9,036 moves, 49 alive) | clean |
| whole `--lib` suite, 1,860 tests | clean |
| `tests/worldgen.rs` + `tests/determinism.rs`, 48 tests | clean |
| `examples/ascii` | clean |
| **the same bed with the pre-fix `carried` read reinstated** | **frame 35** |

At frame 35 it reads organism 41 holding `(219,119)` outside chain
`[(221,119),(220,119)]` with **riders 0**, and organism 49 the same shape while
climbing away from it — the report's own `[(9,119),(9,118)]` plus `(5,117)`,
to the arrangement. Removing `try_swap_with_kin`'s guard, the other candidate,
changed **nothing**: `passes_through_kin` is off in every species file, so that
function returns on its first line in every shipped scene.

**What is added is a guard at the seam, not a second fix.** `World::add_rider`
now attributes the stored cell to the rider (`cell.with_organism_id(id)`), so a
promotion always changes the id and the pruning seam always fires, whatever a
caller hands in. A no-op on every live path. It is at the seam because the
caller that got this wrong *looked correct*, and the symptom surfaced in a
different animal's bookkeeping two hundred frames later. A `debug_assert` was
the first version and is the wrong tool — compiled out of `--release`, which is
where every long run here is measured, so it would guard exactly the runs short
enough not to need it.

**And this is the case the owner's warning was about.** The fault-injection that
"verified" a fix by watching the expected message appear was failing for item 4
instead. Scoring the cause is what separated these two: item 4's shape is
`grid owner == me, not in my chain`, and the rider bugs' shape is
`grid owner == somebody else`. One invariant cannot tell them apart, which is
why `a_crowded_colony_actually_stacks_when_the_cap_is_armed` asks the
grid-disowned question and the new tests ask the other one.

### 11e. Still open, and deliberately not touched

- **A minted lateral does not ask `can_stack_into`.** `relocate_chain`'s mint
  loop writes a re-emerging `Segmented` lateral with a plain `World::set`, so at
  a cap above 1 a `longant` re-widening into a cell a nestmate stands in would
  overwrite it rather than ride it. Unreachable for every `Chain` and `Rigid`
  species, and `longant` is the only `Segmented` one shipped.
- **Suppressed meat is still not booked to `meat_lost`.** The honest amount is
  the *stamp* part only — the live share already goes to `Account::Dissipated` —
  and `place_corpse_beside` does not know `body_energy`. `max_standing_meat` is
  an upper bound, so the omission only ever loosens it and cannot turn a guard
  red; `corpse_worth_suppressed` is the named figure to subtract by hand, which
  is what §10 says it is for.

### 11f. Bit-identicality at cap 1, measured

The whole feature's claim, re-checked because four of these changes touch shared
paths. Crowded bed, cap 1, 3,000 frames, digest over every cell's material, id
and `aux` plus every organism's energy and chain: **`0x6fde91732aaa5a65` on both
`main` and this branch**, with 8,578 moves, 51 alive, 21 deaths on each. At cap
20 the world diverges and must — trophallaxis now reaches inside a stack, which
moves a draw.
