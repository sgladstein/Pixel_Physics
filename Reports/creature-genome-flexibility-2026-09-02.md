# The creature genome: what is still authored, what it costs to free, and how to tell whether it worked

**Status:** design, 2026-09-02. Nothing built. Scoped to **the evolution lab**
— the owner's redirect mid-discussion was *"we are specifically focused on the
evolution lab. we don't need to worry about the outdoor part of this game"* —
so where a change would cost the shipped outdoor ant something, that cost is
named and then set aside rather than paid.

**The brief, in the owner's words, 2026-09-02:** *"I want an implementation
that is very flexible that allows complex behaviors to develop/evolve with the
minimal amount of hard-coded behavior. For example, I don't like that we have
directly encoded there being a nest and dropping food at the nest. It seems
designed like we were intentionally trying to create that behavior and
intentionally trying to create ants."*

Answers given in the same conversation, and binding on everything below:

| | |
|---|---|
| **The nest is generalised, not deleted** | option 1a of the fork put to him — creatures sense a *place* rather than a named material, and a colony becomes one outcome among several |
| **Mind before body** | the brain and its senses first; heritable anatomy (S8) after |
| **The competent ancestor is expendable** | *"willing to give up"* — the lab founder does not have to forage on placement |
| **Eyes cost, and the cost rises with range** | *"anything can get eyes but they should have a cost... although I don't know how we balance that"* |
| **The specimen shelf is expendable** | *"we can lose the shelf"* |
| **Risk accepted** | *"in the middle is better but I am willing to try the risky option and give up if it doesn't work"* |
| **Size must buy survival** | *"yes size (or other physical features) should buy survival"* — §11 is the recommendation asked for |
| **The body will be fully revamped** | so this plan is written **body-plan-independent** rather than run concurrently; the contract is §12 |
| **The ratchet check gets built** | *"lean towards build it"* — `gene_probe`, §10 |
| **Kind 3 genes are not wanted** | *"they don't seem highly important"*; two of the three should never become genes — §7 |
| **The body revamp's target is the silhouette** | *"I don't want all to look like a chain… larger creatures just become snakes/worms"* — §13 |

---

## 0. The recommendation, stated once

**Build the creature Gate 2 — a `selection_arena` for animals — and take the
baseline on today's ant, before any of the mechanism work below.**
Recommended by this document; put to the owner 2026-09-02 and **accepted**.

It is one harness, it is modelled on two that already exist
(`selection_arena` for plants, `labbatch` for the rack), and it is not a
detour: it is the **control arm** the nest change needs anyway, taken on the
same binary in the same session, which is what the paired-comparison rule
requires.

**Why it is not optional.** The lab's own coordinator note has carried the
same line through five rounds — *"Gate 2, does selection have teeth in this
bed, has still never been run, and `selection_arena`'s whole finding is that a
null there is a statement about the world rather than about the genome. Until
it passes, every evolution result measured in this bed is unvalidated."* And
the noise floor is measured: `labbatch`, 12 seeds at 9,000 frames, puts the
**world seed alone** at **2.42x-3.12x** across the lab census with no true
effect present.

So without it, the outcome of the whole programme below is unreadable in the
one direction that matters. If the de-hardcoded ancestor stops foraging, we
cannot distinguish *the mechanism failed* from *this bed never selected for
anything* — and the creature line has now ended three times with the finding
that the answer was the ecology and not the creature.

Full argument in §9; the staging is §10.

---

## 1. What already answered this, and must not be re-derived

The owner made a near-identical objection on 2026-08-30 —
*"this sounds like we're forcing a system into creating behaviors that we want
instead of creating the most correct system and allowing behaviors to
develop"* — and stages S0–S5 answered a large part of it. The account is
[`creature-gates-to-mechanism-2026-08-31.md`](creature-gates-to-mechanism-2026-08-31.md),
PRs #190, #192, #194.

**Already gone, and not to be re-found as findings:**

- `hunger_fraction` and Gate 0's provisioning clause — an animal no longer does
  arithmetic about the price of its own children. It takes what it finds and
  digests it as it walks, so the eat-or-carry *outcome* is a consequence of
  trip time.
- The statement-order bug that made a colony re-take what it had just
  delivered. `Feed` against `Drop` is now a `choose_weighted` over two brain
  outputs, so which verb fires is genome.
- `CreatureDef::food`, the name whitelist. Diet is `TRAIT_GUT_BIAS`, one
  heritable scalar scored against `MaterialDef::food_class` through a matched
  filter.
- A birth payable **from food within reach**, deliberately not from a nest,
  *"which would have hardcoded a colony"*.
- Two hand-written placement rules for a dug pellet, both withdrawn on the
  owner's own ruling: *"is this a problem for you to solve or for the ants to
  solve."*

**The governing line adopted with that programme, and still holding: the
mechanism is code, the policy is genome.** Its two corollaries did the work —
*add senses and economies, never behaviours*, and *a sense must not
pre-categorise what it senses.* Everything in this document is an application
of the second corollary to the places S0–S5 did not reach.

---

## 2. The inventory: what still spells "ant" in Rust

Four mechanisms, and each is a different *kind* of authoring. Distinguishing
them matters, because the remedy is different for each and one of the four
should probably survive.

### 2a. Home is a material named in the species file

`ant.ron` writes `nest: "nest"`. `BrainInput::AtNest` (slot 10) is
`adjacent_nest` (`creature.rs:2575`), a contact scan of the head's
8-neighbourhood for that one material id.

**This is the only sense in the suite that pre-categorises what it senses**,
and it therefore violates the corollary the previous programme adopted. Compare
its neighbours: `FoodAdjacent` is filtered through the animal's own heritable
gut rather than a list; `PreyNear`/`PreyBearing` are "the nearest thing of
`MaterialKind::Creature` my gut would get value from", which is a filter over
data, not a category. `AtNest` is a name.

**And nothing creature-side can ever make nest material.** Ants dig soil into
`packedsoil` pellets (`soil.ron`'s `packs_into` is the only one in the whole
material table). `nest` arrives only from outside: `creature::found_colony`
paints one row across 53 columns, and in the lab `Tool::Colony` does it under
your click. `lab/mod.rs:1440` says why out loud —

> *A colony gets the patch of nest and the band along the ground either side of
> the click, because without a nest there is no gradient and nobody forages.*

So home is a gift from the scene. The clearest single statement of the owner's
complaint is that placing a colony in the lab **paints the precondition for the
behaviour we then observe.**

### 2b. Homing is an algorithm in Rust, not a gene

`creature.rs:1897`:

```rust
let since = world.organism(organism).map_or(0, |s| s.since_nest);
let recency = (1.0 - since as f32 / def.nest_memory.max(1) as f32).max(0.0);
let emit_a = outputs[BrainOutput::EmitA as usize].clamp(0.0, 1.0) * recency;
let emit_b = outputs[BrainOutput::EmitB as usize].clamp(0.0, 1.0);
```

`nest.ron`'s own comment is admirably honest about what this is: *"an ant's
channel-A deposit is scaled by how recently it touched nest material, so
outbound ants paint a gradient that is freshest nearest home, and a laden ant
walks up it. No ant ever asks where the nest is."*

The mechanism is elegant and it is **an odometer written in Rust**. Three
consequences:

1. A lineage cannot evolve a different way of finding its way home. It can turn
   the volume up and down on the one provided, and nothing else.
2. **The two pheromone planes are asymmetric by code, not by evolution.** A is
   the homing plane because a multiplier says so; B is not. A species cannot
   swap them, use both for food, or use neither.
3. `since_nest` is `saturating_add(1)` every tick and reset only on nest
   contact, so the whole homing system is keyed on 2a. Remove the named
   material and this stops working — the two are one mechanism in two places.

Worth recording because it is the trap: `creature.rs:3272` carries a comment
saying an edit once dropped the `since_nest = 0` reset while adding a counter,
and **nothing in the suite went red** — 827 tests passed, clippy clean,
`ascii`'s scenes still delivered food. Only a paired baseline run caught it. Any
work in this area is guard-blind by default.

### 2c. Delivery is privileged

`creature.rs:2786`:

