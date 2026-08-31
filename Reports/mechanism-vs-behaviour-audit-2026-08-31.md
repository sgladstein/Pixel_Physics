# The mechanism-versus-behaviour audit — the evolution lab

**Status: audit and staged plan. No engine code was written and none should
be written from this document without the measurement each stage names.**
Scoped to the **evolution lab** (`cargo run --release --bin lab`) and the
engine it runs on: `creature.rs`, `brain.rs`, `organism.rs`, `plant.rs`,
`src/lab/`, `assets/species/*.ron`. The outdoor-only systems are scoped out
and §6 says which and why.

**Re-based on `main` 2026-08-31 and two findings closed by other lanes while
this was being written — see §7, which is the part worth more than either
finding.** Before that, **independently reviewed and corrected in eleven
places, one of them load-bearing.** The corrections are left visible in place rather than
edited away, per `CLAUDE.md`'s standing rule that a revert keeps the
knowledge. The largest: **F1 shipped with a defect that is false** (soil is
`Powder`, so the moisture field is not "blocked" there and does carry a
soil-water reading), **F3's proposed fix is a recorded dead end** that a
truncated grep hid, and **F8's re-derivation was undercounted 3.5x**. Three
findings were added that the first pass missed (F19–F21), and one item was
withdrawn from the cleared list.

