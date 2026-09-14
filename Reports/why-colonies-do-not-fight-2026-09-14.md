# Why two colonies in one box never fight, and what each missing piece is worth

*Evolution lab, round 35, Lane C. The owner's ask, in his words: "review/explore
why we don't have different colonies fighting or eating each other." A
review-and-explore brief — the deliverable is the explanation and the evidence,
not a bed with a war in it. Written 2026-09-14. Harness:
`examples/rivalry.rs`. Lane note: `Reports/lanes/evolution-lab-rivalry.md`.*

## The answer in four lines

**Nothing in this engine is missing a fight. What is missing is a stranger.**

1. `CreatureDef::scent_spread` is **0**, so every colony of a kind founds at
   one point in scent space and `creature::is_living_kin` is true for every
   ant-to-ant pair in the box. `nearest_foe` skips kin and checks nothing
   else, so **no ant on the shipped bed has a target**, whatever its brain
   wants and however far it can see.
2. Make them strangers and **they fight and eat each other with no other
   change at all** — no eye, no new wire, no species file edited. Measured
   below.
3. The initiation everyone has been looking for is **already there, and it is
   the mouth rather than the jaw**: a stranger is not excluded by
   `adjacent_food`'s kin filter, so an ant *eats* it; the swallow calls
   `cry_alarm`; and `ant.ron`'s shipped `(Alarm, Attack, 2.0)` turns the bite
   into a brawl. Predation is the ignition the combat layer was said to lack.
4. The one dial is `scent_spread`, it is a **species field with no
   player-facing route**, and it is **seed-dependent**: the offsets are drawn
   uniform per slot, so whether two colonies clear the tolerance radius is a
   draw rather than a setting.

**So the honest answer to "why don't colonies fight" is not "three features
are missing". It is "one number is zero, and it is zero on purpose."** The
retired `colony rivalry` switch did exactly this and nothing else, and it was
retired *into* this number
(`CreatureDef::scent_spread`'s own doc, and `organism::TRAIT_TOLERANCE`'s).

## What is wrong in the standing account

`Reports/held-world-game-concept-2026-09-13.md` §10a is the document the brief
starts from, and it is right about the shape and wrong about the cause:

> *"The engine has a fully working fight and no way to start one. … The reason
> is a single structural gap: the shipped ant is blind and its only route to
> `Attack` is `Alarm`, which only a landed bite can raise."*

Both clauses are true of the code and **neither is the binding constraint**,
which is the part that matters: give the ant an eye and the initiation wire
and it still never fights, because there is nobody it is allowed to fight.
§10a never names `scent_spread`. Measured below as the `VW` arm — both of
§10a's gaps closed together, and the attack count does not move.

`Reports/creature-groups-and-combat-design-2026-09-06.md`'s §0 table is wrong
in the other direction, and more simply:

> | do ant colonies ever attack each other? | **Yes, by default, as of
> 2026-09-06.** … Rivalry is retired for the heritable scent of §3, which is
> on by default, and `BrainOutput::Attack` is the aggression verb — wired in
> `ant.ron` and shipped on |

The *mechanism* ships on. The *dial that makes it reachable* ships at zero.
A reader of that row would expect the played bed to show colony fights; it
shows none, on every seed measured here.

## The chain, link by link, against the code

`creature::nearest_foe` is the whole gate. It is `Attack`'s own walk of the
attacker's body ring, and every cell it looks at must clear three tests:

```rust
if owner == 0 || owner == organism || world.organism(owner).is_none() { continue; }
if is_living_kin(world, cell, gut) { continue; }
return Some((nx, ny));
```

and `is_living_kin` is

```rust
(gut.crosses_kinds || s.species == gut.species)
    && scent_distance_sq(&scent_of(&expressed_traits(s, ..)), &gut.scent) <= gut.tolerance_sq
```