```rust
let at_nest = adjacent_nest(world, x, y, def);
// At the nest it is storage and always wanted; out on the route it
// is construction, and *there* the moisture bias decides.
let p = if at_nest { drop_urge } else { drop_urge * moisture_gradient(world, x, y) };
```

A hardcoded claim that putting something down at home is a *different act* from
putting it down anywhere else. It is also where `deliveries` is counted, so the
engine's own definition of a successful forage trip is keyed on the named
material too.

### 2d. The body does not evolve at all

Heritable today: the brain genome, plus exactly three scalars.
`CREATURE_TRAITS = 3` — `TRAIT_GUT_BIAS`, `TRAIT_BIRTH_GRANT`,
`TRAIT_REPRODUCE_AT`.

Not heritable, all species constants in a `.ron`: `body` (plan and length),
`tick_interval`, `sensor_offset`, `sight_range`, `dig_force`, `crop_capacity`,
`digest_rate`, `start_energy`, `idle_cost_per_cell`, `move_cost_per_cell`,
`body_energy`, `synapse_fraction`, `sight_fraction`, `climbs_over_kin`,
`eats_kin`, `nest`, `nest_memory`, `reproduce_threshold`, and **`mutation_rate`
itself**.

**A lineage can change its mind. It cannot change its body.** S8 (heritable
anatomy) is specced in `creature-evolution-plan.md` §2.8 and was never built.

### Three structural caps, recorded rather than complained about

- **Fixed brain topology**: 18 inputs / 4 hidden / 11 outputs, one hidden layer
  with self-recurrence only. This is decision D4 and it is *deliberate* —
  NEAT-style topology evolution was assessed and rejected, on the grounds that
  a variable-length graph genome needs speciation machinery, needs hours of
  noise to bootstrap, and produces networks nobody can read. Do not re-argue
  it. But note the practical cap: **four hidden units is the whole of an
  animal's internal state**, and `ant.ron` already spends all four on one
  gated pair.
- **No recombination.** Reproduction is asexual budding with per-slot point
  mutation. Round four's lab note says it plainly: *"`CROSS` (breed two jars) —
  D4 caged the brain's topology on one shared scaffold precisely so crossover
  is possible, and there is still no verb."*
- **Sight sees only creatures.** `is_visible_prey` requires
  `MaterialKind::Creature`. A herbivore's eyes are useless; all plant food is
  contact-range. This is not a design decision anyone recorded — it is where
  the sense happened to stop when it was built for predation.

---

## 3. The organising frame: three kinds of lever

**This is the reusable part of this document, and it answers the owner's
"I don't know how we balance that" directly.** The recurring failure on both
the plant and creature lines is not that a gene was wrong — it is that a gene
was *unpriced*, went to its cap on the first generation, and expressed nothing
while looking exactly like a working feature.

The evidence, three separate incidents:

- **Plant architecture.** Sympody, tropism and acrotony were built, ranked
  "very high" on silhouette impact, and all three demonstrably fired (46–186
  sympodial forks per shrub, 1,797–2,750 plagiotropic steps per conifer). The
  owner's reading of the sheets was that nothing had changed, and the
  composition numbers agreed. All three moved *which cell gets a label*;
  nothing moved the silhouette. `plant-appearance-design.md`.
- **`phototropism_dir`.** Its codomain was `{(0,-1), (0,0)}`, so `light_weight`
  could only ever reinforce the up-vector. Widening it to a real 2D gradient
  gave those weights a direction they had never had, trees spread instead of
  climbing, never reached `seed_maturity`, and **reproduction went to zero.**
  Every gate stayed green but one.
- **Body length.** `creature-evolution-plan.md` E10 authorised chain length as
  the cheap route to a visible animal, on the stated premise that *"per-cell
  metabolic cost already prices a longer body, so no cost system is owed"*. The
  creature coordinator checked and the premise was false — both cost paths were
  flat per animal. A longer body was **strictly free and strictly better**: 2.9×
  the ink on screen, chain bodies blocked 2–6% of their moves against 25–43%
  for rigid ones, identical bill.

So classify every candidate before building it:

### Kind 1 — self-limiting. No price needed.

More is not better, so the lever cannot ratchet: the optimum is interior and
selection finds it. `sensor_offset` is the clean example — smell too far ahead
and you are sampling somewhere irrelevant, and `pheromone::trail_following_sweep`
measured exactly that shape (0.755 at 4, **0.817 at 6**, 0.743 at 8, 0.727 at
10). Most steering weights are here. **Free to make heritable today.**

The test: *can I state a setting at which more of this is worse, and is that
setting reachable?* If yes, ship the gene.

### Kind 2 — already priced. Safe now.

A cost term exists and scales with the lever. **Safe to make heritable, and
three of these are sitting unclaimed** (§7).

The test: *name the line that charges for it, and show the charge rises with
the gene.* If you cannot name the line, it is Kind 3.

### Kind 3 — unpriced. Do not make heritable yet.

The gene and its cost term are one piece of work, not two. Building the gene
first produces a ratchet that looks like a feature. `dig_force`,
`crop_capacity` and `digest_rate` are all here today.

**The corollary that keeps this cheap: a Kind 3 lever can usually be promoted
to Kind 2 by one cost line, and that line is smaller than the gene.** The
`sight_fraction` work is the worked example — one multiply in the tick, and an
entire axis moved class.

---

## 4. Replacing the nest: a kin sense

The owner chose 1a — generalise rather than delete. The honest generalisation of
*"am I home"* is not another material name.

### The proposal

**Add `KinNear` and `KinBearing` as brain inputs, mirroring
`PreyNear`/`PreyBearing`, and delete the nest's privileges.**

Home becomes *where my own kind are*. That is a genuine sense of a real
quantity, not a category, and it makes a colony an **outcome** rather than a
precondition: aggregation produces a place, a place produces a gradient, a
gradient produces central-place foraging — or it does not, and we have learned
something real rather than confirmed our own scene painting.

### Why this is cheap

The machinery exists. `creature::sight` already casts `SIGHT_RAYS = 16` rays
all round, already resolves organism ids, and already calls `is_living_kin` at
the hit site to *exclude* nestmates from prey. The kin sense is the same rays
with the predicate inverted. `PreyBearing`'s design notes already argue why the
pair is a pair — *"a magnitude says there is something, a direction says that
way, and the pair makes pursuit reachable by one connection from each"* — and
every word of that transfers to aggregation.

It also sidesteps the standing trap. A full-circle bearing to one cell found by
a ray traced at CA resolution is explicitly **not** a two-point difference on a
coarse field — `CLAUDE.md`'s degeneracy, hit four times on three lines and never
once caught by a test.

### What it costs

**Ants have no eyes today.** `sight_range` defaults to 0 and only `beetle.ron`
authors it (64). Giving ants a kin sense turns the ray fan on for every ant in
the box.

`creature-vision-sizing-2026-08-30.md` §5 priced a radius-64 fan at **0.004
ms/frame at five beetles**, 0.14% of a mean `ascii` frame. Ants tick every 6
frames, so 52 ants amortise to roughly 8 casting per frame — the same order.
**A shorter range is both cheaper and more honest for an ant**: 16–32, not the
beetle's 64. This is a measurement to take rather than a number to trust, and
`vision_probe mode=cost` already answers it.

The alternative — a third pheromone plane creatures emit passively — is
**worse**, and measured: E13 gates it at **0.5 ms** on a settled world against
the two existing planes at **0.0014 ms**. Three hundred times the cost of what
is already there, for a weaker signal. Do not reach for it.

### Retiring 2b and 2c

With a kin sense in place:

- **Delete the `recency` multiplier.** A self-recurrent hidden unit driven by
  the kin sense *is* an odometer, expressible in three weights: an input weight
  from `KinNear`, a self-recurrence weight setting the decay, an output weight
  into `EmitA`. What was a species constant (`nest_memory: 3000`) becomes a
  number selection can move, and the two pheromone planes become symmetric —
  either can be a homing plane, both can be, neither need be.
- **Delete the `at_nest` branch in the drop.** See §5.
- **`AtNest` stays wired.** The positional law forbids removing a slot, and the
  outdoor ant keeps it. New senses are appended; not one existing weight moves.

