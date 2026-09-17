# Stacking: many creatures of one colony in one cell

*Design of record, 2026-09-17. Owner's brief, and his rulings throughout §3.
Nothing in this report has been built yet; §5 is the pass being built against
it. Supersedes nothing — it is the first design on this axis — but it is the
third attempt on colony traffic and §1 says what the other two were.*

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

**5. Multi-pip render.** `render.rs` draws from cells, so riders do not exist on
screen until it asks the index. Offset, dimmed pips within the one cell.

**6. Teach `neediest_kin` about riders.** `Share` reaches through
`neediest_kin` (`creature.rs:5890`), which scans `NEIGHBOURS_8` around the
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

**No review card until step 5 lands, and that is deliberate.** The eventual
verdict is a blind A/B of a jammed trail at cap 1 against cap 20 with the
counts in the card's `meta`. But while `render.rs` draws only the cell owner,
an armed stack renders as **fewer** visible ants than cap 1, not more — a card
posted before the render would show a jam looking emptier and invite exactly
the wrong conclusion.

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
