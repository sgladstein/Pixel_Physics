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
| **A rigid body's cells become state** | *"fix it properly as part of the body work"* — not a special case in §11d. Track B, stage B2 |
| **`relocate_chain`'s bug is stage one of the body work** | *"for now it can be stage one"* — Track B, stage B1 |

---

## How to use this report

**If you are the implementing agent, this is your entry point.**

**Read in this order.** §0 (the one recommendation), §2 (what is actually
authored, with line numbers), §3 (the frame that decides whether a gene is safe
to build), then **§10, which is the implementation brief** — two tracks, staged,
each with its acceptance number and its falsifier. Everything else is the
argument behind those.

**What is decided and not open to re-litigation:** the table above. Those are
the owner's rulings, given in conversation on 2026-09-02.

**What is measured, what is assumed.** Claims in this document that were
verified against source carry a `file:line`. Claims that are predictions are
labelled as such — §13c's mobility argument and §13d's shape-threshold claim are
the two that matter, and both have a named pre-check in §10 Track B. **Do not
promote either to a premise without running its check first.**

**What was wrong in the first draft, and why that matters to you:** §16. An
independent review found four confirmed errors, one of which would have stopped
every animal in the world from eating on the first run. They are corrected in
place. §16 exists because the *shape* of those errors is the most reusable thing
here — three of the four came from checking a claim against a neighbouring file
instead of the actual one.

**Before you build anything**, the three rules at the top of §10 govern every
stage: re-take a baseline on the binary you are comparing against, never move
the metabolic budget in parallel with an arena reading, and carry a positive
control rather than only a negative one.

**Two things this report does not authorise.** It does not authorise the body
revamp's *design* — Track B stops at articulation-if-the-pre-checks-support-it,
and §13c records what articulation demonstrably does not buy. And it does not
authorise skipping §0: with the seed alone moving the lab census 2.42×–3.12×,
a result measured before the arena exists is not a result.

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

**A caveat on that exemplar, from the review.** `sensor_offset`'s numbers come
from `pheromone.rs`'s `trail_following_sweep`, an **`#[ignore]`d unit test**
running a bespoke follower on a hand-built trail over **six seeds** — and
`CLAUDE.md` states flatly that *six seeds is not a sweep* (1.64× over six, 1.08×
over the next twelve, pooled median zero). It also measures a *steering score in
a synthetic harness*, not a population mean under selection. So it is a
**plausible** Kind 1 example, not a measured one, and §10 no longer uses it as
`gene_probe`'s reference control.

### Kind 2 — priced *and* self-limiting. Safe now.

A cost term exists, scales with the lever, **and the optimum it produces is
interior.**

**Both halves, and the first version of this document had only the first — which
is the bug in the frame that the independent review found.** "A cost exists and
rises with the gene" is not sufficient, because a cost that rises *linearly*
against a benefit that also rises linearly produces no interior optimum at all:
the ratio is constant and the gene pins at whichever end is marginally better.
`tick_interval` is exactly that case and the first version classified it as safe
(§7).

**The two-part test:**

1. *Name the line that charges for it, and show the charge rises with the gene.*
   If you cannot name the line, it is Kind 3.
2. *Then show the return diminishes or the cost accelerates* — that the benefit
   saturates, or the cost grows faster than linearly. If both are linear, the
   gene is a ratchet **even though it is priced**, and it belongs in Kind 3 until
   something bends one of the two curves.

Step 2 is the same question Kind 1 asks (*can I state a setting at which more of
this is worse?*); pricing does not exempt a lever from having to answer it.
`gene_probe` (§10) is step 2 made mechanical.

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

## 5. The moisture gradient — what it is actually reading, and why that changes the recommendation