**The precedent for authoring a hidden unit rather than coding a mechanism is
already in the tree**, and it is the same shape: `genome_from_wiring`'s doc
records that *"a species that needs a hidden unit on generation zero authors it
as weights, in data, exactly like any other instinct — it does not get a special
case in code"*, and `ant.ron` uses a symmetric pair to express the laden/unladen
gate that a linear layer provably cannot.

**The catch, and it forces a scaffold change**: `ant.ron` already spends all
four hidden units on that pair. An authored odometer needs more. `BRAIN_HIDDEN`
4 → 8 is lawful under the reserve (`HIDDEN_SLOTS = 64`) but is not free — see §8.

---

## 5. The moisture gradient — the one that should mostly survive

`creature.rs:2589`:

```rust
fn moisture_gradient(world: &World, x: i32, y: i32) -> f32 {
    let m = |px, py| world.field_at_bilinear(px as f32, py as f32).moisture;
    let gx = m(x + 4, y) - m(x - 4, y);
    let gy = m(x, y + 4) - m(x, y - 4);
    ((gx * gx + gy * gy).sqrt() / WORM_MOISTURE_SATURATION).clamp(0.0, 1.0)
}
```

Drop probability is multiplied by this; dig probability by its inverse.

### The thinking, and it is good

Source is [`stigmergy-research.md`](stigmergy-research.md) §4 on Facchini et
al., *eLife* 2024. The classical model of termite construction — Deneubourg
1977 and every agent model after it — assumes a **cement pheromone**: a marker
added to deposited material that stimulates further deposition nearby. That
study ran *Coptotermes gestroi* on clay arenas with pellets **sterilised to
remove chemical marking**, tracking collection and deposition as separate
events. Findings:

1. **Collection is uniform across the arena; deposition is concentrated.** That
   asymmetry is the whole algorithm.
2. The single feature shared by every deposition region was that it was a local
   maximum in **evaporation flux** — which is provably proportional to local
   surface curvature.
3. Convex regions attract deposition; concave regions attract digging. This
   reconciled an earlier study that had reported the opposite sign, because that
   study could not separate digging from building.
4. A salt-solution control with **no termites** deposited salt in precisely the
   regions where termites had built.

So this is the **opposite** of the nest. The nest asserts an outcome. This
asserts a *physical fact about drying* and lets the outcome emerge. Pillars,
walls, galleries and chambers are consequences. `act`'s own comment states the
discipline: *"There is no 'build a wall' behaviour and wanting to write one is
the signal to re-read that section."*

**This is the design the owner is asking for, already done right. Keep the
channel.**

### The critique, which is narrower than "it's hardcoded"

**The coefficient is fixed.** Every animal in the world has the same
relationship to curvature, at the same strength, with the same sign, for ever.
A lineage that nests in hollows instead of building on ridges is not a point in
the search space — there is no gene to be one with. And `dead-ends.md` entry 983
already prescribes the remedy in general terms, from the neighbouring case of
gating the spoil drop on `LightHere`:

> *It wants to be a wired instinct on its own brain output rather than a
> coefficient in `act`, so a lineage can lose it.*

**So: add a `MoistureGrad` input carrying the scalar this function already
computes, delete both multipliers from `act`, and author `ant.ron` with the
weights that reproduce today's behaviour.** The termite bias, its inverse, and
everything between become reachable. The physics stays in code; the *response
to* the physics moves into the genome. That is the mechanism/policy line
applied exactly.

This also retires §2c for free: with the drop bias a weight rather than a
multiplier, there is no `at_nest` branch to privilege.

### A dated finding: the sampler predates the field it samples

**`moisture_gradient` samples at ±4 cells. `FIELD_SCALE` is 16.**

- `field.rs:48` — `pub const FIELD_SCALE: i32 = 16;`
- `ca7e9042`, **2026-08-30**, *"field: FIELD_SCALE 8 -> 16, the light resolution
  the owner picked by eye"*, on `main`.
- The `m(x + 4, y) - m(x - 4, y)` lines were last touched by `fac79156`,
  **2026-08-29**.

So the ±4 offsets were chosen when a field block was 8 cells — half a block
each side, a full block across, which is a sensible sampling span. The field
then doubled to 16 and **nobody re-derived the sampler.** The two reads are now
8 cells apart inside a 16-cell block. `field_at_bilinear` saves it from being
the outright block-nearest degeneracy, but the gradient returned is a fraction
of the true inter-block gradient, and the fraction changed silently under a
commit about light.

This is `CLAUDE.md`'s *fixing a bug often exposes a constant that was
compensating for it*, in its second shape: a constant calibrated against a
quantity that then moved.

**And there is a second, larger concern, which is unmeasured.** Field diffusion
is gated on `blocked`, and `rebuild_blocked` marks a whole block blocked if
**one** cell in it is `Solid` — with a deliberate exception only for blocks that
are themselves moisture *sources*. The lab's own round-three finding is the
sibling of this: *"roots steer by air humidity, not soil water — hydrotropism
reads the coarse field channel, which does not diffuse inside solid ground, so
below the surface it has no gradient, which is why roots stop at 13 rows."*

If that holds for `moisture_gradient` too, then **inside a burrow** the drop
term goes to ~0 and the dig term to full `dig_urge`, unshaped — meaning the
termite construction mechanism is **inert exactly where a nest gets dug**, while
galleries still appear for other reasons (`line_burrow`'s tamping, and the
pellet-placement predicate). That is the signature of a mechanism that looks
like it works and is not the thing producing the result.

**This is a hypothesis with a cheap test and it is owed before anything is built
on the channel** — see §10.

---

## 6. Eyes: the cost already exists, and it is better than a range charge

The owner's instinct was *"they should have a cost (require more food and that
increases with range), although I don't know how we balance that."*

**`CreatureDef::sight_fraction` landed 2026-08-31 and does exactly this.**
`creature.rs:1825`:

```rust
let sight_tax = def.sight_fraction * def.start_energy * sight_reads as f32;
```

Its doc records the reason in the same terms this document's §3 uses:

> *Seeing was free until 2026-08-31, and that made `sight_range` a ratchet.
> `sight_casts` and `sight_cells_read` counted the work and nothing billed for
> it, so more eye was strictly better and a heritable sight range would have
> gone to its cap on the first generation, expressing nothing.*

**Charged per cell the eye actually read, not per cast**, and that is better
than the flat range charge the owner imagined. A ray dies on the first blocker,
so it costs only what it traversed — *an animal in a tunnel pays less to look
than one sweeping open ground.* **Shelter then pays for itself twice: it hides
you, and it makes your own eyes cheaper.** Nobody wrote that rule; it falls out
of pricing the work honestly. It is also the exact behaviour S5a went looking
for and could not find a selective gradient for, arriving from the economic side
instead of the predation side.

A fraction of `start_energy` rather than an absolute, for the reason
`synapse_fraction` states: an absolute silently becomes a different tax every
time the budget moves, which once spent **80% of a life on thinking** and
invalidated a three-knob sweep.

### Two gaps, both small

1. **No species authors a `sight_fraction`.** It is `#[serde(default)]` at 0, so
   the beetle's radius-64 eyes are **free today**. The mechanism shipped and
   nothing opted in — a channel with a reader and no writer, which
   `dead-ends.md` names as the failure this project has hit three times.
2. **`sight_range` is a species constant, not a trait.** The pricing exists; the
   gene does not. This is a Kind 2 lever sitting unclaimed.

### How to set the number, rather than balance it

Derive it the way `synapse_fraction` was derived, not by taste. `ant.ron`'s
`synapse_fraction: 0.0000022222222` is authored so that a stated synapse count
costs a stated share of an idle lifetime; `beetle.ron` states its arithmetic
inline (`1.25e-6 × 1600`).

The equivalent here: pick the share of an idle lifetime a **full-radius sweep
over open ground** should cost, divide by cells-read-per-tick at that radius,
and author that. Then stop — **do not balance the range.** Price the cell-read
and let selection settle where the eye lands. That is the whole point of the
exercise, and picking a "correct" range by hand would be the authored-behaviour
failure in a new costume.

---