**Everything below is read off the source at the commit named, or cited from
a report that measured it.** Nothing here was measured by this session, and
where a claim is arithmetic rather than a run it says so. **Baseline: `943ace17`** (PR #188, *hauling and looking cost something*),
read 2026-08-31; **re-verified against `main` after +41 commits later the same
day**, which closed two findings — §7. Every finding below carries its status
against that re-verification, and a reader picking this up later should redo
that check before acting: it is one `grep` per finding and it took a single
call for nineteen of them.

---

## 0. The line this audit is drawn on

The owner's framing, 2026-08-31:

> *"This sounds like we're forcing a system into creating behaviors that we
> want instead of creating the most correct system and allowing behaviors to
> develop."*

Settled as: **the mechanism is code, the policy is genome.** Something must
place a child's body and conserve the matter it is made of — that is
substrate and is not the target. *When* to breed, *how much* to give,
*where* the surplus was sitting, *whether* to eat what is in front of you are
policy, and each should be a weight or a trait that mutates.

Two corollaries, both learned expensively and both used as tests below:

- **Add senses and economies, never behaviours.**
- **A sense must not pre-categorise what it senses.**

**This extends `Reports/design-philosophy.md` §2b rather than restating it, and
the difference matters.** §2b forbids the *outcome* — *"the outcome is
forbidden, not simple rules"* — and **explicitly blesses tuned constants inside
a mechanism**: *"You may absolutely use a simple, tuned, weighted local rule…
The test for any new rule: is the resulting shape a side effect of a
mechanism, or is it curve-fit to look a particular way? The first is always
fine."* Shape 1 below is materially broader than that: it asks that a tuned
quantity be a **gene** wherever it is policy, which §2b does not require. The
owner's 2026-08-31 framing is what authorises the broader standard; §2b is
not, and an earlier draft of this section borrowed §2b's authority for it.
Several findings — F12 most clearly, and F2's and F10's thresholds — rest on
the broader reading and should be weighed as new doctrine rather than as
enforcement of settled doctrine.

### The four shapes, as used here

1. **A threshold standing in for physics.** A constant whose value was
   chosen to make a behaviour appear.
2. **An opt-in that is itself the hardcoding.** A default that encodes a
   design decision.
3. **A sense that pre-judges.** An input named for a conclusion rather than
   for a measurement.
4. **Statement order making a choice.** Two verbs where only one is
   reachable because of where the `return` is.

A fifth shape turned up that the brief did not name, and it outranks three of
the four for the lab specifically:

5. **The substrate silently deletes a policy channel.** A genome weight
   exists, mutates, is authored, fires — and cannot change the outcome,
   because something below it makes every setting resolve identically. This
   is `open-bugs-handoff.md` §R4, and the lab bed is exactly the terrain it
   is worst on.

### What this audit is *not* allowed to conclude

`CLAUDE.md`'s standing rule: **a term in a weighted sum is not an independent
knob.** Every finding below that promotes a constant into a gene reallocates
a budget that other constants were calibrated against. Each entry therefore
carries a **re-derive** line, and an entry whose re-derivation is not costed
**is not scoped, it is merely started** — that is the sentence `plant-
phototropism-lateral-2026-08-27.md` was written to pay for, when the correct
repair to `phototropism_dir`'s codomain sent reproduction to zero.

---

## 1. The inventory

Twenty-one findings — eighteen from the first pass, three (F19–F21) added in
review. Each carries its shape, what behaviour cannot currently evolve because
of it, the mechanism version, and what has to be re-derived alongside. The
ranking is §4; this section is in mechanism order so that related entries sit
together.

### The one that is four bugs stacked

#### F1. `creature::moisture_gradient` reads air humidity, returns a magnitude, and is wired at a fixed weight in a fixed direction

`src/sim/creature.rs:2230` (the function), `:2473` (drop), `:2494` (dig).
**Shapes 1 and 3, plus the coarse-field trap, plus a stale constant.**

```rust
fn moisture_gradient(world: &World, x: i32, y: i32) -> f32 {
    let m = |px, py| world.field_at_bilinear(px as f32, py as f32).moisture;
    let gx = m(x + 4, y) - m(x - 4, y);
    let gy = m(x, y + 4) - m(x, y - 4);
    ((gx * gx + gy * gy).sqrt() / WORM_MOISTURE_SATURATION).clamp(0.0, 1.0)
}
```

Three defects, and **each one alone would be a finding**.

> **Corrected after review.** This entry shipped with a fourth defect that is
> **false**, and the correction is worth keeping because of how it got here.
> It read: *"`field::step_diffusion` skips blocked blocks, so inside soil this
> channel carries no gradient at all."* That is wrong twice. `field.rs:2811`
> sets `blocked` only for `Solid | Plant`, and **soil is `Powder`**
> (`assets/materials/soil.ron:8`) — `field.rs:3238` says so in as many words:
> *"soil is not `Solid`, so it never hit this in the first place."* And
> `apply_moisture_sources` (`field.rs:2580`) **deliberately does not skip
> blocked blocks**, forcing the reading up to `soil_moisture(cell) /
> water_capacity` (`field.rs:2823`). So inside soil the channel *is* a soil-water
> reading — a coarse, block-resolution, lower-bounded one. The sentence was
> inherited verbatim from `e03c7bab`'s commit message, which overstates it the
> same way, and was not checked against `field.rs`. **The plant-side code had
> already corrected it**: `organism.rs:5316` gives the real reason that read was
> broken, and it is the coarse-field trap — defect 3 below — not blocking. The
> audit quoted that very line as its own defect 4 without noticing the two
> contradicted each other. `CLAUDE.md`'s *a commit message is not evidence the
> change is in the file*, one step removed: a commit message is not evidence of
> why it was needed either.

1. **It is a magnitude where the model needs a sign.** The stigmergy model it
   implements (`stigmergy-research.md` §4) says material accumulates at
   convex, *drying* sites. `|grad m|` is large at any boundary and cannot
   tell drying from wetting. This is `open-bugs-handoff.md` **§T2**'s own
   leading hypothesis.
2. **The coupling is code, not genome.** `drop_urge * gradient` and
   `dig_urge * (1 - gradient)` hardcode both the weight (exactly 1.0) and the
   *inversion*. There is no `MoistureGradient` brain input, so no lineage can
   express "I drop where it is wetting" or "I ignore moisture entirely" — the
   two verbs are welded to one channel with opposite signs for every species
   for all time.
3. **The sensor offset is stale, and it is the real coarse-field defect.**
   `±4` cells was chosen to straddle a
   `FIELD_SCALE` block boundary. `FIELD_SCALE` went 8 → 16 in PR #181; the
   offset did not move, so the two samples now sit **half a block apart**.
   Bilinear interpolation is the only thing keeping it from being identically
   zero. `organism.rs:5320` records the plant side re-deriving exactly this
   number's *meaning* when it was fixed; the creature side did not.

**What it costs.** §T2, measured: 1,651 pickups, 1,586 drops, **4
deliveries**, **0 food cells** standing at the nest over 24,000 frames.
Scaling the route-drop product by 0 gives 115 pickups, **13** deliveries,
**2 births**, generation 1. Gate 0 — an ant reaching generation 2 — is
blocked on this and on nothing else in the creature economy. The colony
cannot maintain a larder, so the whole social half of the game is unreachable.

**Which half of the code the cost is in, because the two arms differ and the
first draft conflated them.** `deliveries` increments **only** in the
`at_nest` arm, which does not multiply by the gradient at all — so the
gradient never scales a delivery directly. The measured cost comes from
*route* drops firing too readily **above ground**, where the channel is
ordinary diffusing air humidity and the term is **saturated**, not flat:
§T2's own hypothesis says so — *"a soil bed under air is one continuous
moisture boundary… so `drop_urge x gradient` is near `drop_urge` for the
whole route."* The "constant by construction" reading applies to the *dig*
term, which has no measured cost attached to it anywhere in this audit.
**This reverses the order of the fix** — see the mechanism below.

**And there is no guard.** `examples/ascii.rs:2473` already did the vacuity
work honestly and records the answer: the old `wet_drops > dry_drops`
assertion **passed harder with `moisture_gradient` deleted from the drop
probability entirely** (steep 18/flat 0 broken against steep 6/flat 0
working), because removing a multiplier below 1.0 raises the drop rate
everywhere. It was demoted to a print. Its successor, the `uphill` ratio, is
**printed and not asserted**, deliberately, because both arms stand on six and
eighteen drops. So: *what would the guard do if the mechanism were deleted?*
**Nothing. There is no assertion over this mechanism today.**

**Mechanism version.** `organism::moisture_pull` is already public, already
per-cell, and already returns a **direction and a magnitude**
(`organism.rs:5327`). Three steps, **and the order below is corrected from
this report's first draft, which had the two leading steps the wrong way
round**:

- **The sign first**, because it is the one that addresses the measurement. A
  drop bias wants the gradient *along the surface normal* — material
  accumulates where water is leaving. `moisture_pull` returns the unit
  direction; the drop rule wants its dot product with "away from ground",
  which the head's own footing already knows. A signed term is not saturated
  at a boundary the way a magnitude is, which is exactly what §T2 measured on
  the route.
- **Then** point `creature::moisture_gradient` at `soil_water_fraction`.
  **Do not expect this to move `deliveries` on its own, and do not read PR
  #185's *"the steering fix carries all of it"* as a precedent — the analogy
  is inverted.** There the channel was **flat where it was needed** (a root
  underground); here it is **saturated where it fires** (an ant on the
  surface), and `soil_water_fraction` returns a true `0.0` for air against
  `held/SOIL_SATURATED` for soil, so a per-cell read gives an *even sharper*
  magnitude at the soil/air boundary. It is the right quantity and it is not
  the fix for §T2.
- Add `MoistureGradientAlong` as brain input 18 (the layout is
  append-only and reserved — `brain.rs`'s positional law) and let the
  *genome* multiply it into `Drop` and `Dig`, at the ant's currently-implied
  weights, so generation zero is unchanged.

**Re-derive.** `WORM_MOISTURE_SATURATION` (4.0) is the normaliser and it is
scaled for the field's `0..4` range; per-cell soil water runs `0..1`. PR
#185's own note is the precedent and the warning: it measured `MIZ_THRESHOLD`
across a 10x sweep and found *where the threshold sits is not carrying the
behaviour; what the rule reads is*. Take that as a hypothesis to test here,
not as a result to inherit — the creature rule is a *multiplier* on a
probability, where the plant rule is a *gate*, and a multiplier's scale reaches
the outcome directly. Also re-derive `(Bias, Dig, ..)` and `(Carrying, Drop, ..)`
— tuned against the current near-1.0 multiplier and authored in **six species
files, not one**: `ant`, `ant_long`, `ant_block`, `ant_block_shaded`,
`ant_wide`, `chitin_pale` (`grep -rn "(Bias, Dig" assets/species/`).

**Dead-ends:** `moisture_gradient` returns **zero** hits. Clear.

---

### The economy: three thresholds and three unpriced verbs

#### F2. `hunger_fraction` decides eat-versus-carry with a species constant — **CLOSED 2026-08-31 by PR #190, not by this audit**

`src/sim/creature.rs`. **Shape 1. Filed open; closed by another lane the same
day, and the audit did not know until it re-based.**

**What shipped.** `CreatureDef::hunger_fraction` **no longer exists** — the
field is gone, not merely unread. `OrganismState` carries a `Crop { material,
shade, unit, cells, digesting }`, `CreatureDef` carries `crop_capacity`
(1,440 on `beetle.ron`), and an animal now takes what it finds and digests it
as it walks. `BrainInput::Carrying` was a boolean and is now **crop fill**, so
the input is graded rather than binary. The call site says it plainly:

> *"Nothing here decides between eating and carrying, because there is no such
> decision any more. What stood here was three hand-authored gates in a trench
> coat: eat below `hunger_fraction * start_energy`, carry above it, and — Gate
> 0 — eat again if this exact mouthful would close the gap to a birth bar, or
> if you happened to be standing at the nest. Each was added because a
> behaviour we wanted did not emerge, and the last of them had an ant doing
> arithmetic about the price of its own children."*

That is this finding's own diagnosis and its own proposed mechanism, arrived at
independently. **Nothing in the analysis below is retracted; it is simply no
longer work.** It is kept in full because the re-derivation list is the part
that outlives the finding — thirteen readers of the deleted ceiling, two of
them measurement harnesses, and a lib guard — and because a later session
asking *why* the crop exists should find the argument, not just the code.

**What did not close with it.** The crop deletes the *ceiling*; it does not
make `crop_capacity` heritable. `CREATURE_TRAITS` went 2 → 3, and the new slot
is `TRAIT_REPRODUCE_AT`, not crop capacity. So the trade this finding names —
a big crop hauls more and pays for it every step, now that PR #188 prices
carriage — is available and unspent. **That is F14's Stage 4, and it is now
cheaper than when this was written**, because the mechanism it needed exists.

---

<details>
<summary>The original entry, kept for its re-derivation list and its
argument (click to expand)</summary>

**F2 as originally filed:**

`src/sim/creature.rs:2263`, the `provisioning` clause at `:2364`, the
carry-home `else` at `:2407`. `organism.rs:2086` for the field. **Shape 1.**

```rust
let hungry = bank < def.start_energy * def.hunger_fraction;
```

**What it costs.** The bank ceiling. An ant stops banking above the hunger
line and ferries everything after it, so the most it can ever hold is
`hunger_fraction * start_energy + one mouthful`. `dead-ends.md`:1000 has the
arithmetic — reachability needs `Y - body_energy * cells > start_energy *
(1 - hf)`, and with `ant.ron`'s deliberate `food_energy == body_energy` and a
two-cell body the left side is **negative for any `hf <= 1`**. Measured:
richest bank 568 against a 1,860 bar, births 0. **This is a constant whose
value was chosen to make foraging appear, and it is now the thing preventing
reproduction.**

Gate 0's own dead-end entry names the escape: *"Re-test when: ... an ant
keeps eating past the hunger line."* The 2026-08-30 provisioning clause is
half of that — it lets an ant finish a meal that alone pays for a child, and
keep eating at the nest. It is still a **branch on a constant**, and it now
does arithmetic an ant should not be able to do: `bank < barrier && (bank +
gain >= barrier || adjacent_nest(..))` is an animal computing the price of
its own unborn offspring.

**Mechanism version: a crop.** Give the animal a foregut with a capacity, and
let it **digest at a rate** rather than deciding whether to swallow. Taking
food fills the crop; the crop empties into the bank per tick; the crop is
full or it is not. Then *eat-versus-carry falls out of trip time* — an animal
far from home digests what it took before it gets back, one near home arrives
with it still in the crop and can put it down. No threshold decides anything;
the outcome is a distribution over trip lengths, which is `CLAUDE.md`'s first
law arriving in the right place.

The crop capacity is the gene, and it is a **trade** rather than a ratchet
the moment F5 (hauling cost) is live — which it now is, since PR #188 prices
`carried_cells` per step. A big crop hauls more and pays for it every step.

**Re-derive, and this is the expensive one — the first draft counted six
readers and there are at least thirteen.** In `src/`: `app.rs:1153`,
`lab/stats.rs:1070` and `:1142` (the breed-margin gauge and the larder
histogram's hunger line), `lab/params.rs:449`/`:477`/`:570` (the panel row),
`world.rs:357` (the ceiling doc). **Plus seven example harnesses** —
`creature_probe`, `windfall_probe`, `stamp_probe`, `lab_cost`,
`creature_space`, `filmstrip`, `predation_probe` — two of which are this
audit's own instruments (`creature_probe`'s reachability line is Stage 2's
gate; `windfall_probe` runs §T2's reproduction). **Plus a lib guard**,
`creature.rs:7409`, whose entire assertion is built on `roof =
hunger_fraction * start_energy + diet_yield(..)`; a crop deletes that roof and
the guard goes on passing while testing nothing. Every one is a *readout of the
ceiling this change deletes*. The
`BreedMargin` calculation — `hunger_fraction * start_energy + best_mouthful`
against the bar — stops meaning anything and must be re-derived from crop
capacity and digestion rate, or the lab's own colony page will report a
margin that no longer exists. **Budget that as part of the work.** Also:
`ant.ron`'s `start_energy: 200` was itself cut against this ceiling (E14),
and `reproduce_threshold: 1100` was re-derived against that cut.

**Dead-ends:** five hits, and one is a *near miss* worth stating precisely.
`dead-ends.md`:904 kills *"eat if energy below full"* — that makes every
creature permanently hungry and deletes carrying entirely (14 eats against 8
pickups, zero deliveries). **A crop is not that rule.** The naive rule
removes the gate; a crop keeps a gate and moves it to a *capacity*, so an
animal with a full crop still cannot take more. The re-test condition at
:1001 names this route explicitly. Proceed, but the control arm that has to
be run is *deliveries* — the quantity the dead end destroyed.


</details>

#### F3. Reproduction is opt-in, and `reproduce_threshold: 0` means sterile

`src/sim/organism.rs:2120`; `assets/species/beetle.ron` authors neither
`reproduce_threshold` nor `mutation_rate`. **Shape 2.**

**What it costs.** The beetle is the only predator and the only animal with
eyes, and it **cannot have offspring**. Predator numbers cannot track prey,
so there is no population dynamic between the two species — the thing a
biosphere is *for*. Nine beetles and fifty ants is nine beetles for ever,
however the ants do. And with `mutation_rate` also unauthored, even a beetle
that could breed would produce byte-clones.

**The fix is not to author a predator breeding rate.** That repeats the error
one layer down — a designer choosing how fast predators multiply is exactly
the hardcoded outcome. The fix is that **reproduction stops being opt-in**.
An animal that can pay for a child has one; a species that should not breed
should be prevented by its *economy*, not by a missing line in a file.

**Mechanism version — and the obvious one is a recorded dead end. Withdrawn
after review.**

This entry first proposed defaulting `reproduce_threshold` to `birth_cost + 1`,
the floor `reproduce_at` already imposes, so the default reads "as soon as it
can afford it" instead of "never". **`dead-ends.md`:1543 is about exactly that
value**, and its re-test line reads *"Permanent as a shape of error"*:

> *"A threshold one point above the grant is an anti-freeloading condition
> satisfied by arithmetic and not in fact, and it produces a population that
> explodes and then dies… **12 of 12 seeds breed, 3 of 12 have a single ant
> alive at the end.** … **A floor is not a margin**, and the failure is
> invisible in the `births` column — which reads as a spectacular success —
> and visible only in `live`."*

Two more objections found in review, either of which is on its own sufficient:

- **It violates the field's own stated invariant.** `organism.rs:2102`: the
  threshold *"must exceed `start_energy`… A threshold below the grant would
  make every founder breed on its first tick having done nothing."*
  `birth_cost + 1` has no relation to `start_energy`, and for a lineage whose
  `TRAIT_BIRTH_GRANT` has drifted to its floor (`grant_fraction(-1) = 0`) it
  falls to `stamp + 1` — 801 for a beetle against `start_energy: 1600`.
- **It does not deliver the claimed unlock.** Beetle at shipped traits:
  `grant = 1600`, `stamp = 200 x 4 = 800`, so the defaulted bar is **2,401**
  against a bank ceiling of `0.8 x 1600 + one mouthful`. The default alone
  gives the beetle **no children**. This entry should have run F2's own
  arithmetic against its own proposal and did not.

**What survives is the finding, not the fix.** Reproduction being opt-in *is*
shape 2 and the beetle's sterility is real. But the exit is **not** a
one-line default: it is an authored `reproduce_threshold` for the beetle
carrying a **margin** over what a newborn is given, derived the way
`dead-ends.md`:1543's own working arm was (200 against a grant of 80 — an
animal must earn a whole leaf above its endowment), plus a `mutation_rate`.
That is a species-file change with a sweep behind it, and its readout is
**`live`, never `births`** — :1543's last sentence.

**Re-derive.** `mutation_rate` inherits the opt-in argument and is *not* the
same answer — `organism.rs`'s doc records `0.0` as deliberately the **control
arm**, the clonal null every selected run is read against. And `try_bud`'s
`births_denied_no_space` becomes load-bearing the moment a rigid-body beetle
starts placing four-cell children.

**Dead-ends:** five hits by the narrow grep, and **this entry originally read
them as "all about tuning the number downward… none is about the default,
clear". That was wrong, and the mechanism by which it was wrong is worth
recording**: the grep output was read through a `head` window that cut off
before :1543. `CLAUDE.md`'s *grepping a prose phrase gives false negatives* has
a sibling — **a truncated grep gives false negatives that look like a
complete answer**, and a "clear" verdict is exactly where that is most
expensive.

#### F4. Digging is free

`src/sim/creature.rs:1607` (`let mut spent = idle + synapse_tax +
sight_tax`) against `:2504` (the dig branch, which charges nothing).
**Ratchet — the unpriced term.**

An ant excavates a cell of soil for the metabolism it was paying anyway.
`dig_force` (`organism.rs:2192`) is therefore a lever on which **more is
strictly better**: a higher force opens more materials at no cost, so the
allele goes to its cap on the first generation and expresses nothing. This
is the identical failure `idle_cost_per_cell`'s own doc records for body
size, and that PR #188 closed on two more axes. **It is the third of the
same shape and it is still open.**

**One qualification, in the code's favour.** `dig_force` is not entirely
unconstrained today: `beetle.ron`'s own comment records that its `0.3` sits
*below* soil's `0.8` deliberately, which is what makes a burrow a refuge from
it. So the lever already carries an ecological consequence in one direction.
The finding survives — nothing is *charged*, so within the diggable range more
force is still free — but "more is strictly better" overstates it.

**What it costs.** A burrower and a surface forager cannot trade against each
other. There is no such thing as a lineage that digs *less* because digging
is expensive, so "tunneller" is not a strategy the population can find — it
is a thing every ant does at whatever rate `(Bias, Dig, 0.4)` says.

**Mechanism version.** Charge the dig against the material's own
`penetration_resistance`, which the verb already reads. Cost per cell
removed, proportional to resistance over `dig_force` — so soft ground is
cheap, hard ground is dear, and a strong digger pays for the strength it is
not using. That makes `dig_force` a trade (strength costs, and buys access)
rather than a ratchet, and it needs **no new constant**: the two operands are
both already at the call site.

**Re-derive.** `(Bias, Dig, 0.4)` and `(FoodAdjacent, Dig, 0.8)` in
`ant.ron`, both authored against a free verb. And `burrow_probe`'s roofed-void
baselines, which are the only numbers saying whether galleries still get dug
at all — `dead-ends.md`:1003 and :1006 are two failed attempts to steer this
verb and both are read against those.

**Dead-ends:** "dig cost" returns **zero**. The two dig entries that exist are
about *weighting the roll by burial or by cover*, both measured to do nothing
or to invert. Pricing is a different mechanism and is untried. Clear.

#### F5. Pheromone deposition is free

`src/sim/creature.rs:1682`. **Ratchet.**

`EmitA` and `EmitB` cost nothing, so no lineage has any reason to emit less
and both outputs are ratchets in the same sense `sight_range` was before
yesterday. **One qualification, in the code's favour:** `EmitA` is multiplied
by `recency`, which decays over `nest_memory` (`creature.rs:1679`), so an ant
far from home already lays less channel-A trail. That is a *distance* falloff
and not a price — nothing is charged either way — but it means "more trail is
strictly better" is not true as written for channel A. It is true as written
for `EmitB`, which has no such term. A colony cannot evolve toward
*quiet* — a scout that lays no trail so as not to recruit competitors to a
poor patch is not a reachable strategy.

**Mechanism version.** A per-species `emit_fraction`, the third sibling of
`synapse_fraction` and `sight_fraction`, charged on the deposit that actually
lands. Note the deposit is already gated on `moved` (P-11), so an animal
turning on the spot pays nothing, which is the right shape.

**Re-derive.** `pheromone::DEPOSIT`, and `ant.ron`'s `(Bias, EmitA, 2.0)` and
`(Carrying, EmitB, 2.5)`. **This one has a real risk attached**: the ant's
entire homing mechanism is the channel-A recency gradient, and there is no
steering toward the nest anywhere else. Pricing emission too hard deletes
homing. Sweep `emit_fraction` against `deliveries`, and default it to **0**
in the same way `sight_fraction` did, so the mechanism can land without
arming it.

**Dead-ends:** zero hits on emit or pheromone cost. Clear.

#### F6. `sight_fraction` is armed in code and unarmed in data

`src/sim/organism.rs:2052`; `assets/species/beetle.ron` authors
`sight_range: 64` and **no `sight_fraction`**. **Shape 2, on a mechanism
twelve hours old.**

PR #188 built the price for looking, correctly, and defaulted it to 0 "so a
species that has authored nothing is bit-identical". The only species in the
game with eyes did not author it. **So the ratchet the PR was written to
close is still open for the one animal it applies to.** This is not a
criticism of the PR — a mechanism landing before its calibration is the right
order — it is the second half that has not been done, and it will read as
done to anyone who greps for the field.

**Mechanism version.** Author a `sight_fraction` on `beetle.ron`, derived the
way `idle_cost_per_cell` was: from a whole-animal budget the shipped animal
already pays, divided by what it reads. `sight_cells_read` is already
counted, so the operand exists.

**Re-derive.** `beetle.ron`'s `start_energy: 1600` and `hunger_fraction: 0.8`
were both set against a free eye. And `dead-ends.md`:1545 holds
`(PreyNear, Persist, w)` as *"held rather than rejected"* pending **§R4 —
which this report files as F13, not F14; the first draft cited F14 here and in
F9, and both were wrong.** A sight cost lands in the same weighted sum that
sweep was measured in, so the two must not be changed in one diff.

#### F7. Budding has no verb — **PARTLY CLOSED 2026-08-31 by PR #192**

`src/sim/creature.rs`, `src/sim/organism.rs`. **Shapes 1 and 2. Filed open;
half of it closed by another lane the same day.**

**What shipped.** `TRAIT_REPRODUCE_AT` (`CREATURE_TRAITS` slot 2, taking the
count 2 → 3): **when to breed is now a heritable trait**, scaling the species'
authored `reproduce_threshold`. Its doc resolves Cole's paradox the way nature
does — *"the low allele is very nearly a suicide pact and the high one is a
survival buffer bought by breeding less often"* — which is a real trade in both
directions, and it satisfies the half of this finding that said *when to breed
is a threshold, not a policy*.

**What survives, and it is a real difference rather than a quibble.** This
entry proposed `BrainOutput::Reproduce` — a **per-tick probability**, rolled
after affordability. A trait is a *constant for the life of the individual*. So
the strategies still out of reach are exactly the conditional ones:

- hold back through a lean patch and spend in a rich one;
- decline to breed while carrying a load;
- breed at the nest rather than wherever the animal happens to be standing.

Every one of those needs the decision to read the world at the moment it is
made, and a trait cannot. **The trait is the better first half** — it is
cheaper, it has no shared-budget cost, and it gives selection something to act
on immediately. An output remains available as a lawful append and is now a
smaller change, because the trait already supplies the scale it would modulate.

**One thing the new trait's own doc settles, in F3's favour.** It states that
*"a species that does not reproduce (threshold 0) still does not, whatever this
slot says, so making reproduction universal stays S5c's decision to take
deliberately."* **F3 is therefore explicitly still open, acknowledged at the
source** — the beetle remains sterile by omission, and that was left as a
decision rather than an oversight.

---

<details>
<summary>The original entry (click to expand)</summary>

**F7 as originally filed:**

`src/sim/creature.rs:1720` — `try_bud` is called unconditionally at the end
of every surviving tick, and fires whenever the bank clears the bar.
`brain.rs`'s `BrainOutput` has eleven slots and **none of them is
`Reproduce`.** **Shapes 1 and 2.**

**What it costs.** *When* to breed is the brief's own first example of a
policy, and it is a threshold test. An animal cannot hold back through a lean
patch and spend in a rich one; it cannot decline to breed while carrying;
it cannot breed at the nest rather than wherever it happens to be standing.
Every one of those is a real forager strategy and none is reachable.

**Mechanism version.** `BrainOutput::Reproduce = 11`, a probability rolled
*after* affordability — so the economy still gates it absolutely and the
genome gates it conditionally. The append is lawful under `brain.rs`'s
reserved layout: it lights a row that already exists and is already zero, so
no other slot's weight moves. Author it at `(Bias, Reproduce, +large)` so
generation zero is bit-identical to the current unconditional fire.

**Re-derive — not "nothing", which is what the first draft said.**
`brain.rs:539` states the cost of a live append directly: *"What it does change
is `live_slots()` — 268 -> 288 — so `random_genome` draws 20 more values and a
sampled genome at a given seed is a **different animal** than it was. That is
the real, unavoidable cost of a live verb."* `live_slots()` is 318 today; an
**output** append adds `IN + HID` = 22 slots (+6.9%), an **input** append (F1's)
adds `OUT + HID` = 15 (+4.7%). Since `brain::mutate` iterates `live_slots()` at
a fixed per-slot rate, **the whole-brain mutation load per generation rises by
that percentage at unchanged `mutation_rate`** — a shared-budget reallocation,
which is the thing `CLAUDE.md` says must be named rather than discovered. One
piece of good news the first draft also omitted: `mutate`'s own doc confirms
every caller builds a dedicated stream, so the shared-`Rng` draw-shift gotcha
does **not** apply. And the moment `Reproduce` is authored below saturation,
`reproduce_threshold` stops being the whole bar and `lab/stats.rs`'s
breed-margin gauge needs the second term.

**Dead-ends:** zero hits. Clear.

---

### The senses


</details>

#### F8. `FoodAdjacent` is a boolean over a quantity that is already continuous

`src/sim/creature.rs:1822`, `brain.rs:370`. **Shape 3.**

```rust
inputs[I::FoodAdjacent as usize] =
    if adjacent_food(world, x, y, gut_of(world, organism, def)).is_some() { 1.0 } else { 0.0 };
```

`adjacent_food` returns `(offer, fx, fy, material)` — it has already computed
**exactly how good the best mouthful in reach is**, through the same
`diet_yield` the mouth pays. The sense throws it away and reports a bit.

**What it costs.** An ant standing beside a 1,440 flower and one beside a 480
leaf have **identical brain input**. A lineage cannot learn to hold out for
something better, to eat opportunistically when the offer is poor and carry
when it is rich, or to condition anything at all on food quality. This is
severe in the lab specifically, because `creature-gate0-births-2026-08-30.md`
established that *which* mouthful an ant takes is the whole of the
reproduction arithmetic — the same report is why `adjacent_food` was changed
from first-match to best-match. **The verb was taught to prefer the better
food and the sense was not taught to see it.**

**Mechanism version.** Report `offer / REFERENCE_MOUTHFUL`, clamped, in the
existing slot. `REFERENCE_MOUTHFUL` (`creature.rs:3533`) already exists as
the normaliser. Zero still means nothing edible in reach, so the input keeps
its old meaning at the bottom of its range.

**Re-derive — and this count was wrong by 3.5x in the first draft.**
`grep -rn "FoodAdjacent" assets/species/` returns **21 authored weights across
seven embedded species**: `ant`, `beetle`, `ant_long`, `ant_block`,
`ant_block_shaded`, `ant_wide`, `chitin_pale` (four each, three on the
beetle). The first draft said "six across two", having looked only at `ant.ron`
and `beetle.ron` — while **this same report names the five ant-family variants
in F17**. Every one is calibrated against a term that reads exactly 1.0 or 0.0
and will now spend most of its time near 0.3. This is the largest
re-derivation in the audit relative to the size of the change, and it is
exactly the shape that took plant reproduction to zero. Do not land it without
the sweep.

**Dead-ends:** `FoodAdjacent` returns zero. Clear.

#### F9. `PreyNear` / `PreyBearing` are a food detector wearing the name of an eye

`src/sim/creature.rs:2123` (`is_visible_prey`), `brain.rs:435`, `:455`.
**Shape 3, and it is the brief's own example confirmed.**

```rust
fn is_visible_prey(world: &World, cell: Cell, gut: Gut, self_organism: u16) -> bool {
    ...
    diet_yield(world, cell, gut.bias) > EAT_YIELD_THRESHOLD
}
```

The eye runs the *mouth's* filter. Its own doc argues this deliberately — *"a
meat gut stops seeing leaves"* — and the argument is good for **appetite**
and wrong for **vision**. An eye sees a thing; whether it is food or danger is
what a nervous system decides.

**What it costs, and this is not hypothetical.** An animal **cannot see a
predator**. Not because ants lack eyes — they lack `sight_range`, which is a
separate and legitimate opt-in — but because giving an ant `sight_range: 64`
tomorrow would give it an eye that returns only things it could eat. A beetle
is `food_class: +1` flesh; an ant's gut sits at 0.0; `diet_yield` for a
beetle at a neutral gut is `worth * 0.25`, and whether that clears
`EAT_YIELD_THRESHOLD` is an accident of the beetle's `body_energy`, not a
design. **Predator avoidance is unevolvable by construction**, and so is
every other use of vision that is not eating: following a conspecific,
avoiding a crowd, finding the nest by sight.

**Mechanism version.** Split the sense from the judgement, which is what the
corollary asks for:

- The eye returns the nearest **creature cell** it can see, whatever it is.
- Two inputs carry what a nervous system can act on without pre-judging:
  `SeenNear` (how close) and `SeenBearing` (which way), as now — plus
  **`SeenEdible`**, the `diet_yield` of what was seen, normalised. The
  categorisation moves out of the sensor and becomes a *third input the
  genome weighs*, which is precisely the difference between a sense and a
  conclusion.
- `is_visible_prey`'s two genuine exemptions stay where they are: its own
  body (by owner) and, for `blocks_sight`, occlusion. Those are optics, not
  appetite.

**Re-derive.** `beetle.ron`'s `(PreyBearing, Turn, -2.5)`. And the eye gets
**more expensive** — it now returns hits it used to skip past, so `sight_
cells_read` falls (rays terminate on the first creature rather than the first
edible one), which moves F6's tax in the *opposite* direction to the change.
Those two must be measured in one run or the sight cost will look like it
went the wrong way. `creature-sight-sense-2026-08-30.md` holds the reach,
shape and occlusion baselines to re-take against.

**Dead-ends:** one hit, `dead-ends.md`:1545, which is the `Persist` release
sweep and is *held pending §R4* — **F13, not F14; the first draft named F14 in
both places and that error is what makes this entry's ranking wrong.** It reads
*"both arms of this sweep are running a lever that only half works"*. So F9 is
gated on F13, which §5 lists under *"Not staged, deliberately"* because the fix
is a movement-model change nobody has designed. **Read correctly, F9 cannot be
done at #7 in the ranking below**; it waits on an unsolved problem, and the
ranking is annotated accordingly.

#### F10. `EAT_YIELD_THRESHOLD` is a global binary on what counts as food

`src/sim/creature.rs:1971`. **Shapes 1 and 3.**

One `f32` const, 12.0, decides for **every species and every gut** whether a
cell is food at all. It gates the mouth (`adjacent_food`), the near sense
(`FoodAdjacent`) and the far sense (`is_visible_prey`). Its own doc is
admirably honest — *"Derived from the filter's own arithmetic rather than
measured... It wants re-deriving from WP-8's survival-versus-`gut_bias`
sweep"* — and the sweep has not been run.

**What it costs.** A hard edge where the model is otherwise continuous. A gut
that would get yield 11 from a cell sees **nothing there**; at 13 it sees a
meal. `dead-ends.md`:1534 shows this deciding a whole design question: a
`-1.0` ant gut clears Gate 0 outright, and what stopped it was carrion
dropping below this threshold — *"carrion returns to the menu at about
-0.68, where a 480 corpse pays 12.0 and just clears `EAT_YIELD_THRESHOLD`,
and no position that keeps carrion also breeds on two cells."* **One
unmeasured constant is currently deciding whether an omnivore is viable**,
which is a question the owner has already ruled on directly.

**Mechanism version.** This one is genuinely harder than it looks and I would
**not** promote it to a gene. A per-species threshold makes it authored
policy in a new place; a per-individual one gives every lineage a free
dimension with nothing selecting on it — the exact objection `TRAIT_GUT_BIAS`'s
own doc records against a per-class diet vector. The mechanism version is to
**delete the binary**: make taking a cell a probability proportional to
`diet_yield / REFERENCE_MOUTHFUL` rather than a gate, so a marginal food is
taken rarely rather than never. The distribution replaces the threshold, which
is the first law again. Keep a tiny epsilon to stop the loop offering
zero-value cells.

**The cost the first draft missed, and it argues against its own proposal.**
`EAT_YIELD_THRESHOLD`'s doc opens with the sentence this entry did not quote:
it is *"the number that makes the gene change **behaviour** rather than only
bookkeeping: a meat-gut animal literally stops **seeing** leaves."* Replacing
the binary with a probability proportional to `diet_yield` makes
`TRAIT_GUT_BIAS` — one of only **two** heritable creature scalars — a pure
efficiency multiplier with no qualitative expression at all. That is a direct
cost to **F14's own thesis**, and it also undercuts the `-1.0` grazer route
this entry cites as its motivation: the reason that gut *worked*
(`dead-ends.md`:1534) is precisely that the threshold took carrion off the
menu. Note too that the proposal's own *"keep a tiny epsilon"* re-introduces a
threshold on the same predicate with a smaller number, and nothing here says
what distinguishes it semantically from the one being deleted.

**So this entry is downgraded to a question rather than a proposal.** The
finding — an unmeasured global constant is currently deciding whether an
omnivore is viable — stands. The mechanism version does not, until WP-8's
sweep says what the threshold is buying.

**Re-derive, if it is ever done.** Everything downstream of the menu:
`adjacent_food`'s best-match scan, `FoodAdjacent`, `is_visible_prey`, and the
guard `a_starved_nestmates_corpse_is_still_dinner`, which is a *named owner
verdict* and must be watched going red for its own fault before and after.
**Run WP-8's sweep first.**

**Dead-ends:** one hit, :1534, which is about the gut allele rather than the
threshold, and its re-test condition is an owner ruling. Clear, with the
sweep as a precondition.

---

### Statement order and the missing split

#### F11. `Feed` conflates take-to-eat with take-to-carry, and `act`'s branch order makes the nest larder self-consuming

`src/sim/creature.rs:2276`–`2410` (the eat/pickup block), `:2447` (eat what
is carried), `:2468` (drop). **Shape 4.**

`act`'s order is: **eat-or-pick-up** (gated on `carrying.is_none()`), then
eat-what-is-carried (gated on `hungry`), then drop (gated on `carrying`), then
dig. Follow an ant through its own nest:

- Tick *n*: carrying, at nest, `drop` fires → `deliveries += 1`.
- Tick *n+1*: `carrying.is_none()`, `adjacent_food` finds the cell it just
  put down, `feed_urge` rolls, `!hungry` and not provisioning → **`pickups
  += 1`**. It takes the delivery back and walks off with it.

The drop branch cannot compete, because it is unreachable while
`carrying.is_none()`. **The two verbs are not a choice; they are a two-tick
oscillator**, and a measurement of "the colony eats its own granary" is
really "the code never offers the alternative".

**Two corrections to the brief's version of this, both in the code's favour**,
because a finding overstated is a finding that gets dismissed:

- `AtNest` **is** a brain input (`brain.rs:372`) and `ant.ron` authors
  `(AtNest, Drop, 0.9)`. So a genome *could* author `(AtNest, Feed, -2.0)`
  and suppress the re-take. It is reachable, not impossible.
- The `provisioning` clause deliberately keeps eating at the nest, and that is
  the mechanism `creature-gate0-births-2026-08-30.md` was built for. Eating
  the larder is *wanted*; **taking it away again is not**, and the code cannot
  tell those apart.

**The accurate finding is the conflation.** `BrainOutput::Feed` was split out
of `Dig` precisely because *"§13d added `(Bias, Dig, 0.4)` because ants never
dug, and it silently raised the baseline eating probability at the same time;
nothing could separate the two, so a burrower and a grazer are not
distinguishable points in the genome"* (`brain.rs:516`). **That exact
sentence is true again one level down**: `Feed` now gates swallowing *and*
picking up, so a grazer and a hoarder are not distinguishable points in the
genome. The split that was made was not made again where it recurs.

**Mechanism version.** `BrainOutput::Take = 12`, split out of `Feed` on the
same terms and by the same lawful append: `Feed` keeps "swallow what is in
reach", `Take` gets "pick it up and carry it". Author `Take` at the weights
`Feed` inherited, so generation zero does not move. Then the granary re-take
is a *choice the genome loses or wins*, and a colony that evolves
`(AtNest, Take, -)` has discovered a larder.

**Re-derive.** `(Bias, Feed, ..)` and `(FoodAdjacent, Feed, ..)` are
duplicated onto the new row, so the sum each output sees is unchanged — and
that is **six species files, not one** (`ant`, `ant_long`, `ant_block`,
`ant_block_shaded`, `ant_wide`, `chitin_pale`, plus the beetle's own pair).
This is the cheap case for the *weights*, and it is cheap because the append is
lawful — but it is **not free**: as F7 records, an output append moves
`live_slots()` 318 -> 340 and raises every lineage's per-generation mutation
load by ~6.9% at unchanged `mutation_rate`. Two appends (this and F7's) compound
that; F1's input append adds a third. **Name the total before landing any of
them, not after.** Watch `pickups` and `deliveries` together: the pair is the only thing
that says whether the split fired.

**Note this interacts with F2.** A crop changes what "carrying" means, and
the two should be sequenced — crop first, since `Take` is a much smaller
change once the eat/carry decision is no longer a branch on a constant.

**Dead-ends:** zero hits on a take/feed split. Clear.

#### F12. `CHOICE_EXPLORATION_K` — the last locomotion policy constant

`src/sim/creature.rs:223`, consumed at `:500` (worm) and `:2726` (the chain
step). **Shape 1.**

`Persist` was an anonymous `0.15`. `Tumble` was `TUMBLE_ON_FAILED_MOVE`.
`Caution` was `FOOTING_BONUS`. All three were promoted to genome outputs
after an ablation found **eight of ten authored instincts produced
bit-identical behaviour** — the brain riding along while hardcoded locomotion
policy did the deciding. `CHOICE_EXPLORATION_K` is the surviving member of
that family and it was not promoted.

**What it costs.** Exploration-versus-exploitation, for every animal in the
world, at one value. A bold generalist and a cautious specialist differ
exactly here. It also sets how hard a poor candidate can win, which is the
term that decides whether a colony ossifies on its first path — the same
question `Crowding`'s doc calls *"the negative-feedback term, and it is not
optional"*.

**Mechanism version.** Fourth output of the same family. It is **not** a new
slot: `unit_scale(outputs[Persist])` and this term compete in the same sum, so
the honest version is a genome-scaled `k` passed into `choose_weighted`,
authored so the shipped ant reproduces 0.1 exactly.

**Re-derive.** `choose_weighted`'s two guards
(`choose_weighted_prefers_the_strong_score_without_ever_excluding_the_weak_one`,
`choose_weighted_is_uniform_when_every_score_is_flat`) both pass
`CHOICE_EXPLORATION_K` explicitly and are fine. The real cost is that `k`,
`Persist`, `Caution` and the `footing * 0.5` zeroing at `:2700` are **one
weighted sum**, and the first draft described the other three as "already
genes". **They are genome outputs whose expressible range is set by three more
Rust constants in the same sum** — `PERSIST_MAX = 2.0` (`creature.rs:808`),
`FOOTING_MAX = 1.2` (`:801`), and the bare `0.5` multiplier at `:2700`.
Promoting `k` reallocates against all three, and none of the three is itself a
gene. That makes this entry *larger* than it reads, not smaller. Sweep against `moves_blocked` and
`falls`, which are the two counters that go wrong when this sum is wrong.

**Do this one last.** It is real, it is cheap to build, and it is the entry
most likely to move something that is currently working.

**Dead-ends:** zero hits. But **P-10 is load-bearing and is not negotiable**:
`choose_weighted` must never become an argmax, and a gene that can drive `k`
to zero *is* an argmax. Clamp the floor.

#### F13. `BrainOutput::Turn` is inert on flat ground, and the lab bed is flat

`src/sim/creature.rs:2700` (`footing_ahead` and the zeroing pass).
`open-bugs-handoff.md` **§R4**, filed 2026-08-30. **Shape 5.**

Not a new finding — it is already filed — but **nobody has read it as a lab
finding, and it is one.** The scoring is `base = [turn, persist, -turn]`. On
a flat floor the down-diagonal fails `passable` and the up-diagonal fails
`body_has_foothold`, so the zeroing pass at `:2700` kills the turn candidate
outright unless `turn > footing * 0.5`, which it cannot be while it has no
footing. §R4's measurement: **byte-identical movement over 1,600 frames while
the eye reported prey on 139 of 195 casts.**

**What it costs, in the lab.** The lab bed is a hand-built box with a level
soil surface. So:

- the beetle's eye — the only distal sense in the game, and the whole of E15 —
  **cannot steer in the lab at all**;
- any evolved `Turn` weight is selected on terrain the lab does not have;
- `dead-ends.md`:1545's `Persist`-release sweep, the strongest lever measured
  for pursuit, was run on generated terrain and its own entry says both arms
  ran a half-working lever.

The ant survives this because its trail-following routes through hidden units
into **`Move`**, not `Turn` — run-and-tumble, which works on the flat. The
beetle does not.

**Mechanism version.** Not obvious, and §R4 says so honestly: letting an
unfooted diagonal win puts creatures back to walking off ledges (`falls
16,451 against moves 22,138`). The candidate that does not have that failure
mode is **letting the heading rotate without a step** — turn as its own
action, paid for like any other. That is a movement-model change and wants its
own design pass; it is named here so the lab knows what it is waiting on.

**Cheap first step, from §R4 itself:** a counter for how often a `Turn`
request is discarded because the side it asked for scored zero. In the lab
bed that number is the finding.

---

### The umbrella finding

#### F14. `CREATURE_TRAITS = 3`: the species file is the policy layer, and it barely mutates

`src/sim/organism.rs`. **This is F3, F6 and half the rest seen from one level
up, and for the lab it is the most important entry in the document.**

**Re-measured 2026-08-31 after re-basing: `CREATURE_TRAITS` is now 3, not 2**,
and `CreatureDef` has **26** fields, not 25. PR #192 added
`TRAIT_REPRODUCE_AT` and PR #190 added `crop_capacity`. The ratio moved from
2-of-25 to **3-of-26** — the finding is unchanged in kind and the arithmetic
below is restated at the new numbers.

`CreatureDef` has **26 fields**. A child inherits exactly two things
(`creature.rs:1291`–`1301` and `:1316`–`1324`): the brain genome, mutated at `def.mutation_rate`,
and `traits`, jittered at `def.trait_variance`. `traits` is **three scalars** —
`gut_bias`, `birth_grant` and `reproduce_at`. Everything else comes off the
`def`, which is the species file, shared by every individual of that species
for ever:

> `body`, `tick_interval`, `start_energy`, `idle_cost_per_cell`,
> `move_cost_per_cell`, `synapse_fraction`, `sight_fraction`, `shade_rule`,
> `body_energy`, `crop_capacity`, `reproduce_threshold`, `mutation_rate`,
> `trait_variance`, `climbs_over_kin`, `eats_kin`, `nest`, `dig_force`,
> `nest_memory`, `sight_range`, `sensor_offset`.

**`crop_capacity` is the one to look at**, because it is *newly* on that list.
It arrived with the crop (PR #190) as a species constant, and it is the exact
shape this finding is about: how much an animal can carry before it must eat
or turn for home is a **strategy**, it is priced in both directions since PR
#188 charges for carriage per step, and it is fixed for every individual of a
species for ever. **A mechanism landed and its policy knob was authored rather
than inherited** — which is this finding recurring, one day later, in the code
written to close its neighbour.

**What it costs.** The lab's premise is *evolve a creature, keep it, breed it
forward*. What can actually evolve is a wiring matrix and three body scalars.
Body size, metabolic rate, digging strength, how far it sees, how much it can
carry, how long it remembers home, how often it thinks and how fast it mutates
are all **fixed at the species level**, which means the lab's specimen shelf
can keep a genome that differs from its ancestor only in synapse weights and
those three numbers. Every visible property of an animal — the thing the owner
looks at, and the thing `Reports/creature-appearance-design.md` says is
already hard to see — is outside the genome.

**Compare the plant side, which got this right.** A plant carries **ten**
continuous `genotype_draws` (`organism.rs:3478`), **six** discrete loci with
allele tables (`DISCRETE_LOCI`, `LOCUS_ALLELES`), and **heritable fates**. It
has both the continuous spread *and* the discrete jumps that
`DISCRETE_LOCI`'s own doc argues are what a species is: *"There is no setting
of a continuous genome that yields two clumps."* **The creature side has no
discrete loci at all.** So §5 of the design guide — separation,
specialisation, persistence, the score that names no behaviour — is asking
the creature genome for clusters it is structurally incapable of producing.

**Mechanism version, staged.** Do **not** promote twenty fields at once; that
is a shared-budget reallocation nobody could cost. The order that has a
measurement at each step:

0. **`TRAIT_CROP_CAPACITY` first**, and it moved to the front of this list on
   2026-08-31: PR #188 already prices carriage per step and PR #190 already
   built the crop, so the trade is priced and the mechanism exists. It is the
   one slot on this list whose preconditions are *both* already met.
1. `TRAIT_DIG_FORCE` — but **only after F4 prices digging**, or it
   is a ratchet the day it lands.
2. `TRAIT_SIGHT_RANGE` (slot 3) — **only after F6 arms the sight cost**, same
   reason. Its own doc already says so.
3. Discrete loci for the creature, mirroring the plant's: body plan, whether
   it climbs kin, whether it eats kin. These are *categorical* and are what
   clusters need.

**Re-derive, at every step:** `CreatureDef::scaled` (`organism.rs:2320`ff)
holds the resolution-invariance arithmetic for every per-cell rate, and any
new trait that is a *length* or a *rate* needs its own row there. PR #188's
`sight_fraction` row is the worked example and its doc says why dividing by
`burn` is the easy mistake.

**Positional forever.** `CREATURE_TRAITS`' own doc: a slot dead by
measurement in every species may be re-purposed once; a live slot never. Any
addition must be an **append**.

---

### The plant half

#### F15. A plant's whole reproductive strategy is a species constant

`src/sim/plant.rs:7362` reads `Behavior::Reproduce { seed_cost,
reproductive_allocation, seed_maturity }` straight off the species. **None of
the three is passed through `plant::genotype`** — verified by grepping every
`genotype(world, organism_id, ..)` call site (`plant.rs:2433`, `:2479`,
`:2492`, `:2496`, `:2514`, `:3051`, `:6776`, `:7420`, `:7771`). **Shape 1.**

So: **when** a plant switches from growing to reproducing (`seed_maturity`),
**how much** of its surplus goes to seed (`reproductive_allocation`), and
**how big** a seed is (`seed_cost`) are identical in every individual of a
species and cannot mutate. Those three are, between them, the whole of plant
life-history theory, and the third is Smith–Fretwell's offspring-size trade —
**which the creature side already built, as `TRAIT_BIRTH_GRANT`.** The same
trade-off, recognised and given a slot on one line and not on the other.

**What it costs.** In the lab bed the herb is the founder and the food supply.
`wiki/ants.md` records the supply problem: *"a herb's fruit ripens, hangs
there ready to fall, and mostly never does... a whole bed of herbs puts about
twenty pieces of fruit on the ground."* A population cannot evolve toward
more, cheaper seed, because no individual differs from another in seed cost.
Selection has nothing to sort.

**Mechanism version.** Three appended slots, or — cheaper and better — **one**:
`seed_cost` first, as the direct mirror of `TRAIT_BIRTH_GRANT`, since its
trade-off is already argued out in that slot's doc and is known to be real in
both directions only because a poorly-provisioned offspring can die. **It should be an append, not slot 9.** The first draft proposed spending
genome slot 9, which is live-width with no consumer — and that contradicts this
report's own §3. Slot 9's doc says why: it is reserved for a named purpose (a
strain-response gain), and appending rather than re-purposing was *a deliberate
call* that cost *"the measurement record a second time"* because *"the F4
megastudy re-run is already queued against the current numbering."* Taking it
re-baselines that study. **Append slot 10 instead** — `seed_genotype` keys each
draw on `rng::stream(world_seed, x, y, slot)`, so a slot's value is a function
of its own index and adding one draws a stream nobody had drawn before, which
is exactly what makes an append cheap here and a re-purpose expensive.

**Re-derive.** `REPRODUCTIVE_BUDGET_CAP`, `seed_maturity_met`, and
`plant.rs:7394`'s `affordable = budget / seed_cost`, which is a *count* of
seeds and changes meaning the moment `seed_cost` varies per individual. And
`herb.ron`'s authored triple, which was tuned against a fixed cost.

**Dead-ends:** zero hits on any of the three. Clear.

#### F16. Plant mutation rates are Rust constants, and the design guide says they are data

`src/sim/plant.rs:1356` (`MUTATION_SIGMA: f32 = 0.08`), `:1501`
(`FATE_MUTATION_CHANCE: f32 = 0.30`). **Shape 2-adjacent, and it falsifies a
planning assumption.** One qualification: `FATE_MUTATION_CHANCE` does carry a
live env override, `PIXEL_PHYSICS_FATE_MUTATION_CHANCE` (`plant.rs:1504`), so
a harness can reach it. It is still not `.ron` data and still not on the
parameters page, which is what the guide's claim turns on.

`evolution-lab-design-guide-2026-08-30.md` §7b-i, recording an **owner
decision** on the mid-game mutagen tier: *"The `mutation_rate` and
`FATE_MUTATION_CHANCE` dials already exist as data, so this is equipment that
writes a number, not a new system."*

That is true for the creature half — `CreatureDef::mutation_rate` is a
`.ron` field and is on the lab's parameters page (`lab/params.rs:485`). It is
**false for the plant half**: both plant rates are `const` in Rust, neither is
in `tunables.rs`, neither is on the parameters page. The asymmetry is
invisible from the guide, and the guide is what a later session will plan
from.

**Mechanism version.** `SpeciesDef` fields with the existing `#[serde(default)]`
pattern, defaulted to the current constants so nothing moves, plus two rows on
the parameters page's plant tab. `design-philosophy.md` §2a settles this
without argument: *"a constant graduates to a hot-reloadable `.ron` value
immediately if a non-programmer might plausibly want to tune it."* A mutation
rate is the definitional case.

**Re-derive.** Nothing, if defaulted. But **correct the guide in the same
commit**, or the next session plans against a false premise for the second
time.

#### F17. The lab's `COLONY` verb can only ever place the species named `"ant"`

`src/sim/creature.rs:1349` (`plant_creature_seed(self, x, y, "ant")` — a
string literal inside `found_colony`), `src/lab/params.rs:269`
(`COLONY_SPECIES: &str = "ant"`, honestly documented as mirroring it).
**Shape 2.**

**What it costs, and this is named by the record rather than inferred.**
`dead-ends.md`:1535, the re-test condition on the `-1.0` grazer gut:

> *Re-test when: The owner rules differently on the omnivore, or a **separate
> grazer species** exists to carry the specialist gut (E5 already asks for "a
> new solitary-grazer ancestor" rather than the ant, and `lab::scene` founds
> colonies by the name `ant`, so this needs the lab's species to become a
> parameter).*

A measured route to Gate 0 — a full plant specialist reaching generation 2 on
both seeds — is blocked by a string literal. The player also cannot release
beetles, worms, or any of the five shipped ant-family body plans as a colony.
`LabBox` has a `species` field and it is for **plants only**
(`lab/scene.rs:184`).

**Mechanism version.** `found_colony(&mut self, x, y, species: &str)`, and a
creature chip on the bar mirroring the plant chip. Note the constraint from
`evolution-lab-genetics-2026-08-31.md` §7: **the bar is full**, both rows, 1 px
spare — so this needs a third row, a page, or a removal, and three attempts
are already in `dead-ends.md`. The engine change is a parameter; the interface
change is the work.

**Re-derive.** `COLONY_ANTS`/`COLONY_ANT_SPACING` are calibrated for a
two-cell body (§F-cleared below). A nine-cell body at four-cell spacing
gridlocks — that is dead-ends 775/829 and `climbs_over_kin`'s whole reason for
existing. Spacing must derive from the body plan's extent, not from a
constant, before a second species can be founded.

#### F18. The lab has no food verb, and hand-placed food is the one intervention known to reach Gate 0

`src/lab/ui.rs`'s `Tool` enum: `Look, Plant, Colony, Cull, Soil, Water, Keep,
Release`. **A tooling gap, listed because of what it is a gap in.**

The owner's standing direction, round three: *"If I have access to **food**,
water, can cull, can create plants, and creatures, I can figure it out."*
Food is named first and is the one thing on that list with no verb.

And it is not a convenience. `wiki/ants.md`: *"Put food on the ground beside a
nest and a colony breeds hard — thirteen generations deep inside a single
session... Leave it to forage for itself in the sealed lab bed and it picks
food up sixteen hundred times and brings it home four."* **The single
intervention that separates generation 13 from generation 0 is not available
to the player**, and the design guide's own verb table lists *feed/withhold*
as a verb with the note that *"the economy exists; withholding is how a
bottleneck is applied"*.

**Mechanism version.** A `Food` brush painting the material the species chip
has armed — `windfall`, `leaf`, `corpse` — which reuses the `Soil`/`Water`
paint path exactly. It changes no simulation code at all. It is also the
**control arm** F1 and F11 need: a bed with hand-placed food isolates
"foraging is broken" from "the economy is broken", which is the split §T2
says is still open.

#### F19. `COLONY_ANTS = 52` — withdrawn from the cleared list after review

`src/sim/creature.rs:1343`. **Shape 1, and it was cleared in error.**

The first draft cleared this as *"a placement tool… population thereafter is
births against deaths."* Its own doc is textbook shape 1 and the clearing row
did not read it:

> *"Grassé's threshold, in practice: **below about fifty, a colony looks
> broken even when the code is right**."*

A constant whose value was chosen so that a behaviour would *appear*. And the
concrete cost is the same asymmetry this report files as **F16**:
`lab/params.rs:525` exposes **`founders`** (how many plants) and
**`colonies`** (how many colonies) as tunable lab parameters, while the number
of ants *per* colony is a Rust constant. **A lab experimenter can set the plant
founder count and cannot set the animal founder count.** Founder population
size is the first independent variable Stage 5's Gate 2 ladder would want, and
it is unreachable from the game.

`design-philosophy.md` §2a settles it on the same sentence F16 quotes — *"a
constant graduates to a hot-reloadable `.ron` value immediately if a
non-programmer might plausibly want to tune it."*

**Mechanism version.** A parameter beside `founders` and `colonies`, defaulted
to 52. **Re-derive:** nothing, if defaulted — but see F17, since spacing must
derive from body extent before a non-ant species can be founded at any count.

#### F20. The worm is an entire animal implemented as nine constants and a hand-written branch

`assets/species/worm.ron` has **no `creature:` block at all**;
`src/sim/creature.rs:333` dispatches `None => worm_tick(..)`, and `worm_tick`
(`:378`–`:520`) is a hardcoded rule with **no brain, no genome, no traits and
no reproduction**. **Found in review; this is the audit's largest omission.**

It carries three of the four shapes at once:

- **Shape 1.** `WORM_HEAT_THRESHOLD_ABOVE_AMBIENT: f32 = 25.0`
  (`creature.rs:194`) is a constant deciding *when an animal flees* — the
  purest instance of shape 1 in the file. Eight more `WORM_*` constants
  (`:133`–`:194`) are the whole of its economy: tick interval, start energy,
  move and idle cost, burrow cost, moisture discount, energy from eating.
- **Shape 3.** The forage arm's preference is written as *"prefer burrowing
  into powder (**food**) over drifting through open space"* — a sense that has
  already decided what it is looking at.
- **Shape 4.** The flee branch precedes the forage branch unconditionally.

**What it costs, and it interacts with F17.** F17's stated unlock is that the
player could release worms and beetles as well as ants. **A released worm has
no genome to evolve** — it is not a creature in the sense the rest of this
report uses the word. So F17 delivers less than it promises for one of the two
species it names.

**How it was missed, which is worth recording.** This report cites
`creature.rs:500` in F12 — a line **inside `worm_tick`** — so the function was
read. The worm then appears nowhere else except F17's passing mention. Reading
a function for one constant and not auditing the function is exactly the
"expect the hardcoding one layer below where you are looking" failure the
brief warned about, arriving inside the audit written to catch it.

**Mechanism version.** Give `worm.ron` a `creature:` block and retire
`worm_tick`, so the worm runs the same sense/brain/act path as everything else.
That is not small: it is the migration `design-philosophy.md` §3 names as
deliberately deferred (*"whether moss and the worm get retrofitted onto the new
substrate immediately… or are left alone until they're touched anyway"*), so it
is a scoped project rather than a stage here. **Named, not staged.**

#### F21. Two sense constants that F14 would turn into silent trades

`SIGHT_RAYS = 16` and `SIGHT_EYE_LIFT = 1` (`creature.rs:761`, `:784`);
`CROWDING_RADIUS = 2` (`:791`). **Found in review. Neither filed nor cleared in
the first draft.**

The eye's angular *resolution* and its mounting height are Rust constants while
its *reach* is a species field that F14 stages as a heritable trait. An eye that
sees further at a fixed ray count sees a **coarser** world — so a heritable
`sight_range` behind a constant `SIGHT_RAYS` buys reach and silently loses
resolution, which is a trade nobody authored and nobody would see in a sweep of
`sight_range` alone. `CROWDING_RADIUS` is the same shape one channel over: a
sense's reach as a constant, in the channel whose own doc this report quotes
approvingly as *"the negative-feedback term, and it is not optional."*

**Mechanism version.** Not a gene — `SIGHT_RAYS` is a cost/resolution knob, not
a policy. The fix is that **F14's Stage 4 must scale rays with range** (or
state that it deliberately does not, and what that costs), so the trait means
one thing rather than two.

---

## 2. What was checked and **cleared** as legitimate substrate

This list matters as much as the findings. Sixteen entries, each examined
against the four shapes and passed, with the reason given so it is not
re-audited. **One entry was withdrawn after review** — `COLONY_ANTS`, which is
now F19 — and its neighbours in the same row survive on a narrower argument
than the one that cleared it, so the row says which.

| Cleared | Where | Why it is substrate |
|---|---|---|
| `FORAGE_REACH_BUCKETS`, `FORAGE_TRIP_MIN` | `creature.rs:229`, `:261` | **Telemetry, not policy.** Histogram bucket edges and a reporting bar; nothing downstream of them changes an animal's behaviour. `FORAGE_TRIP_MIN`'s doc sets its value from a measured control with headroom and explicitly says the profile, not the bar, is what to trust. Model conduct. |
| `COLONY_HALF_WIDTH`, `COLONY_ANT_SPACING` | `creature.rs:1338`–`1339` | **Placement geometry, and the spacing is physics.** Shoulder-to-shoulder gridlocks at 27,386 blocked ticks (dead-ends 775/829), so four apart for a two-cell body is a measured necessity. See F17: it must derive from body extent once a second species can be founded. **`COLONY_ANTS` was cleared here in the first draft and has been withdrawn to F19** — it failed the test this row applies, and the row did not notice. |
| `Lab::cull_at` | `lab/mod.rs:441` | **The verb the premise most depends on, and it is graded.** A plant is marked senescent and carried out by `rot_remains` at the species half-life; an animal's energy goes to zero and becomes a corpse that falls, rots, burns and is eaten. Neither leaves a hole. This is `CLAUDE.md`'s first law implemented correctly, and the two paths differ because the kingdoms differ, not because someone wrote two rules. |
| `choose_weighted`'s never-argmax property | `creature.rs:357` | **The noise is load-bearing** (P-10, `stigmergy-research.md` §2). Deterministic selection removes the exploration every trail-laying mechanism depends on, and removes it invisibly. Separately from F12: the *function* is right; only the fixed `k` is a finding. |
| `line_burrow` / `Material::packs_into` | `creature.rs:2544` | **Nothing whitelisted by name.** The ground says what it becomes; stone is untouched because it is not diggable, snow because it has no packed form. This is the pattern every other verb should copy. Its own doc even answers *which object does this rule evaluate* — the question `CLAUDE.md` says to ask. |
| `dig_force` against `penetration_resistance` | `creature.rs:2500` | **A contest, not a list.** "This is not a list of what ants may dig; it is a contest between how hard the ant pushes and how hard the material is." The *mechanism* is exemplary. Only its price is missing — F4. |
| `diet_yield`'s matched filter | `creature.rs:1933` | **A real trade with no free dimension.** One scalar on a bounded axis, squared falloff so small mis-specialisation survives and large does not; the per-class vector was considered and rejected because its magnitude would be a free dimension that drifts and reads as a result. Built on `food_value` so overlay, probe and mouth cannot disagree. |
| `birth_cost` / the body stamp | `creature.rs:1179` | **Conservation.** A parent pays for the meat its child is made of; nothing appears from nothing. This is exactly the "something must place a child's body and conserve the matter" substrate the brief exempts. That it is *large* is an ecology finding (`creature-stamp-routes-2026-08-30.md`), not a hardcoding. |
| `reproduce_at`'s floor at `birth_cost + 1` | `creature.rs:1224` | **A safety invariant.** A threshold below the cost is a species whose every birth kills its parent. It is not a tuning decision and `dead-ends.md`:1481 records that tuning under it does nothing at all. |
| `body_energy` pinned to `corpse.food_energy` | `organism.rs:2073`, pin at `:2298` and `creature.rs:1144` | **A ledger invariant.** Breaking it lets a predator eat a cell of flesh for more than the flesh cost to build — energy creation in a ledger asserted closed. Not a knob. |
| `MAX_ROOT_FRACTION` | `plant.rs:2018` | **Allometry, and its mirror was measured and reverted.** `MAX_SHOOT_FRACTION` cost ~13% biomass and bounded nothing (dead-ends :596). `dead-ends.md`:774 rates the root bound *"unconditional in principle; the fraction itself is a tunable"*. Cleared as substrate; making the fraction heritable is available and is not urgent. |
| `ROOT_BIAS_AT_FULL_WATER` | `plant.rs:4981`, consumed at `:6776` | **Already heritable** — `genotype(world, organism_id, 6, alloc_variance)` scales the whole allocation term. My initial suspicion was wrong. The residual is that the genome scales the *sum* rather than the set-point, so the shape of the stress response is fixed while its gain is not; that is a narrower finding than it first looked and is not worth a stage. |
| `LabBox` geometry (`DEFAULT_SOIL_DEPTH`, `FLOOR_ROWS`, `SHELL`, `CEILING`, `LAMP_*`) | `lab/scene.rs` | **Scene construction, exposed as parameters.** `soil_depth`, `ground_y`, `compartments`, `founders`, `colonies`, `lamp_spacing`, `seed` are all on the parameters page. Every constant carries its own measurement — `DEFAULT_SOIL_DEPTH`'s doc holds a 4×12 sweep and records that its *first* single-seed measurement said the opposite. Exemplary. |
| The oscillator divide-outs | `creature.rs:1808` (`noon_equivalent_light`), `:1815` (`noon_equivalent_temperature`), `plant.rs:4907` | **Method, correctly applied.** *"A brain input that drifts with the hour is a brain input every evolved behaviour is silently conditioned on the time of day."* Exactly right. |
| `scaled_cells`, `CreatureDef::scaled`, `BodyPlan::scaled` | `creature.rs:210`, `organism.rs:2307`ff | **Resolution invariance, and it is hard-won.** One read of `cell_scale` in one place; `sight_fraction` gets its own row because it is the only term scaling on two axes; the anchor bug and its two paired guards are in `dead-ends.md`:1493. Substrate. |
| `is_living_kin` | `creature.rs:2018` | **The difference stated as data, per `CLAUDE.md`.** The diet axis provably cannot separate a live nestmate from a starved nestmate's corpse — same class, same number — so the distinction is carried by whether the cell has an organism id. Live tissue belongs to somebody; carrion belongs to nobody. This is the rule the whole audit is asking other rules to follow. |
| `brain.rs`'s reserved positional layout | `brain.rs:37`ff | **The thing that makes every append in this report cheap.** Slots are positional and reserved; appending lights a row that is already zero, so no other weight moves. Several findings above are affordable *only* because this exists. |

---

## 3. Two things I expected to find and did not

Recorded because a cleared expectation is worth as much as a finding.

**The plant genome is in better shape than the creature genome, by a wide
margin.** Ten continuous slots, six discrete loci with allele tables, and
heritable fates, against two scalars and a wiring matrix. Both discrete-locus
mechanisms trade honestly — `LEAF_RATE_ALLELES` is paired with
`LEAF_TRANSPIRATION_ALLELES` at every consumer *"because a free rate axis
would be selection candy with no bill attached"*, and `WOOD_DENSITY_ALLELES`
scales strength and price with one number *"so tuning cannot quietly turn the
trade into a free lunch"*. That is the standard the creature side has not
reached, and it is the standard, so F14's staging should copy it rather than
invent one.

**Genome slot 9 is a live-width slot with no consumer** (`organism.rs:3440`).
Its own doc says so and explains the choice — appended rather than
re-purposed, to keep the megastudy's numbering comparable. It is a writer with
no reader, which `dead-ends.md` calls *"the failure mode this project has hit
three times"*, and it is **not** a finding here because it is recorded, argued
and deliberate. It is named as the obvious home for F15's `seed_cost`.

---

## 4. Ranking by leverage

Leverage = *how much behaviour becomes reachable that is not reachable today*,
against cost and against what has to be re-derived. Not effort order — §5 is
effort order.

| # | Finding | Unlocks | Cost |
|---|---|---|---|
| — | ~~**F2** `hunger_fraction` decides eat-versus-carry~~ | **CLOSED by PR #190** — the crop. Was ranked #1 | — |
| **1** | **F1** `moisture_gradient` is magnitude-only, coarse, and un-genomed | The route-drop rule, which is what §T2 measured. **Demoted from #1 in review**: one of its four defects was false, and the step it ranked first (the per-cell read) is the one least likely to move `deliveries` | Small code, real re-derivation across six species files |
| **2** | **F14** `CREATURE_TRAITS = 3` | Everything the lab claims to be about. Nothing else on this list makes an *animal* evolvable; this is what does | Must be staged behind F4 and F6 or each new trait lands as a ratchet. **Add `body_energy`**, and take `crop_capacity` first — it is the one slot whose preconditions are both already met |
| **4** | **F18** no food verb | The player's own experiments, and the control arm that separates "foraging is broken" from "the economy is broken" | Trivial — it reuses the `Soil`/`Water` paint path and touches no simulation code |
| **5** | **F11** `Feed` conflates eat with take | A grazer against a hoarder; the granary stops being self-consuming | Cheap, because the append is lawful |
| **13** | **F3** reproduction is opt-in | Predator-prey population dynamics, which is the biosphere's first real feedback loop. **Dropped from #6 in review**: the finding stands, but its one-line fix is a recorded dead end (`dead-ends.md`:1543), violates the field's own invariant, and does not give the beetle a child | A species-file change with a sweep behind it, read on `live` and never on `births` |
| **7** | **F9** the eye is a food detector | Predator *avoidance*, and every non-feeding use of vision | Medium — **but gated on F13, not F14 as the first draft said**, and F13 is unstaged because its fix is unsolved. Treat this rank as conditional |
| **8** | **F4** digging is free | Burrower against forager as a real trade; unblocks a heritable `dig_force` | Small, no new constant |
| **6** | **F13** `Turn` is inert on the flat | The beetle's eye, in the lab specifically. Already filed as §R4. **Promoted from #9 in review**, because F9 and `dead-ends.md`:1545 both turn out to depend on it | Unknown — §R4 says the fix is not obvious. Start with the counter |
| **10** | **F8** `FoodAdjacent` is a boolean | Conditioning on food quality — the quantity Gate 0's arithmetic turns on | Small code, **six authored weights** to re-derive across two species |
| **11** | **F15** plant reproduction is a species constant | Selection on plant life history; the lab's food supply becoming a thing that can improve | Medium; `affordable` changes meaning |
| **12** | **F17** `COLONY` is hardcoded to `"ant"` | A measured route to Gate 0 that is blocked by a string literal | Engine: trivial. Interface: the bar is full |
| **13** | **F6** `sight_fraction` unarmed | Closes a ratchet the code already prices | One authored number, derived like `idle_cost_per_cell` |
| **14** | **F5** emitting is free | Quiet scouting as a strategy | Small, **but it can delete homing** — default to 0 |
| — | ~~**F7** budding has no verb~~ | **PARTLY CLOSED by PR #192** — `TRAIT_REPRODUCE_AT`. What survives is the *conditional* half: a trait cannot hold back through a lean patch, only breed later on average | Cheap; lawful append, and smaller now |
| **16** | **F16** plant mutation rates are `const` | The mid-game mutagen tier the owner has already decided on | Trivial, plus a correction to the design guide |
| **17** | **F10** `EAT_YIELD_THRESHOLD` | Removes a hard edge from a continuous model; currently decides whether an omnivore is viable | **Blocked on WP-8's sweep.** Do not touch before it |
| **18** | **F12** `CHOICE_EXPLORATION_K` | Boldness as a gene | **Larger than first costed** — it reallocates against `PERSIST_MAX`, `FOOTING_MAX` and a bare `0.5`, none of which is a gene. Most likely entry here to break something that works. Last |
| — | **F19** `COLONY_ANTS` | The animal founder count, which Gate 2's ladder wants as its first independent variable | One parameter row. Withdrawn from the cleared list in review |
| — | **F20** the worm has no genome | Nothing, until it is retrofitted — but it is why F17 delivers less than it promises | A scoped migration, not a stage. Named only |
| — | **F21** `SIGHT_RAYS` / `CROWDING_RADIUS` | Nothing on its own; it stops F14's `sight_range` trait meaning two things at once | Absorbed into Stage 4 |

**The story is now two entries, not three, because the middle one shipped.**
It was: F1 makes the route-drop rule sane, F2 makes a larder convertible into a
child, F14 makes the child able to differ from its parent in something a player
can see. **PR #190 built F2**, so what remains either side of it is F1 and F14
— and F14 is the cheaper of the two for the first time, because the crop it
needed now exists.

**Stage 0's control arm still comes first regardless**, and more so than
before: with the crop landed, nobody has yet measured whether the colony can
maintain a larder *now*, and F1's whole cost argument rests on a §T2 figure
taken against the pre-crop economy. **That number needs re-taking before F1 is
worth building.** It is the clearest instance of this document's own rule —
size a problem at the moment it starts, not after the system has responded.

---

## 5. The staged plan

Each stage names what must be **true and measured**, not what must be done —
the design guide's own gate discipline. A stage that cannot be measured is not
a stage.

Two standing rules for every stage below, both from `CLAUDE.md` and both
already paid for in this area:

- **Watch the guard go red for its own fault before citing its green.** §T2's
  own guard was measured *vacuous* and `examples/ascii.rs` says so in place;
  the successor ratio is deliberately not an assertion. Any new guard here
  starts in the same suspicion.
- **The bed is procedural and the outcomes are chaotic.** Six seeds is not a
  sweep — `dead-ends.md` records 1.64× over six seeds and 1.08× over the next
  twelve, pooling to a per-seed median of zero. Gate on an order statistic.

### Stage 0 — the control arm, before anything (F18)

**True when:** the lab has a `FOOD` brush, and a bed with hand-placed food
beside the nest reaches a generation depth the same bed without it does not.

Trivial, touches no simulation code, and it is the instrument every later
stage is read against. `wiki/ants.md` already says the two arms differ by
thirteen generations; nobody has been able to *run* that arm in the lab.

### Stage 1 — the ant can maintain a larder (F1)

**True when:** in the sealed lab bed, `deliveries` and the standing nest
larder are both non-zero at 24,000 frames, over an order statistic across
≥12 world seeds.

The baseline is §T2's: 1,651 pickups, **4** deliveries, **0** larder cells.

Steps, in order — **reversed after review**, which found the first draft's two
leading steps the wrong way round and its PR #185 analogy pointing the wrong
way (there the channel was flat where it was needed; here it is saturated where
it fires):
1. **The signed component.** Nothing else. Measure. This is the step that
   addresses what §T2 measured — a magnitude saturating along the whole route.
2. Only then, point `creature::moisture_gradient` at `soil_water_fraction`.
   Measure separately, and **do not expect it to move `deliveries`**: a
   per-cell read returns a true `0.0` for air, so it gives a *sharper*
   boundary magnitude, not a softer one.
3. Only then, the brain input, authored at the ant's implied weights so
   generation zero is byte-identical.

**Guard, watched red:** a bed with a real, *signed* moisture gradient across
it, asserting that drops concentrate on the drying side. `ascii`'s `uphill`
ratio is the shape; what it lacks is drops to average over, and Stage 0 is
what supplies them. **Re-take `(Bias, Dig, 0.4)` and `(Carrying, Drop, 0.2)`
in the same stage** — they are calibrated against a near-1.0 multiplier.

### Stage 2 — an ant reaches generation 2 without hand-feeding (F2, then F11)

**True when:** `creature_probe` reports non-zero `births` and deepest
generation ≥ 2 in the lab bed with **no hand-placed food**. This is Gate 0,
and Gate 0 is satisfied by any route (design guide §3).

The crop first, `Take` second — `Take` is a much smaller change once
eat-versus-carry is no longer a branch on a constant.

**The control arm is `deliveries`**, because that is the quantity
`dead-ends.md`:904's naive version destroyed (14 eats, 8 pickups, **zero**
deliveries). A crop that reproduces that number has reproduced the dead end.

**Costed re-derivation, and it is the reason this stage is large — the first
draft named six consumers and there are at least thirteen.** Beyond the six
below: **seven example harnesses** read `hunger_fraction`
(`creature_probe`, `windfall_probe`, `stamp_probe`, `lab_cost`,
`creature_space`, `filmstrip`, `predation_probe`) — and two of those are this
audit's own instruments, since `creature_probe`'s reachability line **is
Stage 2's stated gate** and `windfall_probe` is the harness §T2's reproduction
command runs. Worse, there is a **lib guard**: `creature.rs:7409` computes
`let roof = def.hunger_fraction * def.start_energy + diet_yield(..)` and
asserts `route_peak <= roof` and `nest_peak > roof`. A crop deletes that roof,
so unless the guard is rewritten it becomes `CLAUDE.md`'s *"a superseded
mechanism's tests keep passing while testing nothing"* — it will stay green
and assert nothing. Watch it go red for its own fault before and after.

The six originally named: `BreedMargin` (`lab/stats.rs:1142`), the larder
histogram's hunger line (`:1070`), `app.rs:1153`, the parameters row
(`lab/params.rs:449`/`:477`/`:570`) and `world.rs:357`'s ceiling doc. **Budget
all thirteen into the stage, or — in this report's own words — it is not
scoped, it is merely started.**

### Stage 3 — the two ratchets, so Stage 4 has somewhere to stand (F4, F6, F5)

**True when:** `dig_force` and `sight_range` each have a measured **interior
optimum** — a best setting that is neither the floor nor the cap.

**Not "a setting at which more is worse", which is what the first draft said
and is gameable.** `sight_tax` scales with `sight_cells_read`, which scales
with range, so a large enough `sight_fraction` makes "more is worse" true by
arithmetic without demonstrating any trade at all. An interior optimum cannot
be manufactured that way, and it is the real difference between a gene and a
ratchet.

Price digging against `penetration_resistance`. Author `beetle.ron`'s
`sight_fraction`. Add `emit_fraction`, defaulted to 0, and sweep it against
`deliveries` before arming it.

**Sequencing note:** `dead-ends.md`:1545's `(PreyNear, Persist)` sweep sits in
the same weighted sum as the sight cost and is held pending §R4. Do not change
both in one diff.

### Stage 4 — an animal can differ from its parent in something visible (F14)

**True when:** two lineages from one founder, run in the same box, differ in a
trait a player can *see* — and the difference persists across generations
rather than one absorbing the other.

Append `TRAIT_DIG_FORCE` and `TRAIT_SIGHT_RANGE`, each behind its Stage-3
price — and **scale `SIGHT_RAYS` with `sight_range` in the same change, or say
what not doing so costs** (F21: otherwise the trait buys reach and silently
loses angular resolution, and a sweep of the trait alone cannot see it).

**Then `body_energy`, which the first draft's staging omitted and should not
have.** `dead-ends.md`:1478 and :1517 measured it as *the invariant term* that
blocks births — *"the stamp term (480 x 2 = 960) is invariant to both levers:
even at a grant of zero the bar is 961 against a ceiling of 567."* It is the
one body scalar measurement has already identified as the blocker, so on this
report's own logic it belongs ahead of `TRAIT_SIGHT_RANGE`. It is also the
hardest: it is pinned to `corpse.food_energy` by the ledger invariant §2
clears, so a heritable `body_energy` needs the *material's* value to move with
it, or flesh becomes worth more than it cost to build.

Then the first creature **discrete locus**, copying the plant side's
paired-trade discipline: one number scaling a benefit and its bill together.

`CreatureDef::scaled` needs a row per trait that is a length or a rate. PR
#188's `sight_fraction` row is the worked example, including why dividing by
`burn` is the easy mistake.

### Stage 5 — Gate 2, which has still never been run

**True when:** the `selection_arena` `arm=` ladder has been run **in the lab
bed** and discrimination has been located.

This is not new work I am proposing; it is the design guide's own Gate 2 and
the coordinator note's item 3, and it is listed here because **everything
above it is unvalidated until it passes.** `selection_arena`'s whole finding
is that a null there is a statement about the world rather than the genome.

`evolution-lab-genetics-2026-08-31.md` §5 removed the last blocker: a
founder's genome used to be keyed on `(world seed, germination coordinate)`,
so two runs of "the same" experiment started from different plants. The
specimen shelf holds a genome while the world changes, which is the control
arm Gate 2 was missing. **Run it now rather than later** — the coordinator
note already says so.

### Not staged, deliberately

- **F13 (§R4)** — the fix is a movement-model change and wants its own design
  pass. Do the counter first: how often is a `Turn` request discarded because
  the side it asked for scored zero. In the lab bed that number *is* the
  finding.
- **F10 (`EAT_YIELD_THRESHOLD`)** — blocked on WP-8's survival-versus-gut
  sweep, which the constant's own doc asks for.
- **F12 (`CHOICE_EXPLORATION_K`)** — cheap, real, and the most likely entry
  here to move something currently working. Last, and only with `moves_blocked`
  and `falls` in front of you.
- **F17 (`COLONY` species)** — the engine half is a parameter; the interface
  half needs a third bar row, a page, or a removal, and three attempts are in
  `dead-ends.md`. It is an interface decision for the owner, not a lane task.

---

## 6. Scope, and what this audit did not cover

`Reports/two-games-one-repo-2026-08-30.md` settles that the lab's speed comes
from what is not in the *box*, not from what is not in the binary — every
outdoor system is linked and costs ~0 when there is nothing for it to do. The
same argument scopes this audit: `structural.rs`, `load.rs`, `rigid.rs`,
`explosion.rs`, `player.rs` and `worldgen/` are **present in the lab binary
and never reached by a lab scene**, so a mechanism-versus-behaviour finding in
them cannot change what the lab can evolve. They were not audited. Collapsing
tunnels are separately an owner decision (design guide §2b) and the lab
declines the structural purchase outright.

**Three that are lab-reachable and were not audited, named so the gap is
visible rather than implied:**

- **`liquid.rs`** — the `WATER` brush reaches it, and water is the only thing
  that threatens a burrow (`wiki/ants.md`: *"staying dry is the whole of what
  a colony is defending"*). Not audited.
- **`fire.rs`** — no lab verb reaches it today, but the design guide's
  equipment layer wants one, and fire is what makes corpses.
- **`weather.rs` / `field.rs`'s momentum passes** — the design guide's §2c
  finding is that *a fan is weather*, and the equipment layer is the strongest
  argument for the air simulation existing at all. Nothing in the lab drives
  it yet, so there is no policy there to audit; there will be.

**One thing this audit could not do.** Nothing here was measured by this
session. Every quantity is cited from a report or read off the source. So
each finding is a claim about *what the code can express*, which is checkable
by reading, and **not** a claim about what would happen if it were changed —
which is what the stages exist to establish. The distinction matters most for
F1 and F2, where the arithmetic is compelling and the arithmetic has been
compelling and wrong in this area before.


---

## 7. The audit was overtaken while it was being written

**Recorded because it is the most transferable thing in the document, and
because it happened to the report that warns about it.**

This audit was read off `943ace17`. By the time it was reviewed and corrected,
`main` was **33 commits ahead**; by the time the corrections landed, **41**.
Two of the twenty-one findings had been closed by other lanes in that window,
and **both were in the top seven of §4's own ranking**:

| | closed by | what shipped |
|---|---|---|
| **F2** the eat-versus-carry threshold | **PR #190**, same day | the crop. `hunger_fraction` is not merely unread — the field is **gone** |
| **F7** when to breed | **PR #192**, same day | `TRAIT_REPRODUCE_AT`, a heritable trait rather than the brain output proposed here |

**Nobody did anything wrong, and that is the point.** Two lanes were working
the creature economy concurrently; the audit's own §1 says so, and
`branchcheck` printed the branch list at the start of every session. What was
missing is that **a finding is a claim about a moment**, and this document did
not carry the date of its own baseline anywhere a reader would trip over it.
Had #191 landed as first written, it would have sent the next session to build
a crop that already existed — which is exactly the waste `Reports/dead-ends.md`
exists to prevent, arriving through a document rather than through a retry.

**`CLAUDE.md` already has this rule and it is filed under files, not
findings.** *"A file-ownership split is only as current as your last look at
the branch list… read once at session start it is stale within the hour, and
nothing prompts a re-read."* Measured there on a three-lane creature split
where Lane A filed four bug entries Lane B had already filed, better. **The
same decay applies to an audit's findings**, and it is worse there, because a
duplicate bug entry is visible in a merge conflict and a stale finding is not:
it merges cleanly and reads as work to do.

**So, for any document that inventories the state of the code:**

- **Stamp the baseline commit in the status block**, not just the date. This
  one now does.
- **Re-verify before landing, not before writing.** The check is one `grep`
  per finding against `origin/main` and it took a single tool call for
  nineteen of them. Cheap enough that there is no excuse for skipping it, and
  the only step that would have caught this.
- **Expect the overlap to be highest exactly where the document is most
  useful.** The two findings that were closed were ranked #1 and #6 — because
  a finding that matters is a finding somebody else is also looking at.

Three of §4's remaining entries are cheaper than when they were written, for
the same reason: the crop makes `TRAIT_CROP_CAPACITY` a priced trade with a
built mechanism (F14 step 0), and `TRAIT_REPRODUCE_AT`'s own doc settles F3's
status explicitly rather than leaving it inferred.