**Revised 2026-09-02.** The first version of this section recommended keeping
this channel on the grounds that it implements the termite construction result —
deposition tracking evaporation flux, which tracks surface curvature. **It does
not, and that was measured rather than argued.** `examples/field_sense_probe.rs`
(PR #214, Stage 0a) found curvature does not move this channel at any sampling
span. The recommendation changes; the *argument* behind it mostly survives, and
§5f says which half is which.

`creature.rs`:

```rust
pub fn moisture_gradient(world: &World, x: i32, y: i32) -> f32 {
    let m = |px, py| world.field_at_bilinear(px as f32, py as f32).moisture;
    let gx = m(x + 4, y) - m(x - 4, y);
    let gy = m(x, y + 4) - m(x, y - 4);
    ((gx * gx + gy * gy).sqrt() / WORM_MOISTURE_SATURATION).clamp(0.0, 1.0)
}
```

Food drop probability away from home is multiplied by this; dig probability by
its inverse.

### 5a. The design intent, which is good and is worth keeping on the page

Source is [`stigmergy-research.md`](stigmergy-research.md) §4 on Facchini et
al., *eLife* 2024. The classical model of termite construction — Deneubourg
1977 and every agent model after it — assumes a **cement pheromone**: a marker
added to deposited material that stimulates further deposition nearby. That
study ran *Coptotermes gestroi* on clay arenas with pellets **sterilised to
remove chemical marking**, tracking collection and deposition separately:

1. **Collection is uniform across the arena; deposition is concentrated.** That
   asymmetry is the whole algorithm.
2. Every deposition region was a local maximum in **evaporation flux** — which
   is provably proportional to local surface curvature.
3. Convex regions attract deposition; concave regions attract digging.
4. A salt-solution control with **no termites** deposited salt in precisely the
   regions where termites had built.

This is the right kind of mechanism for this engine: it asserts a *physical
fact* and lets the outcome emerge, rather than asserting the outcome. Nothing
below retracts that. What is retracted is the claim that the shipped channel
implements it.

### 5b. The measurement: curvature does not move this channel at any span

`field_sense_probe`'s control bed holds three curvatures **in one elevation
band**, because an earlier version compared a crest against a plain 30 rows
lower and could not say which of the two it was reading. Convex crest against
flat plateau at the same elevation:

| span | crest | flat | ratio |
|---|---|---|---|
| ±4 (shipped) | 0.1746 | 0.1724 | **1.012x** |
| ±8 | 0.3652 | 0.3613 | 1.011x |
| ±16 | 0.7777 | 0.7732 | 1.006x |
| ±24 | 1.2087 | 1.2047 | 1.003x |

**Widening the sampler moves the ratio *toward* 1.0.** The probe's own positive
control passes — depth moves the reading 1.74x — so this is a statement about
the channel and not about the probe. `dy` runs 0.17–0.33 while `dx` is 0.0009 at
the crest and 0.0000 at the notch.

**Do not put a bar on the `dx` column**: an earlier version did and reported a
clean 0.09x separation, which is an artifact twice over — a symmetric ridge's
apex has `dx = 0` by symmetry, and the flat reference sat near its plateau's
edge.

### 5c. What it is actually reading: an interface detector

*(This reading is inferred from `field.rs` rather than separately measured; the
numbers above are what it explains.)*

`apply_moisture_sources` makes **damp soil and standing liquid moisture
sources** — `moisture_source` is `held / water_capacity` for soil, full for
liquid. Air carries moisture only by diffusion, and diffusion is gated on
`blocked`, which `rebuild_blocked` sets for a whole block if one cell in it is
`Solid`.

So the field is high in and at damp ground, decays upward into air, and is
near-uniform deep inside the ground. `|∇moisture|` therefore peaks **at the
air/ground interface** and falls toward zero in both directions — deep
underground, and high in open air.

**It is a surface-proximity detector.** Depth is simply the axis along which it
varies, which is why depth moved it 1.74x.

Read the two shipped rules with that substituted in:

| rule | what it actually says |
|---|---|
| food drop away from home = `drop_urge × surface_proximity` | drop what you are carrying **when you surface** |
| dig = `dig_urge × (1 − surface_proximity)` | dig **once you are already inside the ground** |

That is a coherent hauling rule and arguably a good one — roughly *carry the
spoil out and dump it at the mouth*. It is not stigmergy, and it is not
Facchini.

### 5d. Why it structurally cannot produce architecture

**This is the part that matters more than the mis-citation.**

Facchini's mechanism is **self-amplifying**: deposit → raises local curvature →
curvature attracts more deposition → a pillar grows. The positive feedback is
what produces pillars, walls and chambers.

A surface detector is **self-neutralising**: deposit at the surface, and the new
surface reads the same value the old one did. Nothing accumulates an advantage.
And a placed pellet is `packedsoil` with `water_capacity: 1000` carrying its own
moisture, so it re-sources the field and remains a surface.

**No positive feedback means accretion, not architecture.** So the old §5's
*"pillars, walls and chambers are consequences of that bias"* is not merely
mis-sourced — it is **unreachable with this signal, at any coefficient, in any
genome.** No amount of moving the response into the brain fixes that, because
the fault is in the signal rather than in the response to it.

### 5e. A hypothesis about the guard, with the control named

**Unmeasured. Stated so it is tested rather than assumed.**

`ascii`'s deposition scene measures mean `|∇m|` at the cells ants dropped on
against the mean over a 12-row band — 2.94x before this work, 2.97x after. Its
own comment records that an earlier version was replaced because it *"passed
harder for the broken build"*.

But drops land where an ant is standing, which is a surface, and surfaces are
exactly where `|∇m|` is high; the band average includes deep air and buried
soil, which are both near zero. **So the ratio may read ~3x purely because
drops happen at surfaces, with no contribution from the moisture term at all.**

**The control is one run**: delete the moisture term from the drop probability
and re-measure *the ratio* — not the counts, which is what the previous version
got wrong. If the ratio stays near 2.9x, the guard is measuring "ants drop on
surfaces" and the rewrite traded one blind guard for another.

### 5f. What survives, what dies, and the recommendation

**Survives, and gets stronger — and it has now shipped.** The channel is still
a real field the world computes rather than an authored outcome, and moving the
response coefficient into the genome was still right — more right, because a
fixed coefficient was encoding a rule nobody had correctly identified. **Done in
PR #214**: the coefficient is `BrainInput::MoistureGrad`, and `ant.ron` authors
`(MoistureGrad, Dig, -0.55)` and `(MoistureGrad, Drop, 0.169)` — a weight with a
free sign and magnitude where there was a fixed multiplier. That was this section's
actual argument and the measurement does not touch it. `dead-ends.md` entry 983
prescribes the same move in general terms: *"it wants to be a wired instinct on
its own brain output rather than a coefficient in `act`, so a lineage can lose
it."*

**Dies.** The claim about what the channel produces; Facchini as a description
of the shipped mechanism (it stays as the design *intent*); and the name —
`moisture_gradient` is accurate about the arithmetic and misleading about the
meaning.

**Also retired: the first version's "dated finding".** It reported that the ±4
offsets predate `FIELD_SCALE` doubling 8 → 16 on 2026-08-30 (`ca7e9042`, one day
after `fac79156` last touched those lines) and proposed re-deriving them. **The
dating is correct and the remedy is wrong**: §5b shows widening the span moves
the ratio toward 1.0. Recorded rather than deleted, because the *shape* of the
error is the one `CLAUDE.md` names — a constant calibrated against a quantity
that then moved is worth suspecting, and here it was worth suspecting and was
not the fault.

#### The decision: build the curvature signal

**Owner's call, 2026-09-02, and two of the three objections against it were
withdrawn under his challenge.** They are recorded because the *withdrawals*
are the useful part.

**Withdrawn 1 — "the ecology is not ready" is an ordering argument, not a
don't-do-it.** It was presented as the latter. Building the sense is also how
you find out whether the ecology needs changing.

**Withdrawn 2 — "nothing selects for shelter" is wrong, and the data says so
in the column nobody read.** `predation_probe mode=range`, 12 seeds, paired
against `beetles=0`:

| | no predator | beetles | advantage |
|---|---|---|---|
| roofed vs open | 0.59 / 1.36 | 0.54 / 1.41 | 2.31x → 2.61x |
| predator cannot fit vs could reach | 0.43 / 1.22 | 0.56 / 1.18 | 2.84x → 2.11x |

**Shelter pays 2.3–2.8x with no predator in the world at all.** The two tables
disagree about the *sign* of the beetle term, and at 231 deaths with beetles
against 190 without — about 3.4 extra deaths per seed — predation's
contribution is inside the noise. So the correct statement is the narrow one:
*predation* does not select for shelter, and something else does, strongly.
**The gradient a builder would climb already exists.** What is missing is not a
reason to shelter but the ability to make shelter of a good shape, which is
what this signal is for.

**Not withdrawn, but corrected — the visibility concern.** The worry was that a
three-cell pillar sits below the world's 1–2 cell texture grain, on
`creature-appearance-design.md` §2's decoy counts (127 at 2 cells, 15 at 9, 0
at 16). That finding is real and it was **over-extended here**, three ways: it
measures *finding an animal* rather than *reading as built*, and what reads as
built is **regularity and repetition**, which `decoys` scores independently and
therefore cannot see; the resolution step (#179/#181) doubled cell density, so
the old three-cell feature is six; and structure **accumulates over a run**,
where `motion_look` found a changing feature has 0–2 competitors against a still
one's 141. The concern survives as a question, not as a reason to wait.

**So: build it.**

1. **`SurfaceCurvature` as a brain input** — signed, from a solid-neighbour
   count in a disc (negative concave, positive convex), opt-in per species the
   way `sight_range` is.
2. **Rename the existing channel** to what it measures and keep it. The
   *response* is already a weight (above); what is still misnamed is the
   *signal*. Two honestly-named signals, weighed independently, rather than one
   mislabelled one.
3. **Author the ant and the ancestor a starting weight**, so generation zero
   carries the termite bias *for real* — which it has never actually had.
4. **Feed both drop genes independently.** PR #214 split `Drop` and
   `DropSpoil`, so a lineage can evolve to build with spoil and cache food on
   different criteria. That falls out for free and is a better outcome than one
   coefficient ever allowed.

**One check, folded into the build rather than gating it:** render the bank with
the bias forced on and off, blind A/B. Not permission to proceed — the thing
that distinguishes "it works" from "it works and needs to be bigger". If the
arms are indistinguishable the answer is extent, which is the same answer as the
creature-silhouette work and not a reason to have skipped this.

### 5g. The placement predicate, and why it should not change in the same step

`act`'s spoil drop admits a cell only if it is empty, **at least two of the
three cells directly beneath are solid**, and `SPOIL_HEADROOM = 3` cells above
are clear. Both clauses are measured: without the footing, censused floating
ground runs **26–38** pieces with no path to the floor against **2–6** with it;
without the headroom, `burrow_probe arms=colony` reads roofed void **2–4**
against **89–139** — the colony backfills its own nest.

**The footing clause is specifically anti-pillar.** On a one-cell-wide pillar
top the cells beneath are empty / solid / empty, so the count is 1 and the site
fails. It went from "one beneath" to "two of three" because unmotivated stacking
grew *"thin vertical fingers, which a rendered sheet shows plainly."*

**And a finger and a pillar are the same geometry.** One is noise, the other is
curvature-driven deposition, and the predicate cannot see the difference. That
is exactly `CLAUDE.md`'s *when a rule must tell apart two things that can look
identical, state the difference as data* — the rule four successive support
models learned by failing, each either strong enough to hold a mountain or weak
enough to let a player's tower break, because geometry cannot distinguish a
mountain from a wall someone stacked. **Relaxing 2 to 1 is that mistake in a new
costume, and it should not ride along with the curvature change**: two edits to
one outcome cannot be attributed apart.

**So: leave the predicate alone in this step.** Add curvature within it and see
what it does to the *wide* features it can already shape — mound profile, ridge
thickening, hollow filling. If those read as built, narrow pillars may not be
wanted at all. What distinguishes architecture from noise is regularity, and
regularity would come from the bias being consistent rather than from the
placement rule.

**If pillars specifically turn out to be wanted, the change is one line with its
acceptance bar already in the tree:** relax the footing to one cell beneath and
re-run the floating-ground census against its measured 2–6 / 26–38.

#### The clause that actually binds is the headroom, and nobody has said so

`SPOIL_HEADROOM` is 3, and its own comment records that *"a gallery cut by
`line_burrow` is one to three cells tall"*. So inside a gallery there is never
three cells of clear air above the floor, and **a pellet can never be placed
inside a burrow.**

That is deliberate and it is load-bearing — it is what stops the colony
backfilling its own workings. But the consequence has not been stated anywhere:
**all construction is necessarily external.** Mounds, banks and ridges are
reachable; partitions, chamber walls and any thickening from inside are not. No
curvature signal changes that, because the site is refused before preference is
consulted.

**So if "ants building homes" means rooms rather than mounds, the headroom
clause is the binding constraint and the curvature work will not reach it.**
And unlike the footing clause, this one *can* be restated as data rather than
geometry: a pellet may go inside a burrow when it is **against a wall** —
adjacent to solid on a side, which is what a lining is — as opposed to standing
in open gallery space, which is backfill. That is a different question from
"clear air above" and it would admit internal structure without reopening the
failure the headroom clause was built for. **Not proposed here**; recorded so
the next person asking for chambers knows which rule to argue with.

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
| the moisture response (§5) | a sign *and* a magnitude; both extremes are bad. **But see §5c** — the channel is a surface detector, so the response is to depth rather than to curvature, and the lever is narrower than this row assumed |
| all steering weights | already heritable; listed for completeness |

### Kind 2 — already priced, safe now

| gene | the line that charges for it |
|---|---|
| **`sight_range`** | `sight_tax = sight_fraction × start_energy × sight_reads` — rises with radius through cells read. Author the fraction first (§6). |
| ~~`tick_interval`~~ | **withdrawn — see below.** Priced, but not self-limiting |
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

**`tick_interval` is withdrawn from this table.** The pricing claim is correct —
every cost is charged once per creature tick (`creature.rs:1822`), so halving
the interval doubles the bill per frame. But it fails Kind 2's *second* test
(§3): cost and benefit both scale linearly with tick rate, so there is no
interior optimum and the gene pins at the floor, `tick_interval: 1` — which is
the ratchet the frame exists to prevent.

**It is also a frame-cost lever nobody named.** `tick_interval` sets how often a
creature's active site is rescheduled; 52 ants at 1 instead of 6 is six times
the creature scheduling work per frame. `CLAUDE.md` requires a proposal to say
what it costs the frame, and the first version of this table was silent. It
belongs in Kind 3 until either the benefit saturates or the cost accelerates.

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
  digests the contents as it walks. **Do not make it a gene — and, revised after
  review, do not derive it from body size either.** §11e does the arithmetic the
  first version skipped: deriving capacity from cell count makes foraging cost
  per delivered cell **independent of body size**, so it is size-*neutral*
  rather than a payoff; it silently re-normalises `BrainInput::Carrying` for
  every existing genome; and it is denominated in joules, not cells, so it needs
  a joules-per-body-cell constant nobody has derived. **Keep it authored.**
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

Concretely, and **corrected after review, because the obvious form of this rule
does not permit the change it was introduced for**: `genome_manifest()`
(`brain.rs:211`) hashes **seven dimensions** — `BRAIN_INPUTS`, `BRAIN_OUTPUTS`,
`BRAIN_HIDDEN`, `INPUT_SLOTS`, `OUTPUT_SLOTS`, `HIDDEN_SLOTS`, `GENOME_LEN` —
*and then* the ordered names. Stage 2 grows `BRAIN_HIDDEN` 4 → 8, and **hidden
units have no names**; they are positional, as `specimen.rs` states outright
(*"name-addressed on inputs and outputs and positional on hidden units"*). A
name-prefix rule alone therefore still rejects every jar on precisely the change
it was written for.

**The rule that works has two clauses:** a stored jar loads when its name lists
are a **prefix** of the current ones **and** every stored dimension is
**less than or equal to** the current one. Rename, reorder or shrink still
refuse — which is what the manifest exists for — and a lawful append in any of
the three directions becomes a non-event.

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

**This section is the implementation brief.** Each stage states what it
delivers, the number that says it worked, and the observation that would say it
did not. Revised 2026-09-02 after independent review, which found the original
sequence confounding the very baseline it existed to protect.

### The measurement discipline that governs every stage

Three rules, all from `CLAUDE.md`, and the first one is the review's finding
against the first draft of this plan:

1. **Re-take the arena baseline immediately before every comparison, on the same
   binary.** The original plan took one baseline at Stage 1 and compared Stage 3
   against it — across which the genome widened 318 → 495 live slots,
   `mutation_rate` was re-derived, and a new sight tax landed. §8 says outright
   that *"a sampled genome at a given seed is a different animal"* after the
   widen. **That is §12a's own rule — two changes reallocating one budget cannot
   be read apart — violated inside this document's own sequence.** A baseline is
   valid only against the binary it was taken on.
2. **Nothing that moves the metabolic budget runs in parallel with an arena
   reading.** This retires the original Stage 4's claim that it was *"independent
   of stages 2 and 3, can be built in parallel"*: its `crop_capacity` item moved
   the same budget the arena reads. (That item is now dropped entirely — §11e.)
3. **Every stage carries a positive control, not only a negative one.** The
   review's severest finding (§11c) was reachable because the proposed control
   proved only that a gate is transparent when open, never that it is not shut on
   everything.

---

## 10A. Track A — senses and the nest

### Stage 0 — measurements owed before anything is built

**0a. ~~Does `moisture_gradient` read anything underground?~~ ANSWERED, and
the answer was bigger than the question** (§5b). `field_sense_probe`, PR #214.
Curvature does not move this channel at **any** sampling span — 1.012x at the
shipped ±4, and 1.011 / 1.006 / **1.003** at ±8 / ±16 / ±24, so widening it
moves *toward* 1.0. The channel is a surface-proximity detector, the termite
citation does not describe the shipped mechanism, and §5's remedy changed from
"re-derive the sampler" to §5f's three options. **Nothing further is owed
here**; what is owed is §5e's blind-guard control, which is a different run.

**0b. What does an ant-sized eye cost?** `vision_probe mode=cost` at radius 16,
32, 64 with ~50 animals. Read `rN` **minus `locate`** — that harness's own notes
record the whole-world scan overstating the sense's cost **thirtyfold**, and
`vs blind` is the wrong column.

**0c. Can an ant see kin from inside a burrow?** *(Added after review.)* Sight
rays die on `Solid | Powder`, so an ant in a gallery may see no kin at all — and
"where is home when home is a hole" is exactly the case §4's odometer replaces.
Report the fraction of kin-bearing reads that are non-zero for an ant standing
in a `labnest` gallery. **Mitigating evidence to check it against:**
`foraging-range-measurement.md` puts real excursions at 12–19 cells with ~4.5
per seed past 32, so a 16–32 radius is plausibly adequate above ground; the
question is only what happens below it. **If kin are invisible underground, §4
needs a contact-range fallback and should say so before Stage 3, not after.**

### Stage 1 — the instrument (§9)

Creature `selection_arena`. Controls, all three mandatory:

- `arm=same` **mirrored** must read exactly 50.0% — and is *vacuous*, an
  algebraic identity. It proves the harness runs, nothing more.
- `arm=same` **unmirrored** establishes the seed-driven spread everything else
  is read against. `labbatch` puts that at 2.42×–3.12× on the lab census.
- `arm=lethal` is the mandatory negative control and **must** be detected, or the
  harness is blind.

**One design question the plant harness cannot answer for us**: animals move
between arms in a way plants cannot, so attribution must be by
`OrganismState::lineage` and never by position. Confirm the mirrored control
still means what it means when the two arms can physically mix — and if it does
not, say so rather than inheriting the plant argument.

Then **run it on today's ant**: the baseline, on this binary.

**The falsifier for the whole programme lives here.** If the bed does not
discriminate against a genome known to be worse, nothing downstream is
interpretable and the fix is the ecology, not the genome.

### Stage 2 — the kin sense, additive

**Split in two after review, because the first version could not pass its own
guard.** It authored `ant.ron` both a `sight_fraction` and a `sight_range` and
then asserted `deliveries` would be unchanged — but `sight_tax =
sight_fraction × start_energy × sight_reads` is added to `spent` every creature
tick (`creature.rs:1825`), so turning both on charges the ant a metabolic tax it
does not pay today, which moves energy, deaths, births and therefore
`deliveries`. The stage contradicted itself.

**Stage 2a — the slots, wired to nothing.** Append `KinNear`, `KinBearing`; grow
`BRAIN_HIDDEN` 4 → 8; fix the manifest rule (§8). `ant.ron` gets **no**
`sight_range`, so it casts no rays, pays no tax, and reads both new inputs as a
constant 0.0. **`ascii` must be byte-identical on every counter** — the strict
guard the reserve's own design promises, and the one the first version made
impossible.

**Stage 2b — switch the eye on, and price it.** Author `ant.ron` a
`sight_range` and a `sight_fraction` together (never the range alone — that is
the ratchet `sight_fraction` exists to close). **The guard here is a stated
budget, not "unchanged":** name the expected energy delta from the arithmetic in
§6 before running, and check the measured delta against it. A run that comes
back *unchanged* is now the failure — it means the eye never fired.

Positive control for both: the new inputs must be non-zero where kin are and
zero where they are not.

### Stage 3 — the lab ancestor, and the deletions

A **new species file** — not an edit to `ant.ron` — with no `nest`, no
`nest_memory`, an authored odometer in hidden units, and a `MoistureGrad`
weight. In the same change, behind the species: delete the `recency` multiplier,
delete the `at_nest` drop branch, move the moisture coefficients to weights.

**Re-take the arena baseline first** (discipline rule 1) — the genome has
widened and the eye has been priced since Stage 1.

`ant.ron` authors weights reproducing its current behaviour, and `ascii`'s
foraging scene is the guard — **paired baseline run, not the test suite**,
because §2b records that the suite is blind to exactly this class of edit (an
edit once dropped the `since_nest` reset and 827 tests stayed green).

**Verify before building** that an authored self-recurrent unit can actually
approximate `1 - since_nest / nest_memory`. `squash` is `x/(1+|x|)` and
saturates; if three weights cannot express the decay, §4's central safety claim
— that the ant survives — is false and the stage must stop.

**What says it worked:** the new ancestor aggregates and forages in the arena at
a rate distinguishable from a zero-connection control, over 12 seeds, at an
order statistic. **What says it did not:** wanderers, on twelve seeds. That is a
real possible outcome and the owner has accepted it.

### Stage 4 — armour and severing (§11)

**One atomic change, and no longer claimed to be parallelisable.**

**P1 first, as its own commit:** author `food_energy` and `food_class` on
`beetle.ron` and `worm.ron` (§11a). Nothing can currently eat either, so every
predation measurement downstream is void without it.

Then, together:

1. **Author `penetration_resistance` on all sixteen food materials** (§11b),
   derived from the shipped `dig_force` values so ordinary food stays edible.
2. **`bite_force` on `CreatureDef`, defaulting to `dig_force`.**
3. **The gate in `act`'s ingest branch.**
4. **`reconcile_chain` becomes a severing rule** — 8-connected walk from the
   vital cell — *with* the ledger, corpse-stamp, rigid-path and
   mid-flight/mid-dig specifications of §11d(a)–(d). The rigid path depends on
   the body work's `body_after_step` fix; **until that lands, severing applies
   to chain and articulated bodies only, and rigid bodies keep the current
   rule.**

**`crop_capacity` is NOT touched** — the original plan's "derive it from cell
count" item is withdrawn (§11e).

**Acceptance, positive first:** with the gate live at authored values, `eats`
must be non-zero **on the herbivore path** and `harvested_corpse` non-zero on
the scavenger path. `creature.rs:6300` — the test whose message is *"and it has
to clear the bar, or the scavenger niche is gone"* — is the natural home.
**Then** the negative control (`bite_force` above every resistance reproduces
today's behaviour) and the paired predation delta on `meat_lost`.

**Run `seedbed_probe` on both sides**, because `penetration_resistance` is also
read by roots and by digging (§11c). If lowering `leaf`/`litter`/`moss` changes
germination, that is a plant-line consequence to accept deliberately or to avoid
with a separate `bite_resistance` field.

**What says it did not work:** nothing moves — which would mean bites are too
rare for resistance to bind. Check `eats` and `meat_lost` are non-zero in the
baseline **before** building.

### Stage 5 — the threat sense (§11f)

The mirror of Stage 2's kin sense: a bearing to the nearest animal **whose gut
values me**. Same rays, same full-circle bearing. **Not before Stage 4** — a
threat sense with nothing to fear is an input wired to a constant.

### Stage 6 — the Kind 2 genes

`sight_range` and `bite_force`. **`tick_interval` is withdrawn** from this stage
(§7): it is priced but not self-limiting, and it is an unpriced frame-cost lever.
S8's body axis comes after the body work, not here. `CREATURE_TRAITS` widens
lawfully (a wrong-arity RON tuple **panics** with the file position and both
lengths — measured; the silent case is a *misspelling*, not a widen).

### Stage 7 — crossover

`CROSS` on the shelf, and the heritable mutation rate beside it.

---

## 10B. Track B — the body

Runs **after** Track A's Stage 3 by the owner's mind-before-body sequencing
(§12a), and its first stage is a bug fix that Track A does not depend on and
could be pulled forward at any time.

### Stage B1 — fix `relocate_chain`'s self-overwrite

**Owner's decision, 2026-09-02: stage one of the body work.** `body_after_step`
can place one position twice when a head steps into its own tail, and
`relocate_chain`'s `to.iter().zip(&cells)` silently truncates while still
writing `state.chain = to.to_vec()` (§13b). This is what makes a `Chain(n ≥ 3)`
lose its head marking, and it is the real blocker on longer bodies —
`creature-chain-head-loss-2026-08-30.md` diagnosed it and nothing has fixed it.

**Everything else in Track B is measured on bodies that currently mis-report
themselves, so this is not optional sequencing.**

### Stage B2 — a body's live cells become state

The `body_after_step` fix of §11d(a) and §12b: a rigid body's cell set and count
stop being re-derived from the authored template every step. This is what lets
§11d's severing apply to every body plan, and it is the row §12b's first version
wrongly marked body-plan-independent.

### Stage B3 — the pre-checks (§13e)

`creature_scale mode=walk` including **the missing 2×2 rigid measurement**;
the blind A/B on candidate shapes; and the constant-extent `creature_look` run
at 36 cells that decides whether §13d's threshold claim survives.

### Stage B4 — articulation

Only if B3 supports it. `Chain` and `Rigid` become the two ends of one part-chain
representation (§13c), with the honest limits recorded there: blocking is set by
the **leading** part, a rearward taper is free and a forward one is not.

---

## 10C. Running alongside: `gene_probe`, the ratchet check (§3)

**Owner's call 2026-09-02: build it.** §3 is prose, and `CLAUDE.md`'s own
recurrence audit is that a prose discipline does not survive a real session —
*"make it a command rather than a discipline"*, the finding that produced
`scripts/docbench.py selftest` after two blind controls were written by an agent
that had just finished writing the rule down.

**What it does, plainly:** you are about to make something evolvable. Run the
population at several settings and look at where the population mean ends up.
Pinned to a bound and staying there means the gene is not expressing a choice —
it is expressing that nothing charges for it, or that nothing bends the return
curve, and it will ratchet on the first generation and express nothing after.
An interior mean means two reachable ends.

**Shape**: `examples/gene_probe.rs gene=<name> range=lo,hi seeds=12`, reporting
the population mean per setting and flagging a pin.

**Its controls, corrected after review.** The first version proposed
`sensor_offset` as the known-good. That is invalid: its numbers come from a
six-seed `#[ignore]`d unit test measuring a *steering score in a synthetic
harness*, not a population mean under selection — so if selection pins
`sensor_offset` because trail-following is not the binding fitness term,
`gene_probe` would be **correct** and would read as blind.

**A control for `gene_probe` must be measured with `gene_probe`.** Build it the
other way round: take a lever whose answer is known *by construction* —
`sight_range` with `sight_fraction` **forced to zero**, which is literally the
state that shipped and cannot be anything but a ratchet — as the **known-pinned**
control, and the same lever at an authored `sight_fraction` as the **known-bent**
one. Both arms in one binary, one run apart. Then, and only then, is a reading on
a gene nobody has classified worth anything.

It would have caught the plant architecture phase, `phototropism_dir`, E10's
body-length premise, and `tick_interval` in §7 of this document.

---

## 11. Predation, defence, and why size currently buys nothing

Added 2026-09-02 on the owner's question — *"how do creatures attack/defend"* —
and his ruling: **"yes, size (or other physical features) should buy
survival."**

**Revised 2026-09-02 after independent review**, which found the first version
of §11c would have stopped every animal in the world from eating anything, and
found two factual claims that were checked against the wrong files. Both
corrections are folded in below rather than appended; §16 records what changed
and why, because the *errors* are more instructive than the text that replaced
them.

### 11a. The encoding is good, and most of it should not be touched

**There is no predator and there is no prey.** Neither is a category anywhere.

- `is_visible_prey` (`creature.rs:2483`) is: a cell of `MaterialKind::Creature`,
  not me, not living kin (unless `eats_kin`), whose `diet_yield` **against my
  own heritable gut** clears `EAT_YIELD_THRESHOLD` (12.0). "Prey" resolves to
  *anything my gut values* — a filter over data, and a heritable one.
- **Attack is the `Feed` verb.** There is no `Strike`; slot 12 is deliberately
  unnamed (E13). A bite is `world.set(fxx, fyy, Cell::EMPTY)` plus
  `reconcile_chain` telling the victim.
- `is_living_kin` (`creature.rs:2347`) is species identity only, one line. It is
  not a relatedness model and does not pretend to be.

**The mechanism is symmetric. The shipped data is not, and that is a
prerequisite nobody had noticed.**

`assets/materials/beetle.ron` and `assets/materials/worm.ron` author **no
`food_energy` and no `food_class`** — zero occurrences of either, verified by
count. Both default to 0.0 (`material.rs:467`). `food_value` returns
`m.food_energy` for a living cell, so `diet_yield` is 0, so `is_visible_prey` is
false **at every gut bias**.

So **nothing in this world can eat a living beetle or a living worm.** Predation
is one-way today: the beetle (carnivore gut, `traits: (1.0, ...)`) eats ants;
nothing eats beetles. An arms race is impossible before this is fixed, and it is
one more instance of the failure class this document keeps finding — a channel
with a reader and no writer.

**Prerequisite P1: author `food_energy` and `food_class` on `beetle.ron` and
`worm.ron`.** Small, and everything in §11f depends on it.

### 11b. The finding: digging respects hardness, biting does not

**`creature.rs` reads `penetration_resistance` in exactly one place — the dig
branch, `creature.rs:2910`. The bite path never reads it.**

```rust
// dig, creature.rs:2910
if target.material != material::EMPTY
    && world.materials.get(target.material).penetration_resistance <= def.dig_force { ... }

// bite, act's ingest branch -- the whole test
if diet_yield(world, cell, gut.bias) > EAT_YIELD_THRESHOLD { ... }
```

So an ant with `dig_force: 1.0` cannot dig sand (1.4), gravel (3.5) or stone,
and **cuts through flesh with no resistance at all.** Flesh is the only
substance in the world that offers none.

**The scale of the gap, measured rather than sampled.**
`default_penetration_resistance()` is **100.0** (`material.rs:1491`),
*"impenetrable by default"*. Enumerating every material in
`assets/materials/` that authors a `food_energy`:

> `ant`, `ant_block`, `ant_block_shaded`, `ant_long`, `ant_wide`, `chitin_mid`,
> `chitin_pale`, `corpse`, `deadleaf`, `flower`, `fruit`, `leaf`, `litter`,
> `moss`, `seed`, `windfall`

**All sixteen author no `penetration_resistance` and sit at the 100.0 default.**
Every food in the game is carrying a value that says *you cannot cut this*, and
nothing reads it.

*(The first version of this section said "four creature materials, identical in
every food property". That was checked on `ant`/`chitin_pale`/`chitin_mid` and
generalised to `beetle` without opening it — which is how 11a's error got in
too. The sixteen-material figure is the corrected one and it is what makes 11c
dangerous.)*

### 11c. Recommendation R1 — a bite is a cut, and cuts already have a rule

**Route the bite through the test the dig already uses**: a bite removes the
target cell only if the attacker's force clears that cell's material's
`penetration_resistance`.

Why this rather than a bespoke damage model:

- **Zero new concepts.** Force-vs-resistance is the engine's universal "can this
  get through that": roots use it (`Behavior::Grow`'s `penetration_force`),
  digging uses it, and `dig_force`'s own doc already argues the case —
  *"the pattern roots already use, **not** a material-name whitelist: a species
  that can chew soil but not stone should say so in force, so a future softer
  stone is diggable automatically."* Armour is that sentence with flesh
  substituted for stone.
- **Armour becomes per-cell data, and therefore evolvable** once cell materials
  are heritable. A body of `chitin_mid` is harder to bite than one of `ant`.
- **It is priced by an existing term.** `body_energy` is what a cell costs to
  stamp and what it is worth as meat. Hard material costing more `body_energy`
  makes armour a real trade — slower to grow, dearer to replace, worth more to
  whoever does get through it. **That is the Kind 3 → Kind 2 promotion of §3, in
  one number per material rather than a new system.**

#### The trap, and it is the whole reason this section was rewritten

**A default of `bite_force = dig_force` does not mean "nothing changes until a
species says so". It means nothing eats anything, ever.**

`act`'s ingest branch is **one branch for all food** — `adjacent_food` returns
leaves, fruit, seeds, corpses and flesh alike (`creature.rs:2671`). With all
sixteen food materials at 100.0 and `ant.ron` at `dig_force: 1.0`, the gate is
`1.0 >= 100.0` for **every mouthful in the world**. Herbivory, scavenging and
predation stop on frame one.

This is `CLAUDE.md`'s named ban, and it passes the syntactic test while failing
the semantic one: *a size cap must bound work, never gate whether something
happens* — **the test is whether exhausting the gate produces an answer or
merely less work.** Here it produces an answer: *not food*. The first version of
this section quoted that rule approvingly in §3 and then violated it two
sections later, which is the same shape as the three reports `CLAUDE.md` records
quoting the rule while failing to be saved by it.

#### So R1 lands as one atomic change, with the food table authored

1. **Author `penetration_resistance` on all sixteen food materials**, soft to
   hard: plant matter and corpse low, `ant` low-ish, `chitin_pale` /
   `chitin_mid` higher, `beetle` highest. Values chosen so the shipped
   `dig_force` values still clear ordinary food — i.e. **derived from the
   existing forces**, not picked.
2. **`bite_force` on `CreatureDef`, defaulting to `dig_force`** — a separate
   field so biting and digging can diverge later without a migration.
3. Only then, the gate in `act`'s ingest branch.

**The shared-field coupling must be stated, because it is real.**
`penetration_resistance` is read by **roots** and by **digging** as well.
Lowering `leaf`, `litter` and `moss` from 100.0 makes them root-penetrable and
diggable for the first time. That may well be right — a root *should* grow
through leaf litter — but it is a change to the plant line, not a creature-side
tweak, and `seedbed_probe` is the instrument that owns it (it measured a
deadwood mat blocking 16 of 16 germinations, and found the gate is **water, not
material**). **Run `seedbed_probe` on both sides of this change**, and if the
coupling turns out to be unwanted, the fallback is a `bite_resistance` field
defaulting to `penetration_resistance` — a second knob, and therefore the
second choice.

#### The acceptance bar, corrected

The first version proposed a negative control only: `bite_force` above every
resistance must reproduce today's behaviour. **That control cannot catch this
bug** — it proves the gate is transparent when open, never that it is not shut
on everything.

**The control that catches it is the positive one `CLAUDE.md` demands:** with
the gate live at authored values, `eats` must be **non-zero on the herbivore
path** — an ant eating a leaf — and `harvested_corpse` non-zero on the scavenger
path. `creature.rs:6300` already carries a test whose message is *"and it has to
clear the bar, or the scavenger niche is gone"*; that test is the natural home
for the assertion. Only then is the predation-side `meat_lost` delta meaningful.

### 11d. Recommendation R2 — a bite severs; it does not kill

Today, `reconcile_chain` (`creature.rs:616`): if the surviving cells no longer
start with the chain's first cell, *"head gone, the rest is meat"*, and the
animal dies outright. **One bite on the right cell kills a 2-cell ant and a
20-cell animal identically.**

That is a binary outcome, and this project's first law is that an outcome is a
distribution — stated in `CLAUDE.md` as applying *"to every line in the
engine"*, learned from destruction, and rediscovered independently on the plant
line as graded death by `rot_remains`.

**The replacement: losing cells is damage, and what disconnects is severed.**
An 8-connected walk from the vital cell — 8 because body cells are placed at 8
neighbours and `CLAUDE.md` requires a traversal to use the writer's
neighbourhood. What stays attached lives on smaller; what detaches becomes meat
where it stands. Death is the vital cell taken, or energy at zero.

**Why not hit points:** a number with no physical referent, attached to the
animal rather than to its parts. Severing is the mechanism the engine already
runs everywhere — it is how a plant discovers it has lost a leaf, and how
structural collapse decides what falls. It is also satisfying in the house
sense: the second law is *there must be a verb and it must deliver something*,
and what a successful attack delivers is **a piece**.

#### Four things the first version left unspecified, all of which break it

**(a) A rigid body regenerates its lost cells for free.** `body_after_step`
(`creature.rs:3856`) branches on `is_rigid()` and returns
`def.body.offsets(west)` — **the authored template, re-derived from the head**,
with no reference to how many cells the animal still owns. A beetle bitten to
three cells asks for four positions next step and gets them back.

So §12b's contract was wrong: *"its cells"* and *"a cell count"* are **not**
body-plan-independent — for `Rigid` they are re-asserted from the plan every
step. **Owner's decision, 2026-09-02: fix it properly as part of the body work**
— a body's live cell count becomes state rather than a re-derived template —
rather than special-casing severing to chains. §12b's table is corrected
accordingly and §13's staging owns the fix.

**(b) The energy ledger double-books.** `reconcile_chain:643` books
`energy_ledger.meat_lost += body_energy * lost` **precisely because a cell lost
from a living animal never becomes meat** — it is a sink. If severed cells now
*do* become meat, the same matter is booked twice and `expected_live_total` stops
closing; if `meat_lost` is simply dropped, every other loss route (fire, brush,
explosion) loses its accounting. **The rule: a severed cell moves from
`meat_lost` to `harvested_*`-eligible standing matter, and the ledger entry
follows the matter.** Cells destroyed rather than severed still book
`meat_lost`.

**(c) Severed cells need their worth stamped.** `creature_dies`
(`creature.rs:4296`) shows the arithmetic — a corpse cell carries per-cell worth
in `aux` (`worth_in_aux`, authored only by `corpse.ron`). Severed cells take the
same path or they are worth the wrong thing as food.

**(d) Mid-flight and mid-dig are reachable and undefined.** The airborne path
(`creature.rs:3770`) holds `state.chain` and pro-rates idle against it;
`release_if_bodyless` (`:561`) frees the organism when `cells` empties. Severing
during a flight or a dig must be specified, not discovered.

### 11e. What this buys size — corrected, and it is less than the first version claimed

**The dilution argument does not work for a one-cell-wide chain, and the first
version of this section asserted that it did without doing the arithmetic.**

Under severing, a bite at position *k* of an *n*-chain amputates everything
distal from it. For a uniformly-placed bite the expected loss is about **n/2 —
half the body.** Today, by contrast, `reconcile_chain` filters by ownership and
preserves order without testing contiguity, so a mid-body bite costs exactly one
cell and leaves a geometrically disconnected but living animal. **Severing makes
each non-fatal bite cost a chain proportionally *more*, not less.**

So the honest statement:

| body | what severing does to it |
|---|---|
| **`Chain(n)`** | a bite costs ~n/2 cells. Size is a **liability** per bite, offset only by there being more of it |
| **articulated (§13)** | a bite severs at the nearest **joint**, so it costs *one part*. Size dilutes the fatal target honestly, and a big animal loses a limb and walks |
| **`Rigid`** | undefined until (a) above is fixed |

**This is an argument for §13, not an argument that stands without it.** The
graded-damage payoff and the articulated body are the same piece of work, and
the report should not have presented §11d's benefit as independent of it.

#### And `crop_capacity` from body size is neutral, not a payoff

The first version claimed deriving `crop_capacity` from cell count hands size a
foraging payoff. **Do the arithmetic.** With body cells *b*, capacity *k·b*,
round-trip distance *D* and trip time *T*, the energy per trip is
`idle·b·T + move·b·(1+k)·D` against a delivery of `k·b` cells, so cost per
delivered cell is `(idle·T + move·(1+k)·D) / k` — **independent of *b***.
Deriving crop from size makes foraging **size-neutral**. It removes a penalty;
it does not add a payoff.

Two further costs the "one line" framing hid:

- **`BrainInput::Carrying` is `c.worth() / def.crop_capacity`**
  (`creature.rs:2129`). Changing the denominator changes that input's
  normalisation **for every existing genome** — and `ant.ron` authors
  `(Carrying, Drop, 0.2)` as its whole away-from-nest putting-down rule.
  `dead-ends.md` line 984 records exactly what happens when that input starts
  lying: 30–35 of 52 ants standing laden, `digs` 121 against 881, a colony that
  from outside reads as having lost interest in digging. **This is a
  shared-budget reallocation and needs the re-derivation §3 demands.**
- **`crop_capacity` is in joules of face value** (`ant.ron:130`, `1440.0`), not
  cells. Deriving it from a cell count needs a joules-per-body-cell multiplier
  that nobody has derived — precisely the unpriced constant §3 warns about.

**Recommendation: keep `crop_capacity` authored for now.** It is not the lever
that makes size pay; §11d-with-§13 is.

### 11f. The gap that blocks an arms race, and it is one-sided

**A predator has an input for finding prey. Nothing has an input for detecting a
predator.** `PreyNear`/`PreyBearing` report *food*, not *danger*, and
`sight_range` defaults to 0 with only `beetle.ron:88` authoring it — so **an ant
cannot perceive a beetle at any distance**, only on contact.

Fleeing is not a missing verb; `Turn` and `Move` away are fleeing. It is a
missing **sense** to trigger on. `creature-evolution-plan.md` §7 names an arms
race as *"the standard engine of open-ended dynamics, and this world has never
run one"* — which cannot start while only one side can see, and (per 11a) cannot
start while one side is inedible.

**The fix is §4's slot pair evaluated the other way round**: a bearing to the
nearest animal *whose gut values me*. Same rays, same full-circle bearing, no new
category, no fear pheromone (gated behind a measured 0.5 ms third-plane cost
anyway).

**What S5a already measured, before anyone builds on this**: predation today
punishes neither ranging nor sheltering; mortality is biased *toward* home in
both arms; the two shelter tables disagree about the sign of the beetle term at
231 deaths. Shelter pays enormously and predators are not what makes it pay. So
P1, R1, R2 and the threat sense are **preconditions** for predation having
teeth, not refinements of a working system.

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
| **its cells**, as world positions | metabolism, rendering, damage | **yes, for `Rigid`** — see below |
| **one vital cell** | death (§11d) | no, *provided a body plan designates one* |
| **the connectivity neighbourhood** (8) | severing (§11d) | no |
| **each cell's material** | armour (§11c), meat value | no |
| **a cell count** | `idle_cost_per_cell`, `move_cost_per_cell` | **yes, for `Rigid`** — see below |
| **a movement rule** | stepping, passability | **yes — and nothing here touches it** |

**Two rows were wrong in the first version of this table and the review caught
them.** `body_after_step` (`creature.rs:3856`) branches on `is_rigid()` and
returns `def.body.offsets(west)` — the **authored template**, re-derived from
the head every step. So a rigid body's cell set and cell count are not state at
all; they are re-asserted from the plan, and a bitten beetle silently regrows.
**Owner's decision 2026-09-02: fix it properly as part of the body work** — a
body's live cells become state — rather than special-casing §11d to chains. §13
owns the fix.

That correction *strengthens* rather than weakens the contract's conclusion: it
names a specific, bounded defect to repair, after which all six rows hold for
every body plan.

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

The owner, 2026-09-02, on his single biggest issue with the body:

> *"My biggest issue is the visual. I don't want all [creatures] to look like a
> chain. That is not interesting, and so larger creatures just become
> snakes/worms. There are lots of other interesting things that could be done,
> but that is my number 1 issue."*

**Revised 2026-09-02 after independent review**, which found this section
citing a bug that had already been diagnosed and closed, and found the mobility
argument only half-working. Both are corrected in place; §16 records them.

### 13a. This is already written down in the engine, in the same words

`BodyPlan::scaled`'s doc comment (`organism.rs:1849`):

> *A `Rigid` plan supersamples and a `Chain` can only stretch… A chain is a
> path… So `Chain(n)` scales to `Chain(n*k)`, which is the right physical length
> and still one cell wide. **A chain cannot be made physically identical at a
> finer resolution, and that is not a bug to fix here: it is the reason the
> owner's "creatures should be more than chains of pixels" and the resolution
> step are the same piece of work.**

So the complaint is neither new nor an oversight. It is a property of having
exactly two body plans, and both fail to gain structure with size, in opposite
directions:

| | scaled up | what you get |
|---|---|---|
| `Chain(n)` | `Chain(n·k)` — stretches | a longer worm, still one cell wide |
| `Rigid(cells)` | each cell becomes a `k`×`k` block | the same silhouette, bigger. `ant_block`'s 3×3 becomes a 6×6 — **the "perfect cube"** |

The owner's verdict on the 36-cell creature — *"Shape. It is a perfect cube. Are
there perfect cube creatures in our world?"* — is literally the second row.

### 13b. Shape costs an order of magnitude of mobility — and the *other* blocker was a counter bug

**The mobility gap is real and survives everything.** A rigid body is blocked
**25–43%** of its moves; a chain **2–6%** (`creature-appearance-design.md` §4–5,
across all three trees it was taken on; note the *within*-rigid ranking in that
section is explicitly withdrawn — only the coarse gap survives).

So: **a body with an interesting outline cannot move, and a body that moves has
no outline.** Any answer to D1 must break that trade rather than pick a side.

**The first version of this section also cited
`creature-body-extent-2026-08-30.md`'s finding that "no chain longer than two
cells leaves a living colony" as a second, independent blocker. That finding is
superseded and the citation was wrong.**

`Reports/creature-chain-head-loss-2026-08-30.md` closed it: **the colony never
dies at all.** A `Chain(n ≥ 3)` loses its `CellType::Head` marking to a
self-overwrite in `relocate_chain` — `body_after_step` can place one position in
the next body twice when a head steps into its own tail — so `live 0` is *a
head-cell counter reading zero over a living, feeding, delivering population*.
Both controls were run: `kinfood=off` came back byte-identical, and
`eatskin=on` moved `meat_lost` 0 → 40,320, so the instrument could have reported
the other answer. `Reports/README.md` carries the standing, including **"the
extent lever is recoverable."**

**This was a process failure, not bad luck.** `CLAUDE.md` requires checking a
report's standing in `Reports/README.md` before trusting it, and that check was
not run. It is recorded here rather than quietly fixed because the same omission
would have sent an implementing agent to solve an ecology problem that does not
exist.

**What is actually blocking longer bodies is the `relocate_chain`
duplicate-position bug, and it is unfixed.** `relocate_chain`'s
`to.iter().zip(&cells)` silently truncates when the two lists differ in length
while still writing `state.chain = to.to_vec()`, so the chain can claim
positions the organism does not own.

**And an articulated body makes it strictly worse** — more positions in the
follow list, more opportunity for self-overlap. **Owner's decision 2026-09-02:
fixing it is stage one of the body work.**

### 13c. The proposal: an articulated body, which has both current plans as its ends

**A body is a short chain of *parts*. Each part is a small rigid shape. Each
part follows the path of the part ahead, as a chain cell follows the head.**

- `Chain(n)` is the case where every part is a single cell.
- `Rigid(cells)` is the case with exactly one part.

So this is **the generalisation the two existing plans are already the endpoints
of**, not a third plan beside them — which is why it is worth doing rather than
adding `Blob` next to `Chain`. It is also §2.8's `body_of(segments, girth)` with
connectivity free by construction, so disconnected and self-overlapping bodies
are unrepresentable rather than rejected.

#### The mobility argument, corrected — it works in one direction only

The first version predicted articulated bodies land *"between the chain's 2–6%
and the rigid body's 25–43%, much nearer the chain"*, on the grounds that a part
moves into ground the part ahead has vacated and proven passable.

**That is true of trailing parts and false of the one that matters.** Blocking is
set by the **leading** part, which moves into fresh ground and is a rigid body of
its own size. So a body's mobility is roughly **the mobility of its head part
alone**, and two consequences follow that the first version did not draw:

- **The silhouette you can afford is the silhouette of one part, repeated** —
  a much weaker claim than "a waist, a taper and a head that is not the same as
  the abdomen".
- **A taper backwards is free; a taper forwards is not.** Any part wider than
  the part ahead of it is *not* moving into vacated ground and meets the rigid
  problem directly. **A small head with a big abdomen — the insect silhouette
  this section is reaching for — is exactly the case the mechanism does not
  help.**

**This is a real limit on the proposal, not a detail.** What articulation buys
is a body that is *longer and jointed* at near-chain mobility, with modest
widening and free rearward taper. What it does not buy, for free, is an
arbitrary outline.

**And the number that would settle it does not exist.** There is **no measured
blocked rate for a 2×2 rigid body anywhere**, even though the shipped beetle is
one: `creature-appearance-design.md` §5 has `Chain(2)` 5%, `Chain(6)` 4%,
`Rigid` 3×3 43%, `Rigid` 5×2 41%, and nothing at four cells. One
`creature_scale mode=walk` run converts this whole argument from a prediction
into a number, and it is now pre-check 2.

#### What it does buy, stated at the strength the evidence supports

- **Proportions survive a resolution change**: at `k` each part supersamples, so
  a physically identical animal keeps its shape instead of becoming a longer
  worm. This is 13a's defect closed.
- **A joint is a natural cut line**, which is what makes §11d's severing grade
  properly — a bite costs one part rather than half a chain (§11e).
- **Local armour**: parts can differ in material, so a hard head and a soft
  abdomen become expressible (§11c).
- **A small, bounded, evolvable encoding**: part count and per-part extent.

### 13d. The honest caveat, and the counter-argument the first version dodged

**`creature-appearance-design.md` §4 measured shape at constant extent moving
nothing.** Two nine-cell bodies — a filled 3×3 and a waisted 5×2 insect outline
— came out **0.8% apart on ink and inside the noise on contrast**, on all three
trees. §1 states it flatly: *"Extent is the only lever."*

**Half the reconciliation is sound.** `creature_look`'s numbers — `ink`,
`|contrast|`, `decoys` — measure **findability**: can you locate the animal
against a textured world. The owner's complaint is not that he cannot find it;
it is what it is once found, and *"it is a perfect cube"* is a shape reading
delivered by eye at 36 cells. Stated as a claim that could be wrong:

> **Shape is below the noise at 9 cells and legible at 36.** The appearance
> report's 0.8% and the owner's "perfect cube" are the same axis measured either
> side of a threshold, and the resolution step (#179/#181) is what moved a
> physically ant-sized animal across it.

**The other half was missing, and it cuts against the proposal.** This document
says twice — §7's appearance paragraph and §12d — that the plant line's finding
is **composition, not architecture, sets a silhouette**, and that *"a revamp
that only changes which cell gets a label will land in the same place."*
Articulation **is** an architecture lever. The first version rescued it against
the appearance *metric* and never answered the *composition* argument it had
itself made two sections earlier. That is where the motivated reasoning was.

**The answer, and it is a genuine one rather than a patch:** articulation is not
only a relabelling, because it changes **extent and proportion**, which is the
lever the appearance report says *does* work. A three-part body is physically
longer than a one-part body of the same part size. Where it *is* only
relabelling — rearranging cells at constant extent — the appearance report
predicts it moves nothing, and this document should expect that.

**So the split prediction, which is what makes it falsifiable:** articulation
that *increases extent* moves the silhouette; articulation at *constant extent*
does not. If the pre-checks show the second, the answer to D1 is extent — bigger
animals — and articulation is only the means of making bigger animals still
mobile. **That would still be a win, and it is worth saying now so it is not
later dressed up as a failure.**

**And the standing gap: no instrument in this repository measures whether
something reads as an animal rather than a smudge.** Every appearance number
answers *can it be seen*. That gap is exactly how a shape lever fires and is
judged as nothing — the failure `plant-appearance-design.md` records costing a
whole phase. **Therefore D1's verdict comes from the review queue, not from a
metric**, and *before* the lever is built.

### 13e. The pre-checks, before any of this is built

1. **Fix `relocate_chain`'s self-overwrite** (13b). Stage one of the body work
   by the owner's decision. Everything else here is measured on a body that
   currently mis-reports itself, so this is not optional sequencing.
2. **Does it actually move?** `creature_scale mode=walk`, which is the body-plan
   mobility instrument and carries a standing positive control: **`Chain(2)` must
   reproduce 5%** (`examples/creature_scale.rs:31`, `:320` — the first version of
   this section quoted 5.2%, which is not the instrument's own figure). Measure
   **the missing 2×2 rigid** in the same run, then the articulated plans. If they
   do not land near the chain, the trade in 13b is unbroken and the proposal
   fails.
3. **Does anyone want these shapes?** Render candidates — 3 parts vs 5, uniform
   vs waisted vs tapered — at shipped resolution and post a **blind A/B**.
   `creature_scale mode=size` already renders one body per panel cropped to fixed
   *physical* units. If the owner cannot tell them apart, the lever is below
   threshold and the answer is extent, not articulation.
4. **The cheap test 13d needs and the first version omitted:** run
   `creature_look` on **two 36-cell bodies of different shape at constant
   extent**. If the 0.8% becomes ~15%, the threshold claim is supported and §13
   has a number. If it stays near 1%, the appearance report generalises and
   articulation's value is extent alone. One run.

### 13f. What this changes in §12's contract

| the engine asks | under articulation |
|---|---|
| its cells | **fixed by 12b's correction** — live cells become state for every plan |
| one vital cell | unchanged — the head of the first part |
| connectivity (8) | unchanged, and **stronger**: a part chain is connected by construction, so §11d's severing gets a natural cut line at a joint |
| each cell's material | unchanged, and **more useful**: armour can be local |
| a cell count | **fixed by 12b's correction** |
| **a movement rule** | **this is the row that changes**, and it is the only one |

**One concern checked and cleared, recorded because it would otherwise be
re-raised:** `parallel.rs`'s cross-chunk write-safety is **not** threatened by an
articulated body of any length. Creature movement runs in the **serial
active-site phase**, not the checkerboard — `creature.rs:536` says so in source
(*"Serial active-site phase, so plain `World::set` is correct and `MAX_REACH`
does not bind"*) — and `parallel.rs` touches creatures only through the
`pending_active_sites` queue. `MAX_REACH == CHUNK_SIZE / 2` stays load-bearing
for what it was load-bearing for, and this work does not go near it.

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

---

## 16. What the independent review changed, and the shape of the errors

A reviewer with no stake in this document checked its claims against source on
2026-09-02. It found **four confirmed errors severe enough to change what gets
built**, plus five moderate ones. All are corrected in place above. They are
recorded here because the errors generalise better than the corrections do.

### The four that mattered

| | what was claimed | what is true |
|---|---|---|
| **§11c** | `bite_force` defaulting to `dig_force` means *"nothing changes until a species says so"* | **All sixteen food materials** sit at the 100.0 "impenetrable" default and `act`'s ingest branch is one branch for *all* food, so the gate would have been `1.0 >= 100.0` for every mouthful in the world. **Nothing eats anything, ever** |
| **§13b/e** | *"no chain longer than two cells leaves a living colony"*, cited as a live blocker | Superseded. `creature-chain-head-loss-2026-08-30.md` diagnosed it as a **head-cell counter reading zero over a living population**; `Reports/README.md` records *"the extent lever is recoverable"* |
| **§11a/b** | the four creature materials are *"identical in every food property"*, so predation is *"symmetric by construction"* | `beetle.ron` and `worm.ron` author **no `food_energy` and no `food_class`**. Nothing in the world can eat a living beetle at any gut bias, and §11b's own headline example was unreachable |
| **§11d/§12b** | a body's cells and cell count are body-plan-independent | `body_after_step` re-derives a **`Rigid` body's template from the head every step**, so a bitten beetle regrows its cells for free. Severing was incompatible with the one row §12 promised nothing would touch |

### The shape of them, which is the reusable part

**Three of the four came from checking a claim against a neighbouring file
instead of the actual one.** `ant`, `chitin_pale` and `chitin_mid` were opened;
`beetle` was not, and it is the one that differs. The four creature materials
were counted; the *sixteen* food materials were not, and the ingest branch does
not distinguish them. This is `CLAUDE.md`'s *ask what your number counts when
nothing is wrong* with the instrument being a `grep` — **a sample of a table is
not the table**, and the file that breaks the pattern is exactly the one a
sample omits.

**One came from not checking a report's standing.**
`creature-body-extent-2026-08-30.md` was cited on its own text.
`Reports/README.md` records it as superseded, and `CLAUDE.md` requires that
check. The cost would not have been a wrong sentence: §13e made the superseded
finding a gate on shippability, so an implementing agent would have gone to
solve an ecology problem that does not exist.

**And the frame had the bug it was written to prevent.** §3 classified
`tick_interval` as *"already priced, safe now"* on the strength of a cost line
that genuinely rises with the gene — while cost and benefit both scale linearly,
so there is no interior optimum and the gene pins at the floor. The taxonomy
needed a second clause, and the section arguing that unpriced levers ratchet had
itself shipped a ratchet.

**The severest error passed a rule this document quotes approvingly.** §3 cites
*a size cap must bound work, never gate whether something happens*, and §11c
then proposed a gate that produces an **answer** (*not food*) rather than less
work. `CLAUDE.md` already records three reports quoting that rule while failing
to be saved by it; this is the fourth. The lesson is the one already written
there — **the test is semantic, not syntactic** — and quoting the rule is not
performing it.

### What this says about the document you are reading

The review's verdict was that the **diagnosis** half is sound and the
**prescription** half was not — and that the two failing sections were the last
two commits, written fastest and checked least. §§1–9 verified clean, including
the live-slot arithmetic, the `sight_fraction` gap, the S8 pricing correction
and the `FIELD_SCALE` finding.

**So weight this document accordingly**: its inventory of what is authored is
reliable and carries line numbers you can check. Its recommendations are one
review old, and §10's pre-checks exist because the predictions in §11 and §13
have not been run yet.