**The colony label is not consulted at all.** Kin is *species and scent*,
full stop — as `Reports/creature-signature-and-castes-2026-09-06.md` says of
its own build ("with the colony label nowhere in it"); the colony clause in
that report's §0 table describes the **retired** switch, and reading it as
current is an easy mistake this lane's brief made. The label
enters only once, at founding, through `creature::colony_scent_offset` — and
that function opens `if spread <= 0.0 || colony == 0 { return [0.0; 3] }`. So
at the shipped `scent_spread: 0` every colony's offset is the zero vector,
every ant's signature is the species' authored point, every distance is 0,
every radius contains it, and every ant is every ant's family. **`nearest_foe`
returns `None` for every animal in the bed by construction.**

### Link 2, the eye — real, and not on this path

`sight_range` is 0 on every ant variant (`ancestor.ron` authors 32, `beetle`
64, `flitter` 32; `ant`, `ant_block`, `ant_block_shaded`, `ant_long`,
`ant_wide`, `longant`, `chitin_pale`, `hopper` author none). `creature::
sighted` opens `if reach <= 0 { return (Sightings::default(), 0) }`, so
`ThreatNear`, `ThreatBearing`, `PreyNear`, `KinNear` and `BloomNear` all read
a hard 0.0 on every shipped ant tick.

**But `nearest_foe` does not read the eye.** It walks the 8-neighbourhood of
the attacker's own chain. Sight decides whether an ant can *approach* a rival
across the box; it has no bearing at all on whether an ant standing next to
one will bite it. So the eye is a real gap for *finding* a war and cannot be
the reason there isn't one.

### Link 3, initiation — it exists, and it is the mouth