## 7. The candidate list, classified

### Kind 1 — self-limiting, free to make heritable now

| gene | why it cannot ratchet |
|---|---|
| `sensor_offset` | measured interior optimum: 0.755 at 4, **0.817 at 6**, 0.743 at 8, 0.727 at 10 |
| the moisture response (§5) | a sign *and* a magnitude; both extremes are bad architecture |
| all steering weights | already heritable; listed for completeness |

### Kind 2 — already priced, safe now

| gene | the line that charges for it |
|---|---|
| **`sight_range`** | `sight_tax = sight_fraction × start_energy × sight_reads` — rises with radius through cells read. Author the fraction first (§6). |
| **`tick_interval`** | every cost is charged **once per creature tick** (`spent = idle + synapse_tax + sight_tax`), so halving the interval doubles the bill per frame. **Priced by construction, and nobody noticed.** |
| **body length / girth (S8)** | `idle_cost_per_cell × body_cells` and `move_cost_per_cell × (body_cells + carried_cells)`. |

**The S8 entry is a live correction.** The creature coordinator's note
(2026-08-30) blocked heritable body size on a verified reading of the code —
*"`creature.rs:1287` `let mut spent = def.idle_cost + synapse_tax;` …
`creature.rs:1329` `spent += def.move_cost;` … Both flat per organism. Nothing
in either cost path reads `chain.len()`."* That is **no longer true.** Both
fields went per-cell on 2026-08-30 and were renamed to carry the meaning change,
and `idle_cost_per_cell`'s doc names E10's false premise as the thing it was
fixing. **The stated blocker on heritable body length has been removed and the
plan has not caught up.**

Two conditions from §2.8 still stand and are not about pricing: the girth
pre-check (if wide bodies are more than twice as blocked, width is lethal at
every setting but one and is not a gene — `creature_scale mode=walk` answers
it), and birth placement (*"no space → the birth does not happen"* is a gate on
whether something happens, keyed on a heritable trait, which is a silent
selection pressure for smallness).

### Kind 3 — unpriced. The gene and its cost are one piece of work

`dig_force` (stronger mandibles cost nothing), `crop_capacity` (a bigger
stomach is free), `digest_rate` (faster absorption is free). Each needs a cost
term built *with* the gene.

**Owner's ruling 2026-09-02: none of these is wanted as a gene, and the
recommendation is that two of the three should never become one.**

- **`crop_capacity` is how much food, by worth, an animal can hold at once** —
  its crop is checked in `act` as `c.worth() + c.unit <= cap`, and the animal
  digests the contents as it walks. **Do not make it a gene: derive it from
  live cell count** (§11e). A bigger animal carries more, which is the honest
  physical answer, costs nothing to price because body size is already priced,
  and hands the body axis a foraging payoff to set against its upkeep.
- **`dig_force` should not get a slot either** — §11c gives it a second job
  (biting) and therefore two opposed pressures, which is what makes an axis
  worth having. Promote it when the body is heritable and it can scale with
  the mandible rather than float free.
- **`digest_rate` is genuinely left for now.** No recommendation; nothing in
  this plan needs it and it has no obvious physical parent to derive from.

**The general move worth keeping:** the cheapest fix for a Kind 3 lever is
often not to price it but to **derive it from a lever that is already
priced**. That converts an unpriced gene into a free consequence and removes a
knob rather than adding a cost system.

### Not parameters at all — the three that change what is reachable

**Crossover.** The largest missing piece of evolutionary machinery, and the
scaffold was caged for it: D4's whole argument for a fixed topology ends
*"which is also what keeps crossover cheap to add later."* Asexual budding plus
point mutation gives each lineage an independent random walk; recombination is
the only operator that lets two good ideas that arose in different lineages ever
meet. The shelf already holds jars to cross and the verb does not exist.

**Heritable mutation rate.** One trait slot, and **self-regulating**: too high
and the lineage shreds itself, too low and it stops adapting. Evolvable
evolvability, nearly free. Kind 1 by the §3 test.

**Heritable spread, not just a heritable mean.** `creature-evolution-plan.md`
§7 item 3 calls this *"the one genuinely open-ended-flavoured mechanism in the
review that survives contact with the constraints"* — a lineage able to widen
its own reachable range. Compatible with budding, costs one array. Its price is
that clamps become the only thing between evolution and a 4,095-cell animal, so
those clamps must **bound work rather than gate whether something happens.**

### The appearance axis, which is the owner's own stated blocker

Recorded here because it will otherwise be rediscovered. E8's constraint is
*"we definitely want new creatures to not look like a recolored ant"*, and the
36-cell evolved creature came back rated on **shape**: *"It is a perfect cube.
Are there perfect cube creatures in our world?"* and *"both are smudges but A is
closer."*

Colour is per-**material**, resolved by species name. `ShadeRule` gives a body a
gradient; it does not give two individuals different palettes.
`plant-appearance-design.md`'s finding is that a lever which relabels a cell
cannot move a silhouette that texture and colour set — and that finding has now
arrived on the creature line intact. **Until palette and texture vary per
individual, "not a recoloured ant" is unreachable by evolution however rich the
encoding gets.** Mind-first is the owner's call and this is not next; it is the
thing that must not be forgotten when body work starts.

---

## 8. What the scaffold change costs

### The genome grows, lawfully

Live counts today: `BRAIN_INPUTS = 18`, `BRAIN_HIDDEN = 4`,
`BRAIN_OUTPUTS = 11`. Wired slots, computed from `is_live_slot`'s own
arithmetic:

```
input -> output   11 x 18 = 198
input -> hidden    4 x 18 =  72
hidden self-rec         4 =   4
hidden -> output  11 x  4 =  44
                          -----
                            318
```

Proposed: inputs 18 → 21 (`KinNear`, `KinBearing`, `MoistureGrad`), hidden
4 → 8, outputs unchanged.

```
input -> output   11 x 21 = 231
input -> hidden    8 x 21 = 168
hidden self-rec         8 =   8
hidden -> output  11 x  8 =  88
                          -----
                            495
```

**`GENOME_LEN` does not change** and **not one existing weight moves.** That is
S2's reserve working exactly as designed: `INPUT_SLOTS`, `HIDDEN_SLOTS` and
`OUTPUT_SLOTS` are all 64 against live counts of 18/4/11, and every block is
sized from the reserve rather than from a live count. Appending lights up
storage that already existed and was already zero.

### But the wiring is not free, and this is the real cost

`live_slots()` goes **318 → 495, a 56% increase in the mutable surface.**
Consequences, all of which must be handled in the same change:

- **`mutation_rate` silently changes meaning.** It is a *per-slot* probability,
  chosen so that the expected number of moved slots per birth is sensible. At
  495 slots the same rate mutates 56% more of the genome per birth. This is a
  shared-budget reallocation and `CLAUDE.md` requires the constant be re-derived
  as part of the work, not inherited.
- **The shelf's "brood" dial changes with it.** One brood is *"the engine's own
  per-birth mutation applied once"*, so the dial's units move when
  `mutation_rate` does.
- **The synapse tax rises with density.** `synapse_fraction × start_energy ×
  active_synapses`, and a wider genome drifts toward more active synapses. This
  is the intended pressure, not a bug, but it means an unchanged
  `synapse_fraction` is a *different* tax after the change.
- **`random_genome` draws more values**, so a sampled genome at a given seed is
  a different animal. Every `creature_space` baseline is void and must be
  re-taken.

### The shelf breaks, and the fix is small

`brain::genome_manifest()` hashes the six dimensions **and the ordered slot
names**, and it is pinned to a literal — `brain.rs:1041`,
`assert_eq!(genome_manifest(), 1_520_499_525)`. `specimen.rs`'s `genome_of`
refuses any jar whose stamp is not exactly this build's.

So appending a sense invalidates **every specimen on the shelf**. The owner has
said this is acceptable (*"we can lose the shelf"*), so it is not a blocker —
but the fix is worth doing anyway, because it is small and it makes every
future append cheap:

**The refusal is over-conservative for an append.** Jars store sparse
**named** wiring lists (`brain::Wiring`), not raw floats. A pure append — no
rename, no reorder, no removal — leaves every stored weight perfectly
meaningful; the names it refers to all still exist and mean what they meant.
Trait vectors already handle this correctly (`Vec`, padded with the species
mean when short, refused when long, with the doc noting that fixed arrays
*"would have turned a lawful append into 'every jar on the shelf fails to
parse'"*). **Only the brain axis is brittle, and it should adopt the rule its
neighbour already has.**

Concretely: stamp the *dimensions and names* such that a jar loads when the
stored name list is a **prefix** of the current one, and refuse otherwise. That
keeps the protection against a rename or a reorder — which is what the manifest
exists for — while making an append a non-event.

---

## 9. Why an instrument comes before the mechanism

**There is no Gate 2 for creatures.** `selection_arena` — two genomes competing
in one bed, attribution by lineage, read at an order statistic — exists for
**plants only**. The lab coordinator note has carried the same line through five
rounds:

> *Gate 2, does selection have teeth in this bed, has still never been run, and
> `selection_arena`'s whole finding is that a null there is a statement about
> the world rather than about the genome. Until it passes, every evolution
> result measured in this bed is unvalidated.*

And the noise bar is measured. `labbatch`, 12 seeds at 9,000 frames on the
shipped bed:

| column | min→max ratio from the **seed alone** |
|---|---|
| plants | 2.42× |
| plant cells | 2.62× |
| animals | **3.12×** |
| seeds borne | 2.53× |

**That is the spread with no true effect present, and it is the bar any real
comparison has to clear.** Its `same` arm reads exactly zero on all six
columns, so the engine is reproducible and the spread is genuinely the seed.

The consequence for this work is blunt: **if the de-hardcoded ancestor stops
foraging, we cannot currently tell whether the mechanism failed or the bed never
selected for anything.** That is the failure `CLAUDE.md` names — a null that is
a statement about the world wearing the clothes of a statement about the genome
— and it is the difference between an experiment and a session spent guessing.

The instrument is not a detour. It is the **control arm** the change needs
anyway, and building it first means the baseline is taken on the same binary in
the same session, which is what the paired-comparison rule requires.

**What it is:** a creature `selection_arena`, modelled on the plant one and on
`labbatch`'s rack. Two genomes, one bed, competing for the same food, water and
space; attribution by `OrganismState::lineage` and **never by genome**, which is
what makes the identical-arms control measurable at all. `arm=` a ladder rather
than a switch, so it reports *where* the world stops discriminating. And — the
plant harness's hardest-won lesson — **`mirror=off` is not an option, it is the
control**: mirrored, `arm=same` is one simulation with the labels swapped, so
`A == B` is an algebraic identity and its exact 50.0% says nothing. That was the
first result the plant harness produced and it was vacuous.

---

## 10. The staged plan

Each stage states what it delivers, what number says it worked, and what would
say it did not.

### Stage 0 — the two measurements owed before anything is built

**0a. Does `moisture_gradient` read anything underground?** (§5)

Print the value for ants inside a `labnest` gallery against ants on the open
surface, and the same at both `FIELD_SCALE` sampling spans. **Positive control
first**: a hand-built convex ridge must read high and a flat plain low, or the
probe is measuring nothing. If the burrow reads flat, the termite mechanism is
inert where nests are dug and §5's remedy changes from "move the coefficient
into the genome" to "fix the sensor, *then* move it".

Cheap, and it is the `check-that-a-planned-step-can-demonstrate-itself` question
that has cost this project a whole phase once.

**0b. What does an ant-sized eye cost?** `vision_probe mode=cost` at radius 16,
32 and 64 with ~50 animals. The existing figure (0.004 ms/frame at five
beetles at r64) is the right order but the wrong population. Note the standing
trap in that harness's own notes: the `locate` arm exists because a whole-world
scan **overstates the sense's cost thirtyfold**, so read `rN` minus `locate`,
never `vs blind`.

### Stage 1 — the instrument (§9)

Creature `selection_arena`. Ships with its own controls: `arm=same` mirrored
must be exactly 50.0% and is vacuous by construction; `arm=same` **unmirrored**
establishes the seed-driven spread against which everything else is read;
`arm=lethal` is the mandatory negative control and must be detected, or the
harness is blind.

Then **run it on today's ant** — the baseline, taken on the same binary, before
anything changes.

**The falsifier for the whole programme lives here.** If the bed does not
discriminate against a genome known to be worse, nothing downstream is
interpretable and the fix is the ecology, not the genome. The creature line has
now ended three times with that same finding in different costumes.

### Stage 2 — the kin sense, additive

Append `KinNear`, `KinBearing`; grow `BRAIN_HIDDEN` 4 → 8; relax the manifest to
prefix-compatible (§8); author `ant.ron` a `sight_fraction` and a short
`sight_range`. **Nothing is deleted in this stage.** The ant behaves as it does
today plus a sense it does not use, which makes the whole stage a pure
positive-control exercise: the new inputs must be non-zero where kin are and
zero where they are not, and `ascii`'s foraging scene must be unchanged in
`deliveries`.

### Stage 3 — the lab ancestor, and the deletions

A **new species file** — not an edit to `ant.ron` — with no `nest`, no
`nest_memory`, an authored odometer in hidden units, and a `MoistureGrad`
weight. In the same change, behind the species: delete the `recency` multiplier,
delete the `at_nest` drop branch, move the moisture coefficients to weights.

`ant.ron` authors the weights that reproduce its current behaviour, and
`ascii`'s foraging scene is the guard on that — **paired baseline run, not the
test suite**, because §2b records that the suite is blind to exactly this class
of edit.

**What says it worked:** the new ancestor aggregates and forages in the arena at
a rate distinguishable from a zero-connection control, over 12 seeds, read at an
order statistic. **What says it did not:** wanderers. Twelve seeds of animals
that never aggregate, never form a place, and never deliver anything to it. That
is a real possible outcome and the owner has accepted it.

### Stage 4 — armour and severing (§11)

**Independent of stages 2 and 3 and of the body revamp** — it touches the bite
and the damage rule, neither of which reads a body plan (§12b). It can be built
in parallel or slotted wherever it fits.

Three changes:

1. **`bite_force` on `CreatureDef`, defaulting to `dig_force`**, tested against
   the target cell's `penetration_resistance` in `act`'s ingest branch.
2. **Author `penetration_resistance` on the creature materials.** They are all
   at the 100.0 default today, which reads as impenetrable and is unread. `ant`
   soft, `chitin_mid` and `chitin_pale` harder, `beetle` harder still — with
   `body_energy` raised alongside, so armour is paid for in growth and is worth
   more as meat.
3. **`reconcile_chain` becomes a severing rule**: 8-connected walk from the
   vital cell; what is still attached lives, what is not becomes meat where it
   stands. Death is the vital cell taken, or energy at zero.

Also here, because it is one line and it converts a Kind 3 lever to a free
consequence (§11e): **derive `crop_capacity` from live cell count** rather than
authoring it.

**What says it worked:** a paired run at two `bite_force` settings must move
`meat_lost` and the standing population, and an armoured species must survive
measurably longer against the same predator. **The mandatory negative control**
is `bite_force` above every resistance in the table — that must reproduce
today's behaviour exactly, or the change did something other than what it says.

**What says it did not:** nothing moves, which would mean bites are so rare
that resistance never binds. Check `eats` and `meat_lost` are non-zero in the
baseline *before* building — a bite rule cannot matter in a world where nothing
bites, and S5a measured predation at 231 deaths over twelve seeds.

### Stage 5 — the threat sense (§11f)

The mirror of stage 2's kin sense, and cheap once that exists: a bearing to the
nearest animal **whose gut values me**. Same rays, same full-circle bearing.
This is what makes an arms race possible at all, and it should not be built
before stage 4 — a threat sense with nothing to fear is an input wired to a
constant.

### Stage 6 — the Kind 2 genes, once there is a working ancestor

`sight_range`, `tick_interval`, `bite_force`, and then S8's body axis after its
two pre-checks — **and after the body revamp, per §12a.** `CREATURE_TRAITS`
3 → 5 or 6, which is a lawful widen (a wrong-arity RON tuple **panics** with
the file position and both lengths — measured; the silent case is a
*misspelling*, not a widen).

### Stage 7 — crossover

`CROSS` on the shelf, and the heritable mutation rate beside it.

### Running alongside: `gene_probe`, the ratchet check (§3)

**Owner's call 2026-09-02: build it.** §3 is prose today, and `CLAUDE.md`'s own
recurrence audit is that a prose discipline does not survive a real session —
*"make it a command rather than a discipline"*, which is the finding that
produced `scripts/docbench.py selftest` after two blind controls were written
by an agent that had just finished writing the rule down.

**What it does, plainly:** you are about to make something evolvable. Run the
population at several settings of it and look at where the population mean
ends up. If it pins to the top of the range and stays there, the gene is not
expressing a choice — it is expressing that nothing charges for it, and it will
ratchet to its cap on the first generation and express nothing thereafter. If
the mean sits in the interior, the lever has two reachable ends and is worth
shipping.

**Shape**: `examples/gene_probe.rs gene=<name> range=lo,hi seeds=12`, reporting
the population mean per setting and flagging a pin. Two controls in the
binary, on the pattern every instrument here follows: a **known-good** lever
(`sensor_offset`, measured interior at 6 — 0.755/0.817/0.743/0.727) which must
read interior, and a **known-ratchet** lever (`sight_range` with
`sight_fraction` forced to 0, which is literally the state that shipped) which
must read pinned. Without both it cannot distinguish a quiet gene from a blind
probe.

It would have caught the plant architecture phase, `phototropism_dir`, and
E10's body-length premise. It is the positive-control rule applied to genes
rather than to measurements.

---

## 11. Predation, defence, and why size currently buys nothing

Added 2026-09-02 on the owner's question — *"how do creatures attack/defend"*
— and his ruling on the answer: **"yes, size (or other physical features)
should buy survival."**

### 11a. The encoding is good, and most of it should not be touched

**There is no predator and there is no prey.** Neither is a category anywhere
in the engine.

- `is_visible_prey` is: a cell of `MaterialKind::Creature`, not me, not living
  kin (unless `eats_kin`), whose `diet_yield` **against my own heritable gut**
  clears `EAT_YIELD_THRESHOLD` (12.0). So "prey" resolves to *anything my gut
  values*, which is a filter over data and a heritable one. A carnivore-leaning
  ant and a beetle looking at each other both read "food". Predation is the
  diet axis plus the ability to find things, and it is symmetric by
  construction.
- **Attack is the `Feed` verb.** There is no `Strike`; slot 12 is deliberately
  left unnamed (E13). A bite is `world.set(fxx, fyy, Cell::EMPTY)` plus
  `reconcile_chain` telling the victim.
- **`is_living_kin` is species identity only** — one line,
  `s.species == species`. It is not a relatedness model and does not pretend
  to be.

This is the mechanism/policy line working. Keep it.

### 11b. The finding: digging respects hardness, biting does not

**`creature.rs` reads `penetration_resistance` in exactly one place — the dig
branch (`creature.rs:2910`). The bite path never reads it at all.**

```rust
// dig, creature.rs:2910
if target.material != material::EMPTY
    && world.materials.get(target.material).penetration_resistance <= def.dig_force { ... }

// bite, act's ingest branch -- the whole test
if diet_yield(world, cell, gut.bias) > EAT_YIELD_THRESHOLD { ... }
```

So an ant with `dig_force: 1.0` **cannot dig sand** (`penetration_resistance`
1.4), cannot dig gravel (3.5) and cannot dig stone — and **can bite clean
through a beetle.** Flesh is the only substance in the world that offers no
resistance to being cut.

**And the armour axis already has a home in the material table, uniformly
unauthored.** `material.rs`'s `default_penetration_resistance()` returns
**100.0** — *"Impenetrable by default -- see `penetration_resistance`'s own doc
for why the safe default is 'no', not 'yes'"* — so every creature material is
already carrying a value that says "you cannot cut this", and nothing reads it.
`ant`, `chitin_pale`, `chitin_mid` and `beetle` are four distinct materials that
are **identical in every food property** (`food_energy: 480.0`,
`food_class: 1.0`, `density` 1.0/1.2) and identical in resistance because none
of them authored one.

This is `sight_fraction` again, one system over: **a field with a reader and no
writer on one side, and a writer with no reader on the other.**

### 11c. Recommendation R1 — a bite is a cut, and cuts already have a rule

**Route the bite through the test the dig already uses.** A bite removes the
target cell only if the attacker's force clears that cell's material's
`penetration_resistance`.

Why this and not a damage model:

- **Zero new concepts.** `force vs penetration_resistance` is the engine's
  universal "can this get through that" pattern — plant roots use it
  (`Behavior::Grow`'s `penetration_force`), digging uses it, and
  `dig_force`'s own doc already argues the case against the alternative:
  *"the pattern roots already use, **not** a material-name whitelist: a species
  that can chew soil but not stone should say so in force, so a future softer
  stone is diggable automatically."* Armour is the same sentence with flesh
  substituted for stone.
- **Armour becomes data, per cell, and therefore evolvable later.** A species
  whose cells are `chitin_mid` is harder to bite than one made of `ant`. That is
  a material choice a species file makes today and a genome can make once cell
  materials are heritable.
- **It is priced by an existing term.** `body_energy` is what a body cell costs
  to stamp and what it is worth as meat. Hard material costing more
  `body_energy` makes armour a real trade — slower to grow, more expensive to
  replace, and *worth more to whoever does get through it* — rather than a free
  win. **This is the Kind 3 → Kind 2 promotion (§3) and it is one number per
  material, not a new system.**
- **It gives the existing `dig_force` gene a second job**, which is how a
  burrower and a fighter become the same axis pulled in different directions —
  and how "attack" and "excavate" stay one verb rather than two.

**Name the attacker's term carefully.** Reusing `dig_force` verbatim couples
digging strength to biting strength permanently; a separate `bite_force`
defaulting to `dig_force` keeps them separable when that turns out to matter.
Recommend the second, defaulted, so nothing changes until a species says so.

### 11d. Recommendation R2 — a bite severs; it does not kill

Today, `reconcile_chain`: if the surviving cells no longer start with the
chain's first cell, *"head gone, the rest is meat"*, and the animal dies
outright. **One bite on the right cell kills a 2-cell ant and a 20-cell animal
identically.**

That is a binary outcome, and this project's own first law says an outcome is a
distribution rather than a binary — learned from destruction, stated in
`CLAUDE.md` as applying *"to every line in the engine"*, and independently
rediscovered on the plant line as graded death by `rot_remains`.

**The replacement: losing cells is damage, and what disconnects is severed.**

- Cells that lose **connectivity to the vital cell** are no longer part of the
  animal. They become meat where they stand.
- The animal lives on with what stays connected — smaller, and therefore
  slower, weaker, and **cheaper to run**, since `idle_cost_per_cell` and
  `move_cost_per_cell` already charge the live body rather than the authored
  one. *An animal that has lost a cell to a predator burns less* is already
  true in the tree and fell out of reading the right quantity.
- Death is then two things, both already in the engine's vocabulary: the vital
  cell itself is taken, or energy reaches zero.

**Why this is the right shape rather than hit points.** Hit points are the
authored-game answer — a number with no physical referent, attached to the
animal rather than to its parts. Severing is the same mechanism the engine
already runs everywhere else: a connectivity check over a body whose parts can
be removed, which is exactly how a plant discovers it has lost a leaf
(`reconcile_chain`'s own comment: *"A plant finds out it has lost a cell
through its own connectivity check, which is exactly what made herbivory need
no new code"*), and exactly how structural collapse decides what falls.

It is also **satisfying in the house sense**: the second law is *there must be
a verb and it must deliver something*, and what a successful attack delivers is
**a piece**. A limb coming off is visible; an animal blinking out of existence
is not.

**One constraint travelling with it**, from `CLAUDE.md`: *a traversal must use
the same neighbourhood the writer used.* Body cells are placed at 8 neighbours,
so the connectivity walk is 8-connected or it will sever animals that are
perfectly intact.

### 11e. What this buys size, which is the owner's actual question

With R1 and R2 in place, size stops being a pure cost:

| | before | after |
|---|---|---|
| more cells | more meat for the attacker, more upkeep, **no survival value** | more cells that are not the vital one — the fatal target is diluted |
| a lost cell | fatal if it was the head, free otherwise | graded: a piece comes off, the animal continues diminished |
| hard cells | nothing — resistance unread | genuinely harder to cut, paid for in `body_energy` |

**And a second benefit of size should be taken while we are here, because it
converts a Kind 3 lever into a free consequence (§3):** derive
`crop_capacity` from body size rather than authoring it. A bigger animal
carries more. That removes a gene that would otherwise need its own cost term,
and it gives size a *foraging* payoff to sit beside the survival one — so
`body size` becomes a real trade with reachable ends rather than a ratchet in
either direction.

### 11f. The gap that blocks an arms race, and it is one-sided

**A predator has an input for finding prey. Nothing has an input for detecting
a predator.** `PreyNear`/`PreyBearing` report *food*, not *danger*, and
`sight_range` defaults to 0, so **an ant cannot perceive a beetle at any
distance whatsoever** — only on contact, and then as food.

Fleeing is not a missing verb; `Turn` and `Move` away are fleeing. It is a
missing **sense** to trigger on. And `creature-evolution-plan.md` §7 names an
arms race as *"the standard engine of open-ended dynamics, and this world has
never run one"* — which cannot start while only one side can see.

**The fix is the same slot pair as the nest work, evaluated the other way
round.** §4 adds a bearing to the nearest animal *my gut values*; the mirror is
a bearing to the nearest animal *whose gut values me*. Same rays, same
full-circle bearing, no new category, no fear pheromone (which is gated behind
the measured 0.5 ms third-plane cost anyway). Pursuit and evasion then differ
by the sign of one weight.

**Note what S5a already measured before anyone builds on this**: predation
today punishes neither ranging nor sheltering, mortality is biased *toward*
home in both arms, and the two shelter tables disagree about the sign of the
beetle term at 231 deaths. Shelter pays enormously and predators are not what
makes it pay. So R1, R2 and the threat sense are **preconditions** for
predation having teeth, not refinements of a working system.

---

## 12. The body-plan interface contract

The owner, 2026-09-02: *"your current analysis of this partial relies on the
current body design but I would like to fully revamp that, so either we need to
do that concurrently or plan something flexible."*

### 12a. Recommendation: flexible, not concurrent

**Do not run the body revamp alongside this work.** Three reasons, and the
third is the one that has cost this project real time:

1. **The revamp is unscoped.** What the body should *become* has not been
   stated, so a concurrent plan would be planning against an unknown.
2. **Mind before body is the owner's own sequencing call**, made 2026-09-02
   and unretracted.
3. **Two changes reallocating one budget cannot be read apart.** This is
   `CLAUDE.md`'s shared-budget rule and
   `why-changes-cost-so-much-2026-08-27.md`'s whole subject: the moment two
   changes move the same quantity, every measurement of either is confounded
   and the constants calibrated against the old behaviour are being re-derived
   twice against a moving target. Body and metabolism are the *same* budget —
   `idle_cost_per_cell`, `move_cost_per_cell`, `body_energy`, `crop_capacity`.

**Flexibility here is cheap rather than a compromise**, because nothing this
document proposes needs to know what a body looks like. That is worth checking
explicitly rather than asserting, which is what 12b is.

### 12b. What the rest of the engine may ask a body

State the contract, build against it, and the revamp becomes a swap behind it.
Everything in this document uses **only** these:

| the engine asks | used by | body-plan dependent? |
|---|---|---|
| **its cells**, as world positions | metabolism, rendering, damage | no — a set of positions |
| **one vital cell** | death (§11d) | no, *provided a body plan designates one* |
| **the connectivity neighbourhood** (8) | severing (§11d) | no |
| **each cell's material** | armour (§11c), meat value | no |
| **a cell count** | `idle_cost_per_cell`, `move_cost_per_cell`, `crop_capacity` (§11e) | no |
| **a movement rule** | stepping, passability | **yes — and nothing here touches it** |

Only the last row is genuinely body-plan-specific, and it is the one row this
document never reads. `BodyPlan` today is `Chain(u8)` or `Rigid(Vec<(i8,i8)>)`
— *"two movement rules, not one with a parameter"* — and the mechanisms
proposed in §4, §11c and §11d are indifferent to which.

### 12c. The one thing the revamp must honour

**Any new body plan must designate exactly one vital cell.** Both current plans
already do — `offsets()` returns the head first, `(0,0)` is the head in a
`Rigid` and is implicit — so this is a contract the code already satisfies
rather than a new requirement. It is written down here because §11d's death
rule is the only thing in this plan that depends on it, and a revamp that
quietly dropped the notion would turn "death" into an undefined question rather
than a failing build.

If the revamp wants *several* vital cells (a redundant nervous system, a
creature that survives decapitation), that is a strict generalisation and it
only makes §11d more graded. It is not a conflict; it is a later gene.

### 12d. What the revamp should be told about, from this document

Three findings that a body revamp will otherwise rediscover the expensive way:

- **Body length is no longer unpriced** (§7). Both cost paths went per-cell on
  2026-08-30. The coordinator note's blocker is stale.
- **Size must buy something or it is a ratchet in reverse** (§11e). Today it
  buys more meat for the attacker and nothing else, so an unguided body gene
  would shrink rather than grow.
- **Composition, not architecture, sets the silhouette** (§7's appearance
  paragraph). Three plant levers all fired and moved no pixel; the creature
  line then got *"It is a perfect cube"*. A revamp that only changes **which
  cell gets a label** will land in the same place.

---

## 13. The body: why growing a creature makes a worm or a brick

The owner, 2026-09-02, asked for his single biggest issue with the body:

> *"My biggest issue is the visual. I don't want all [creatures] to look like a
> chain. That is not interesting, and so larger creatures just become
> snakes/worms. There are lots of other interesting things that could be done,
> but that is my number 1 issue."*

### 13a. This is already written down in the engine, in the same words

`BodyPlan::scaled`'s doc comment:

> *A `Rigid` plan supersamples and a `Chain` can only stretch, and the
> asymmetry is the plan's, not this function's. A rigid cell is an area, so at
> `k` it becomes the `k`x`k` block it covers and the silhouette is preserved
> exactly. A chain is a path... So `Chain(n)` scales to `Chain(n*k)`, which is
> the right physical length and still one cell wide. **A chain cannot be made
> physically identical at a finer resolution, and that is not a bug to fix
> here: it is the reason the owner's "creatures should be more than chains of
> pixels" and the resolution step are the same piece of work.**

So the complaint is not new and it is not an oversight. It is a **property of
having exactly two body plans**, and both of them fail to gain structure with
size in opposite directions:

| | scaled up | what you get |
|---|---|---|
| `Chain(n)` | `Chain(n·k)` — stretches | a longer worm, still one cell wide |
| `Rigid(cells)` | each cell becomes a `k`×`k` block | the same silhouette, bigger. `ant_block`'s 3×3 becomes a 6×6 — **the "perfect cube"** |

The owner's verdict on the 36-cell creature — *"Shape. It is a perfect cube.
Are there perfect cube creatures in our world?"* — is literally the second row.

### 13b. And shape currently costs an order of magnitude of mobility

The reason nobody has simply authored better shapes is measured, and it is the
real constraint:

**A rigid body is blocked 25–43% of its moves; a chain 2–6%.** That survives
all three trees it was measured on and no reshuffling closes it
(`creature-appearance-design.md` §4 — note that the *within*-rigid ranking in
that section is explicitly withdrawn; only the coarse gap survives).

`creature-body-extent-2026-08-30.md` adds the other half: **at the shipped seed
and horizon, no chain longer than two cells leaves a living colony** — at
three, four, six or nine cells, and at the old flat bill as much as the new
per-cell one, reproducing on a flat slab so it is not terrain.

So today: **a body with an interesting outline cannot move, and a body that
moves cannot have an outline.** Any answer to D1 has to break that trade rather
than pick a side of it.

### 13c. The proposal: an articulated body, which is one representation with both current plans as its ends

**A body is a short chain of *parts*. Each part is a small rigid shape. Each
part follows the path of the part ahead of it, exactly as a chain cell follows
the head.**

- `Chain(n)` is the degenerate case where every part is a single cell.
- `Rigid(cells)` is the degenerate case with exactly one part.

So this is not a third body plan beside two others — it is **the
generalisation the two existing plans are already the endpoints of**, which is
the kind of unification this codebase prefers and is why it is worth doing
rather than adding `Blob` next to `Chain`.

**Why it should recover the mobility.** Passability is checked **per part**, and
a part moves into ground the part ahead has already vacated and proven passable.
So a 2×2 part meets the rigid-body problem at 2×2 scale rather than at
whole-body scale. **Predicted to land between the chain's 2–6% and the rigid
body's 25–43%, much nearer the chain.** That is a prediction, not a premise —
see 13e.

**Why it should recover the silhouette.** Parts may differ in size and shape, so
a body gets a **waist, a taper and a head that is not the same as the abdomen** —
which is precisely what a uniform chain and a uniform block both lack. And it
scales correctly: at `k` each part supersamples, so a physically identical
animal keeps its **proportions** instead of becoming a longer worm.

**Why it is the right thing for evolution rather than only for authoring.** A
part list is a small, bounded, continuous-ish genome: how many parts, and each
part's extent. That is `body_of(segments, girth)` from
`creature-evolution-plan.md` §2.8 — *"continuous genes, discrete phenotype…
disconnected, self-overlapping and 4095-cell bodies are unrepresentable rather
than merely rejected"* — with the connectivity guarantee falling out for free,
because a chain of parts is connected by construction.

### 13d. The honest caveat, and it is the one that has cost this project a phase

**`creature-appearance-design.md` §4 measured shape at constant extent moving
nothing.** Two nine-cell bodies — a filled 3×3 and a waisted 5×2 insect
outline — came out **0.8% apart on ink and inside the noise on contrast**, on
all three trees. Its §1 states the conclusion flatly: *"Extent is the only
lever."*

Read carelessly, that says D1 is unreachable and articulation is the plant
line's three architectural levers all over again — built, fired, moved no pixel.

**It does not say that, and the distinction is the most important thing in this
section.** `creature_look`'s numbers — `ink`, `|contrast|`, `decoys` — measure
**findability**: can you locate the animal against a textured world. The owner's
complaint is not that he cannot find it. It is *what it is once he has found
it*, and his own verdict — **"it is a perfect cube"**, at 36 cells — is a
shape reading, delivered by eye, on a body large enough for shape to register.

So the two are not in conflict; they are at different sizes and about different
questions. The synthesis, stated as a claim that could be wrong:

> **Shape is below the noise at 9 cells and legible at 36.** The appearance
> report's 0.8% and the owner's "perfect cube" are the same axis measured
> either side of the threshold, and the resolution step (#179/#181, cell
> density doubled, *"the direction, not an experiment"*) is what moved a
> physically ant-sized animal across it.

**And the consequence for how this gets judged: there is no instrument in this
repository that measures "does this read as an animal rather than a smudge".**
Every appearance number here answers *can it be seen*. That gap is exactly how
a shape lever fires and is judged as nothing, which is the failure
`plant-appearance-design.md` records costing a whole phase.

**Therefore D1's verdict comes from the review queue, not from a metric** —
`review.py ab --blind`, rendered sheets of candidate bodies, the owner's eye.
That is not a fallback; `CLAUDE.md` requires it (*"post rather than describe…
you are choosing between approaches and the difference is visual: post a blind
A/B"*), and it must happen **before** the lever is built, not after.

### 13e. The pre-checks, before any of this is built

Three, and they are cheap because two of the instruments exist.

1. **Does anyone want these shapes?** Render candidate articulated bodies —
   3 parts vs 5, uniform vs waisted vs tapered — at the shipped resolution and
   post a blind A/B. `creature_scale mode=size` already renders one body per
   panel cropped to fixed *physical* units. **If the owner cannot tell them
   apart, the lever is below threshold and the answer is extent, not
   articulation.** This is the *check-that-a-planned-step-can-demonstrate-itself*
   question, asked first this time.
2. **Does it actually move?** `creature_scale mode=walk` is the body-plan
   mobility instrument and carries a standing positive control: `Chain(2)` must
   reproduce **5.2%**, or the run is measuring something else. Articulated
   bodies must land nearer that than the rigid 25–43%. **If they do not, the
   trade in 13b was not broken and the proposal fails.**
3. **Can a colony of them live?** `creature-body-extent-2026-08-30.md`'s finding
   is that no chain past two cells leaves a living colony — at *any* pricing. A
   bigger body is not blocked on cost, and articulation does not by itself fix
   whatever that is. **Run `body=` across the articulated plans before treating
   any of this as shippable**, and if the colony still dies, that is an ecology
   problem to solve before a body problem.

### 13f. What this changes in §12's contract

Nothing is retracted. Articulation touches exactly the one row §12b marked as
body-plan-specific and that nothing else in this plan reads:

| the engine asks | under articulation |
|---|---|
| its cells | unchanged — still a set of positions |
| one vital cell | unchanged — the head of the first part |
| connectivity (8) | unchanged, and **stronger**: a part chain is connected by construction, so §11d's severing rule gets a natural cut line at a joint |
| each cell's material | unchanged, and **now more useful**: parts can differ in material, so armour can be *local* — a hard head, a soft abdomen |
| a cell count | unchanged |
| **a movement rule** | **this is the row that changes**, and it is the only one |

So §11's armour and severing can be built before, during or after the body
work, and §4's kin sense is untouched. **The sequencing in §10 does not need to
change.** That is the flexibility §12a was arguing for, now tested against a
concrete revamp rather than asserted.

---

## 14. What not to re-derive

- **Whether `store_in_body` needs a slot.** It does not; the `Feed`/`Drop`
  weights already express both forks and are conditioned on everything the brain
  senses, which a scalar trait is not.
- **Whether `reproduce_at` needs senescence first.** It does not; starvation
  already makes the axis two-sided, because `place_creature` tops a parent to
  `birth_cost + 1` and then charges `birth_cost`, leaving an eager breeder on
  about three ticks of upkeep.
- **Whether the body-size refuge exists.** It does, and it is tested by name:
  `a_wide_body_cannot_enter_a_one_cell_tunnel_that_a_chain_walks_through`. There
  is no hiding code anywhere; a rigid body's passability check covers every cell
  of it.
- **Whether the ant's metabolic costs are the lab bed's binding constraint.**
  They are not. Income is 5–9× outgo while an ant has food, and the 52 → 12
  crash has a control attributing it to **overgrazing**: the four founder
  columns nearest the nest die and the four furthest are untouched to the cell
  (99 vs 105, 69 vs 69, 54 vs 54). The remedy is spatial.
- **Whether topology should evolve (NEAT).** D4, decided. The reserve and the
  crossover argument both depend on the cage.
- **Whether predation currently selects for shelter.** It does not — mortality
  is biased *toward* home in both arms and the two shelter tables disagree about
  the sign of the beetle term at 231 deaths. Shelter pays enormously and
  predators are not what makes it pay.

---

## 15. The honest risk

The change cannot lose the ant. Three independent guarantees: the positional law
forbids removing `AtNest`; every code-side coefficient becomes an authored
weight in a file rather than a deleted mechanism; and the deletions land behind
a new species rather than as an edit to `ant.ron`.

**The risk is that the de-hardcoded ancestor does nothing interesting.** Nothing
in this document guarantees that aggregation emerges, that a place forms, or
that anything resembling central-place foraging appears when the scene stops
painting its precondition. The evidence that it *might* is that the genome space
is measurably not degenerate — 400 random genomes across 8 seeds cover 26 of 81
behaviour cells, span survival 0.103–0.541, produce at least three distinct
successful strategies, and **the hand-authored ant is beaten by a random
genome**. Selection has real gradient to act on. Whether it has gradient toward
*a home* is exactly the question, and it is not currently answerable from
anything measured.

Stage 1 is what makes the answer readable either way. Without it, a null is
uninterpretable and the session produces an opinion.