`ant.ron` wires exactly one edge into `Attack`: `(Alarm, Attack, 2.0)`.
`Channel::Alarm` is written by `creature::cry_alarm` and by nothing else in
the sim (`lab::Lab::alarm_at` is the player's tool). `cry_alarm` has **three**
call sites, and only the first is the fight:

| site | what raises it |
|---|---|
| `creature.rs:6696` | a bite from the `Attack` verb landed |
| `creature.rs:6881` | **the mouth gnawing a living cell it could not swallow whole** |
| `creature.rs:6921` | **the mouth swallowing a living cell outright** |

The second and third are the feeding path, not the fighting path. So the
sequence that starts a war already ships:

> a stranger stands in reach → `adjacent_food` does not exclude it, because
> `is_living_kin` is false and `diet_yield` prices flesh (the ant's
> `TRAIT_GUT_BIAS` is authored **0.0**, a neutral omnivore) → the ant eats it
> → `cry_alarm` → `Alarm` → `(Alarm, Attack, 2.0)` → everyone nearby swings.

**Measured, in `examples/rivalry.rs control=selftest`**: two colonies pulled
apart in scent with **no wire added and no eye given** report 13 attacks, 13
cells and 8 kills over 6,000 frames, against 0/0/0 at the shipped dials in the
identical box. `ThreatNear -> Attack` is not the missing initiator; it would
be a *second* one.

### The fourth thing: `attacks` is not a fighting counter, and on the played bed it is entirely plants

`nearest_foe` skips kin and checks **nothing else** — in particular it never
asks whether the target is an animal. A plant cell is an organism, is never an
ant's kin (different species, `kin_crosses_kinds` off), and stands in the ring
of any ant gnawing a stem. So a plant is a valid foe, and the same `cry_alarm`
in the feeding path that would start a war also fires when an ant bites a
leaf.

The played bed therefore reports a busy `Attack` verb that has nothing to do
with anybody fighting:

| seed | attacks | cells taken | kills | animal-vs-animal kills |
|---|---|---|---|---|
| 1 | 475 | 79 | 0 | 0 |
| 2 | 407 | 86 | 0 | 0 |
| 3 | 344 | 58 | 0 | 0 |

and the control that says so is one command — the same bed with the plants
taken out (`founders=0`), where the ants still meet, still starve and still
scavenge each other's corpses:

| seed | attacks | deaths | starvation share |
|---|---|---|---|
| 1 | **0** | 94 | 100.0% |
| 2 | **0** | 94 | 97.9% |

**`attacks` falls to exactly zero when the plants are removed.** Every swing
on the played bed is an ant biting a plant.

This matters beyond this report in two ways. §10a's headline — *"on the bed
the owner plays it reports `attacks 0`"* — does not reproduce here at all;
whatever produced that zero, this bed at 24,000 frames on three seeds reports
344–475. And it is another instance of `CLAUDE.md`'s worst-recurring failure:
`attacks` is arithmetically correct, has always been correct, and answers
"how often did the `Attack` branch reach a target" rather than "did anything
fight". Filed as `Reports/open-bugs-handoff.md` §Z23.

## "Eating each other" is a different question, and the answer is: they already do

The brief's guess is right and it is worth stating plainly, because it changes
what wants building.

A corpse is a **`Powder`**, not a `Creature` cell. It carries no organism id,
so it carries no species and no colony. Nothing in `adjacent_food`,
`diet_yield` or the swallow path asks whose it was — `is_living_kin` is a test
on *living* tissue and never runs. **Any ant will eat any corpse, including
one from another colony, and that is true on the shipped bed today.**

The `noplants` control above is the proof, and it is a clean one: a box with
**no plant in it at all**, so the only thing in the world with a food value is
carrion, reports `corpse_j` of **570 / 1,368 / 1,163 J** on three seeds. On
the played bed, with a full larder standing, it is still **456 / 1,248 / 456 J**
against 11,911–23,450 J of plant. So the colony is scavenging throughout; it
is simply 2–4% of intake and nothing anywhere says it happened.

**It is a readout gap, not a mechanism gap.** `EnergyLedger::harvested_corpse`
is a single global `f64` split from `harvested_plant` only by
`Material::worth_in_aux` — one number for the whole world, for both colonies,
for their own dead and each other's. Kills are attributed and carrion is not:
`World::kills_log` carries `(victim_species, victim_colony, attacker_species,
attacker_colony)` per kill and `GroupDeaths::killed_by` tallies them, so "ANT 2
killed 6 of ANT 1" is already answerable. "ANT 2 *ate* 6 of ANT 1" is not,
by anybody, today.

**For Lane B's per-colony ledger, the account that would reveal this is
`harvested_corpse`, and it needs one thing the others do not: the carrion has
to carry an identity to be attributed to.** The eater's colony is known at the
swallow (`creature.rs:4057` has the organism in hand), so a per-colony
`harvested_corpse` is cheap and answers *"which colony scavenges"*. Answering
*"whose dead did it eat"* needs the corpse stamp to keep the victim's group,
which `stamp_as_corpse` does not record — that is a real change to
`creature.rs`, not a ledger split, and it is Lane B's call whether the first
half is worth having on its own. **It is**: "ANT 1 lives 40% off carrion" is a
finding a player can read, and it does not need to know whose.

## How this was measured

`examples/rivalry.rs`, the played lab bed (`LabBox` at its defaults, two
colonies of 52, six seeds, 24,000 frames). **24,000 rather than 9,000** for
`creature_arena`'s reason: an ant's founding grant is `start_energy /
(idle_cost_per_cell x cells)` ≈ **12,000 frames**, so inside a shorter window
not spending is strictly better than earning and the bed has no teeth. The
arms are run-time overrides — no species file was edited, so the
`include_str!` trap (three bit-identical "sweeps" on record here) cannot
apply, and every knob is echoed on the run's first line.

| column | what it is | why it is not `attacks` |
|---|---|---|
| `between%` / `within%` | share of living animal pairs that are mutually outside each other's tolerance, **split by whether the pair share a colony label** | two colonies becoming foreign to each other, and a colony eating itself, are opposite findings that a pooled figure cannot tell apart |
| `gap` | mean scent distance between members of *different* colonies, against a tolerance radius of 1.0 | this is what explains a null: at `scent_spread = 1` the per-colony offsets are a **draw**, so a seed can land inside the radius and the dial does nothing |
| `contacts` / `cross` | living animals standing in each other's `nearest_foe` ring, and the subset that are mutually strangers | the **opportunity** count — it separates "they never meet" from "they meet and do not bite", which `attacks == 0` cannot |
| `xcol` / `own` / `plantkill` | kills from `World::kills_log`, split by whether attacker and victim shared a colony, and by whether the victim was an animal at all | **"colonies fighting" is `xcol`.** `attacks` is not a fighting counter — see §Z23 |
| `starv%`, deaths by cause | the owner's bed reports every death as starvation | if closing a gap only changes *which* deaths happen, the bed has gained a new way to die rather than a war |
| `corpse_j` / `plant_j` | `EnergyLedger`, for the eating half | scavenging is not fighting and is measured separately |

**Both controls run in one command**, `control=selftest`, and two of the four
arms began as predictions this harness falsified — the prediction that a
stranger pair would not fight without a wire (it does, 13 attacks), and the
prediction that `cross` would be positive whenever fighting happened (a
cross-contact is *consumed* by the fight that follows it, so a sampled census
of a standing state is the wrong instrument and `cross` is reported, never
asserted).

`scent_drift` is pinned to 0 in every control arm, and that is load-bearing
rather than tidy: see the lane note's trap on `rivalry=1`.

## The factorial: each link alone, each pair, all three

Eight arms plus two controls, **six seeds each, 24,000 frames**, the played
bed with two colonies. `S` is `scent_spread = 1`, `V` is the sight allele at
its ceiling (**a resolved reach of 64 cells**, not the number on the command
line), `W` is `ThreatNear -> Attack` at 2.0. `xcol` is cross-colony kills —
the only column in this harness that means *colonies fought*.

### Cross-colony kills, every arm, every seed

| arm | s1 | s2 | s3 | s4 | s5 | s6 |
|---|---|---|---|---|---|---|
| base (shipped) | 0 | 0 | 0 | 0 | 0 | 0 |
| noplants (control) | 0 | 0 | 0 | 0 | 0 | 0 |
| **V** — sight alone | 0 | 0 | 0 | 0 | 0 | 0 |
| **W** — the wire alone | 0 | 0 | 0 | 0 | 0 | 0 |
| **VW** — §10a's two gaps, both closed | 0 | 0 | 0 | 0 | 0 | 0 |
| **S** — scent alone | 0 | **9** | **8** | **9** | 0 | **12** |
| **SV** | 0 | **8** | **6** | **9** | 0 | **8** |
| **SW** | 0 | **6** | **6** | **13** | 0 | **10** |
| **SVW** | 0 | **4** | **8** | **6** | 0 | — |

**`S` is necessary and it is sufficient. Nothing else is either.** Every arm
containing `S` fights; no arm without it does, on any seed. `VW` — the two
gaps the standing account names, closed together — is **0 on six seeds of
six**, which is the measurement that settles what this report is for.

**They are not multiplicative and they are not even additive.** `SV`, `SW`
and `SVW` are not above `S`; if anything the three-way arm is below it. An
eye is a per-tick energy charge on every animal (`sight_fraction` bills per
cell read) and a wire is a synapse charged every tick, so both arms pay for a
capability in a bed whose binding constraint is food. There is no interaction
term here to find.

### Why seeds 1 and 5 are zero, and what that says about the dial

Not noise. The per-colony offsets are **drawn**, uniform in `-spread..=spread`
per slot, so whether two colonies clear the ancestral tolerance radius of
**1.0** is a property of the seed:

| seed | founding gap | strangers between colonies | xcol |
|---|---|---|---|
| 1 | 0.907 | 0.00% | 0 |
| 2 | 1.710 | 100.00% | 9 |
| 3 | 2.237 | 100.00% | 8 |
| 4 | 1.658 | 100.00% | 9 |
| 5 | 0.976 | 0.00% | 0 |
| 6 | 1.140 | 100.00% | 12 |

**The threshold is exact and the outcome is binary**: above 1.0 every pair
across the two colonies is a stranger, below it none is. Two seeds of six
land under. So at `scent_spread = 1` **a third of beds are still one family**,
and there is no value anyone can author that means "rival colonies" — which is
the argument for exposing the dial rather than picking a number for it.

On seed 1 the `S` arm is **byte-identical to its baseline** — same attacks,
same cells, same deaths, same survivors. That is `CLAUDE.md`'s stale-binary
tell with an innocent cause, and it is worth stating as a positive finding:
**moving an ant's scent changes nothing whatever unless it crosses somebody's
tolerance.** The `gap` column is the only thing that tells the two cases
apart, which is why the harness prints it.

### What a colony keeps while it fights

`within%` — strangers *inside* a colony — is **0.00% on every arm and every
seed, at every sample**, while `between%` sits at 100%. Two cohesive families,
foreign to each other. That is an independent confirmation of the nest-cohesion
build's central claim (`ant.ron`'s `scent_drift` comment: *"no setting of this
dial can make a cohered nest eat itself"*) from a direction it was not measured
from: the shipped `scent_drift: 0.15` is running throughout, and no colony ate
itself on any of 54 runs.

### What it costs the bed — and this is the part to be honest about

Paired against the same seeds' baselines, on the four seeds that separate:

| | base | S |
|---|---|---|
| median alive at 24,000 | 6 | 7.5 |
| median deaths | 690 | 747 |
| median starvation share | 30.7% | 24.6% |
| median births | 2.5 | 4 |

**No population effect is detectable here, and the report should not claim
one.** Eight to twelve cross-colony kills against 550–900 deaths is **1–2% of
mortality**; the spread between seeds of the *same* arm is far larger than the
difference between arms, and four seeds cannot separate them. What the numbers
say is narrower and is the useful claim: **making colonies strangers adds a
visible event, not an ecological force.** The bed still starves, the deaths
are still mostly starvation, and the colony that loses a fight loses about one
ant in ten of what hunger takes from it.

That is not an argument against it. `CLAUDE.md`'s first law is that an outcome
should be a distribution rather than a binary, and a bed where the only way to
die is hunger is exactly the missing middle. It *is* an argument against
expecting a war to reshape a session, and against tuning toward one until the
owner has said he wants that.

## What closing each gap would look like

**None of this is proposed for landing.** The owner asked to explore, the
switch below changes the ecology of every bed in both games, and he has not
seen it yet. What follows is the shape each option takes and what it costs.

### Gap 1 — the stranger. One number, and it is the whole answer

The mechanism is built, guarded and shipped; only the dial is at zero.
`scent_spread` is a `CreatureDef` field, so today it is reachable from a
species file and from `lab::params` and from nowhere a player can find.

**Ship it default-OFF and expose it, rather than choosing a value.** That is
this repo's own standing answer to "does this look right" — *ship a runtime
selector rather than choosing* — and it is the honest shape here for three
reasons. The setting is a **draw, not a value**: two colonies at `spread = 1`
separate on some seeds and not others (measured below), so there is no number
anyone can author that means "rival colonies". It is **irreversible within a
run** in the direction that matters: a founding offset is drawn once, so a bed
that started as one family cannot become two by moving a slider, which makes
it a *new-box* setting rather than a live one. And the owner has not watched
it yet, which is what the card is for.

**Two things it must not become**, both already rejected here. It must not be
an exemption — nothing gets an invulnerable or exempt state; if one colony is
to be harder to kill it pays for armour on the same axis everything else does
(`dead-ends.md` :272, :276, :403, four support models dead that way). And the
dial that separates colonies must not be `TRAIT_TOLERANCE` narrowed to `-1`:
that is a radius of zero, it makes every ant a stranger to its own children at
the shipped `scent_drift: 0.15`, and it is the confound in `labstats`'
`rivalry=1` alias.

### Gap 2 — the eye. Real, useful, and not about fighting

`nearest_foe` is a ring walk and never reads `sight_range`, so an eye cannot
start a fight that adjacency would not have started anyway. What an eye buys
is **approach**: `ThreatNear`, `ThreatBearing`, `PreyNear` and `PreyBearing`
all read a hard 0.0 on every shipped ant, so a colony cannot cross the box
toward a rival, cannot run from one, and cannot evolve either behaviour
because the inputs carry no signal to select on.

It is also the expensive one. `sight_fraction` bills **per cell read** and one
cast at reach 64 is 328–1,186 `World::get`; the measured duty cycle of the
threat sense is one cast in thirteen. Giving the ant an eye is a frame cost
and an energy cost on every animal in the bed, for a behaviour nothing yet
selects on — and `dead-ends.md`'s flight race is the warning: a wired flight
raced at `sight=64` read **47.9% median against the plain turn's 52.4%**, both
inside the harness's own 2.42–3.12x seed noise. **Give the eye when there is
something worth seeing, not before**, which is to say after Gap 1.

### Gap 3 — initiation. It exists; a second one is optional

`ThreatNear -> Attack` is a real wire and it is **inert without an eye**, by
construction: `sighted` returns early at reach 0, so the input it reads is a
constant. Measured as the `W` and `VW` arms below. What it would add on top of
an eye is *pre-emption* — biting something that is coming rather than
something that has arrived — and that is a behaviour worth having, but it is
a refinement of a war, not a way to start one.

**`Caution` is still a working consumer that no species authors**, confirmed:
zero occurrences across all twenty species files. Unrelated to fighting and
worth someone's round.

### The eating half — a ledger split, not a mechanism

See above. Per-colony `harvested_corpse` is cheap and answers "which colony
lives off carrion". "Whose dead" needs the corpse stamp to carry the victim's
group, which is a `creature.rs` change and a different size of job.

## What this lane would do next, in order

1. **Show the owner a fight and ask whether it is worth watching** (the card
   below). Everything after this depends on the answer and nothing before it
   does.
2. **If yes: expose `scent_spread` as a new-box setting, default 0.** One
   dial, on the page a player can reach, with the honest label — *how far
   apart two colonies start*. Default-off because the owner asked to explore
   and has not seen it; he can rule otherwise in a sentence.
3. **Fix §Z23 either way.** It is not conditional on any of this: the played
   bed today destroys 58–95 cells of standing food per 24,000 frames through
   a verb aimed at plants, and the counter everyone reads as "did anything
   fight" is counting that.
4. **Then the eye, if a war turns out to be worth approaching.** Not before —
   it is a per-tick charge on every animal for a behaviour nothing yet
   selects on, and `dead-ends.md`'s flight race is the warning.

**What this lane is not proposing**: no species file changed, no default
moved, no `creature.rs` edit, and no tuning toward a bloodier bed. The
measurement says making colonies strangers adds an event worth seeing and
does not reshape a session, and the next decision is the owner's.

## Cards

**Card `20260914T053420770Z-6a7aa9`** — *Two colonies that are strangers to
each other*, board `creatures`. An A/B of **frame sequences** (175 frames
each, 960x300, the surface band where the two colonies meet), un-blinded and
labelled, because the question is "can you see it" rather than "which is
better" and blinding would remove the very thing the owner needs to answer.
Same bed, same seed, same 4,200 frames, same patch of ground; the only
difference is `scent_spread`. `meta` carries the number a picture cannot give
— **0 killings between colonies against 9** — per the queue's house rule.

**A sequence rather than only a GIF**, on the review skill's own head-to-head
finding that a posted sequence played where a valid GIF did not; the GIF was
written too and is in the scratch directory if it is ever wanted.

**`20260914T053354967Z-98ac84` is a duplicate of it** — the same card posted
twice by mistake, identical in every field. Either may be answered; the other
can be ignored.

