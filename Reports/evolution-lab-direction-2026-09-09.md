# The evolution lab: direction, 2026-09-09

*Written after reading every report from the last 24 hours, the owner's rulings across 51 lab review cards, a wiring audit of what the engine has built and never connected, and a fresh render of the default bed. Answers "what is the best next step, and in what direction," records the owner's rulings made the same day, and carries the first phase of the implementation — five lanes, all landed on `main` with this report. Status: **direction of record for the lab through phase one; the standing rulings here supersede the coordinator note's where they differ, and the coordinator note points here.***

## 0. The direction in one paragraph

**The lab has everything a player needs and nothing a player is shown.** Every one of the last day's reports arrives at the same shape from a different side: the box measures more than it tells (what-is-missing), the engine has a working jump that no animal was ever given (movement-modes), the colony has a trail-following circuit that cannot cold-start because nobody lays the first trail (colony-starvation), and the roster has a graveyard with cause of death that the owner rated 5 and that has no place in the game (owner rulings). So the next phase is not a mechanism. **It is turning the instrument toward the player**: give the individuals in the box names and a story, show evolution as an event when it happens, make the animals visible, and wire in the behaviours the engine already has. After that, make the box alive (the colony's cold start) and then make it a rack.

This is the owner's own direction read back: *"Give me the tools, data, access ... that is the game"*, and the three complaints from *"the most fun I have had in the game yet"* were all legibility: who is who, only one graph, what is a group.

## 1. Three arcs, in order

### Two corrections from the owner, 2026-09-09, that govern Arc A

1. **The harness bed is not the played bed.** Every headless figure in the last day's reports is from a box where 52 ants land on eight seedlings at frame 0. The owner grows the bed first, then adds ants, and populations build up (*"although over long enough time everything usually dies out"*). So §Z6 is a long-horizon problem the player already works around, not the wall in front of every other change — and the harnesses need a played-bed mode (`ants_at=N`: plants grown N frames before the colony lands) before any further census is quoted. **Built the same day, and the first honest picture of the played bed says the owner was right**, `labshot`, same seed, same bed:

| frame | colony founded at frame 0 | colony founded at frame 6,000 |
|---|---|---|
| 6,000 | 5 | 39 (just founded) |
| 12,000 | 4 | 31 |
| 30,000 | 13 | 31 |

Dropped on seedlings the colony collapses at once and never recovers; founded on a grown bed it seats fewer (39, less room) and holds. Same binary, opposite trajectory. Every §Z6 figure quoted before this was taken on the left column.
2. **It has to work at a thousand ants**, which the owner has reached. Per-individual names, per-birth log lines and per-event pauses all drown at that scale. The design principle that follows:
   - **identity lives at the line** — founders and the lines that split off them are named; an individual is only spelled out ("Vernal-9") when pinned or inspected. Lines are bounded by founders plus splits, so this reads at 4 ants and at 4,000;
   - **events are records, not births** — the log and the call-back hook fire only on line-bounded events: a line ending, a line splitting off, a line setting a new record on a trait ("Vernal-31 is the first of its line to see twelve cells"). Records fire roughly logarithmically in births by construction. Per-birth mutation diffs live on the pinned individual's inspector, never in the log;
   - **the overlay is a swarm, not a thousand labels** — a marker per animal in its colony colour, no text on the bed;
   - **per-line population is the story at scale** — at a thousand ants the readable question is which lines are winning.

### Arc A — somebody in the box (this session)

| step | what the player gets | owner standing | cost |
|---|---|---|---|
| **A1 names and a chronicle** | Founders get a line name; children carry it with a generation number; the run log reads as sentences: *"Vernal-9 fed herself for the first time"*, *"The Vernal line ended, 14 generations from the founder"* | asked for per-line identity (*"every line should have its own colour even when there are twenty"*); an open card asks which naming scheme | very low |
| **A2 evolution as an event** | A birth that differs from its parent says how: *"Vernal-9 born: gut +12%, sight −4%"* | inside the log's measured event budget (15 animal births per 90k frames); *"default to your recommended settings, not off"* | very low |
| **A3 the box calls you back** | A notable event slows the dial and puts the camera on the subject, then the dial climbs back; a toggle makes it a full stop | unruled; it answers guide §8.9 *"what does the player watch during a Running phase?"* which the owner never answered. Graded (linger) rather than binary (pause), per the ethos | low |
| **A4 the animals are visible** | A marker overlay draws every animal as a legible blob at play zoom during Running | asked for twice (*"you said you needed the gut-bias overlay to see ants and beetles"*); the trail half already failed once and stays out | low |
| **A5 wire the free gaits** | A second animal that hops (the engine's jump, unwired since August), and species that move at different paces | *"with the new jump is great"*; *"we definitely want new creatures to not look like a recolored ant"*; ship new behaviours as default (round twenty) | one `.ron` each |

### Arc B — the box is alive (next)

`colony-economy-design-2026-09-09.md` (PR #289, design only) measured where the colony's energy goes: locomotion is 52% of burn, intake is 16% of burn, every founder empties its identical 200 J grant inside the same 500-frame window, and the generation clock (8,600 frames) is the foraging economy seen from the other side. Its four routes, in the order this direction takes them:

- **B0 rest, via hunger — already in the wiring lane.** `(Bias, Move, 2.0)` and `(Bias, Dig, 0.4)` mean an ant walks 67% of ticks and digs 29% of ticks for its whole life; there is no idle state. The owner's own play report (*"they tend to just dig out the world when they have nothing else to do"*) has this exact cause. Moving the drives onto `Energy` — a sense that exists and nobody reads — makes rest a state an ant can be in, and is the precondition for scouts-then-trail-then-colony to be *found* rather than authored (trail following and castes already ship). Measured by the cliff and the dig count, not by taste. **Owner's constraint, 2026-09-09: rest is the absence of a reason to act, not the presence of a full stomach** — the ant must not become an animal that only acts when hungry, because a colony has to find food before it runs out, defend itself, and dig chambers as it grows. So the move drive is a *sum of reasons* — own hunger, nestmates' hunger (the trophallaxis input, which pushes a full forager out as well as making it share: demand-driven foraging), the trail when empty, alarm — over a residual wander that never reaches zero; and digging is conditioned on crowding at the nest rather than zeroed. An arm that flattens the cliff by collapsing bed coverage fails.

**Measured, four arms, three seeds, `labforage` and `labstats` on the default bed** (A = unwired; B = `Bias 2.0` kept, `Energy -1.75`, `FoodAdjacent` re-derived to `-1.16`, `Dig` cut to a residual `0.15` behind a new crowding-at-nest hidden gate; B′ = B with `Energy` normalised to the breeding bar instead of the founding grant; B″ = B at `Energy -1.0`):

| seed | arm | alive at 4,500 | columns walked by 49,500 | digs by 9,000 | births by 49,500 | walking share of burn |
|---|---|---|---|---|---|---|
| 1 | A | 6 | 448 | 477 | 19 | 51.5% |
| 1 | B | **46** | 383 | 243 | **131** | 41.2% |
| 1 | B′ | 10 | 388 | 179 | 27 | 51.9% |
| 2 | A | 8 | 435 | 497 | 25 | 52.5% |
| 2 | B | **46** | 279 | 288 | 6 | 40.6% |
| 2 | B′ | 10 | 411 | 222 | 15 | 51.9% |
| 3 | A | 11 | 342 | 476 | 30 | 51.9% |
| 3 | B | **47** | 254 | 243 | 18 | 38.5% |
| 3 | B′ | 13 | 326 | 200 | 7 | 52.0% |

**The cliff is walking cost, and only rest closes it.** B′ and B″ — every attempt to keep the founders walking — bring the cliff straight back to A's level. B ships, with its price recorded: births 155 against 74 in aggregate but a lower per-seed median (18 against 25), coverage 14–50% lower, and one seed (2) at six births in 49,500 frames — a colony that survives and mostly rests. Two things bound that price and neither is measured yet: the demand-driven move wire (`KinNeed`, from the trophallaxis lane) that pushes a full ant out when its sisters are hungry, and the played bed, where a colony founded on grown plants holds 31 under arm A already — so the cliff B closes is partly a frame-0 artifact, and **A against B on the played bed is the number that decides whether the Move wire stays.** Run the same evening (`labforage ants_at=6000 frames=30000`, three seeds): survivors at 30,000 **27 / 43 / 45 with the wire against 27 / 46 / 34 without**, births 23 / 30 / 31 against 13 / 37 / 22, burn a third lower for the same intake, a quarter fewer columns walked. Neutral on survival, ahead on the birth median, cheaper, narrower. **The wire stays.** Also shipped in the same lane: the parameter pages describe the armed animal, the five stale ant forks carry the ant's current instincts, the shelter readout counts real exposure, `labstats` prints the flight counters, and a first hopping species (`hopper.ron`, plus the material file it turned out to need before it could hatch) — 2,551 launches in its first 3,000 frames, and 42 of 52 dead by then, seven of them in the air, because a bias of 2.0 on the jump is a hop on two ticks in three. That rate is the creature line's to set.

**And the queen, ruled on the same day: build it, never as a type the engine knows.** The owner's instinct was that a hardcoded queen breaks the principle of letting creatures arise, and that the principle has not delivered — the box has one creature with no behaviour differences. The resolution: a queen is three authored values over mechanisms that exist or are in flight — a per-species *founding rule* (one founder with a large reserve, versus a cohort), a founder who *rests because she is full* under the hunger wire and is kept full by trophallaxis, and *sterile workers* provisioned through the caste channel that shipped and has never fired (`Provision`, audit row 12). Reproduction concentrated in one individual makes the colony the unit of selection, which is the only condition under which castes are adaptive — the shortest path to visible behaviour differences in one box — and twenty workers feeding one breeder is the "uninterrupted feeding" the generation clock is priced in. The principle check: a line that zeroes the sterility weight must revert to budding workers. What it does not do: make a *world* of creatures arise; that needs niches, and every food cell is currently worth the same 480 J at every height.

The one-page sketch (`scratchpad/queen-sketch.md`) priced it: almost all of a queen is a `.ron` file (`colony_ants: 1` is a knob today; `found_colony_of` takes a count), and the single genuinely new thing it identified — *a mother cannot make a child that is a different animal from herself* — dissolves under the composition above, because the queen differs from a worker in **traits**, which the developmental channel already moves, not in brain weights. She does not change the 8,600-frame clock (she does not feed; workers do); she changes the opening and the cliff. Her death is graded only if her workers outlive her — check by playing. The nest ruling cuts both ways: a sealed-chamber verb deepens the encoded nest the owner objected to; an endowed founder with no reason to walk replaces it, because home becomes wherever she stopped. **Two halves for less**: staggered founder reserves (~10 lines, in the trophallaxis lane now; the owner approved it) and founders released in waves (zero lines — scenario timelines already carry `Colony(species, x, count)` entries).

**Owner's downstream question, same day: will castes actually behave differently, or only breed differently?** The answer that binds the eusociality lane:

- The *capacity* exists: the provisioned caste value is itself a brain input (`BrainInput::Made`, computed and never read), so any `(Made, verb)` weight makes a verb caste-conditional — gene regulation, one genome expressed by cell type — and body differences add to it (eyes see threats; a reserve is never hungry).
- It will **not** stratify on its own, for three reasons that are the lane's requirements: (1) selection pays only for what the colony pays for — soldiers need predators or rival colonies (beetles ship off; `scent_spread` ships at zero so colonies are never strangers), foragers-who-bring-home need bringing home to feed the breeder; (2) trophallaxis as specified shares *down the energy gradient* and equalises — a queen is fed preferentially only if need is defined against her breeding bar, not a fixed reserve; (3) **the clock changes unit**: queen-only breeding makes worker mutations dead ends and an evolutionary generation a colony founding a colony — a handful per session, against the owner's 60–70 target.
- So: **graded fertility, not a sterile bit** (worker breeding suppressed by proximity to a breeder; a queenless colony's workers resume — the ethos' middle, and the clock keeps ticking through workers), pressure shipped on (scent spread, directed need), and **an instrument before a claim** — per-caste verb counts (attacks, deliveries, shares by caste bucket). The lane's first deliverable is the measurement of evolutionary generations per session under individual / queen-only / graded breeding; the box is not committed to queen-only until that number is in front of the owner.
- **B1 trophallaxis — the first real build.** Fifty-two ants are fifty-two economies that never exchange a joule, which is why they die on one schedule. A shared crop turns the cliff into a draining reserve: a binary into a distribution, the ethos' first law, at no change to any price. Visible: ants meeting head to head. **Owner's ruling, 2026-09-09: a brain output the genome can evolve, shipped on.** A rule-based always-on transfer was the alternative and was not taken — it hardcodes the colony where the output exposes it. Spec written the same day (`scratchpad/trophallaxis-spec.md`, to land as a report): energy-direct rather than crop, because a 480 J leaf arriving in one lump is another binary; one constant `SHARE_FRACTION = 0.25` that is at once the cap, the floor and the grading; the price is one jaw closure through the existing bill; `KinNeed` (largest deficit among adjacent living kin) is the sense, computed in the pass the mouth already makes; the default wiring `(Energy, Share, 2.5) (KinNeed, Share, 1.9) (Bias, Share, -2.5)` is derived so that a full ant beside a starving sister shares on two ticks in three and a starving ant never gives away its last joules, and the predicted behaviour is that *whoever eats redistributes* — income pooled, not stock reshuffled. Creature ticks are serial (the scheduler runs them after the CA sweep), so no two-body verb ever meets the checkerboard. Cost: 706 → 763 live genome slots, `mutation_rate` re-derived in three files, every ant's RNG stream shifts. **The spec also found the shared-budget trap in the rest wire**: `(FoodAdjacent, Move, -1.5)` was calibrated against the constant bias, so the composed Move row keeps `Bias 2.0` as the *starving* ant's rate, adds `Energy -1.75` (full ant rests at 0.20, wanders), `KinNeed +1.25` (a full ant beside hungry kin goes out), and re-derives `FoodAdjacent` to `-1.16`; both lanes were given that one table. Being built now, with staggered founder reserves in the same lane. **First control, same evening, a negative worth keeping**: with sharing on and the Move row untouched, shares fire (196–276 per 6,000 frames, zero with the ablation switch off) and the colony holds more animals at frame 3,500 — and then the cliff is *taller* on two of three seeds (3 and 7 alive at 4,500 against 20 and 20), with harvest roughly halved. A colony that pools its stock without foraging harder pools its way to the same floor faster. Two suspects sent back for measurement: a share tick that excludes moving and feeding in the verb arbitration (in a hungry colony some adjacent kin is always needy, so every richer ant spends its ticks giving), and the absence of the demand-driven move wire the spec's own §5b named. The lesson is the spec's own prediction read the other way: *whoever eats redistributes* only helps if redistribution does not cost the eater its foraging time.
- **B2 litter — measure first.** Plants shed to the floor only on death and rot. Census the floor band over a long run before building continuous shedding.
- **B3 rain.** The owner asked for *"a auto rain option"*; the air simulation sits idle. Lights stay on (ruled).
- **B4 the gut: expose the price, keep the ruling.** A plant-specialist gut measures 138x the births and generation 19 against 4, and it is an owner-vetoed dead end (*"an omnivore should be viable"*). The price of that ruling has moved since it was given; put the number on the page and let castes carry specialisation.
- **B6 hands on the colony — owner's idea, 2026-09-09.** *"It might be fun if the user could manually lay down pheromone trails or have other tools to interact with the creatures."* Both halves exist: the scent planes are live and the shipped ant follows them; only an ant already carrying can lay one, which is the cold start. So: `SCENT` (drag to lay food-route or home scent with the ants' own deposit call), `ALARM` (drop alarm scent and watch who answers; the plane gets its missing overlay in the same lane), `FLING` (click an animal and it launches, on the jump the engine already has), `LAMP` (the built-and-uncalled lamp verbs). Keys and a page, since the bar is full. The second law applied to creatures: a verb, and it delivers.
- **B7 the trail gate — found by the tools lane's positive control, and it sits under everything above.** A laid channel-B trail moved a 20-ant colony's near-target count by exactly zero, twice, at two deposit shapes. The arithmetic on `ant.ron`'s hidden layer says why: the laden gate is `-45 + 75·Carrying`, which parks the unit at 30 on the squash curve where the slope is one in a thousand, so the `±6` trail term moves the step probability by about **±0.003**. Round twenty-four's *"the circuit is live end to end"* is true and the circuit is inert — an additive gate large enough to switch a unit off flattens the signal it lets through. Filed as a bug with its bar; the fix (a gate that keeps the unit on the slope, or a multiplicative gate) is a creature-line race, not a coordinator's guess. Until it lands, the cold-start hypothesis is upstream of a reader that cannot read, and `SCENT` lays a gradient the ant cannot act on.

  **Measured the same night, and the answer split.** The gate fix works as
designed and the shipped ant with both halves does worse on every bed
tried; the homing half alone is a clear win in the arena and free on the
played bed, and it ships. The food-trail half stays open as a finding
about the larder: a colony that can read where its sisters found food
converges on patches it has already eaten. Recruitment is a bet on patchy
food, and no bed tried was patchy enough to pay it.
- **B5 no beetles in the default box**: *"the user plants and starts colonies manually"*. The predator is one chip-cycle away.

**And one big swing from that report worth its own line: found the colony the way nature does — one queen, not 52 workers.** The synchronised cliff cannot happen in nature because members are born at different times with different reserves; it happens here because the box drops 52 strangers with identical wallets into bare soil. A queen who seals herself in and raises a first brood from her own reserves makes the opening state *mean* something, and it is a lifecycle and scenario change, not an afternoon.

### Arc C — the box is a rack (after)

- **C1 small chambers at 4x.** Compartments, the rack page and the tab strip all exist; what is unbuilt is camera and UI on a 128-wide box. Attacks legibility, divergence and reach at once.
- **C2 `CROSS` on the shelf.** Already scoped by the prior lane (D4 caged the brain topology *so that* crossover is representable). World stays asexual; player gets the scissors.
- **C3 the outdoor game feeds the lab.** Nobody has asked this. Both games share the `.ron` species format, so a wild specimen collected outdoors is already a file the lab can load. This gives the outdoor game a reason to exist relative to the lab, and the lab a supply of starting material that is not hand-authored.

## 2. Big swings worth discussing

1. **The chronicle as an export.** Once the log is prose, a run can write its own history to a text file on reset: a legends page. Cost: a formatter. Payoff: the thing players tell each other.
2. **Do not sweep the bed between experiments.** The bed is a physical medium; litter, old galleries and buried deadwood accumulate. A failed experiment leaves a layer, not a blank box (what-is-missing bet 3). Zero mechanism.
3. **Body plans, not recolours.** The owner's stated number-one issue is that every creature looks like a chain. The hopper is the first animal with a different verb; the next question is whether it can have a different *shape*.
4. **Wild collection.** See C3.

## 3. What this deliberately does not do

- **Commissions / score / economy** — deferred by the owner three times in one sitting. Not built.
- **Mating in the world** — hard dead end (asexual budding *is* the isolation). Only the shelf verb.
- **Darkness regimes** — *"lights do not turn off for now"*.
- **Sound** — never raised by anyone; noted as the one genuinely unruled item and left for the owner.
- **A magnifier verb** — zoom exists and was rated 5.
- **A museum** — the graveyard is built and rated 5; only a place for it is missing, and that is a presentation change for a later pass.

## 4. Features built and never wired (the audit)

A read-only audit of what the lab can reach, with brain wiring extracted by a parser over all 17 species files (hidden layer included, so the trail-following mistake cannot recur) and a live-signal census over 22 animals so every "unread" claim says whether the channel carries a signal. Twenty-two items; the ones that change what a player sees:

| # | what | evidence | wire |
|---|---|---|---|
| 1 | **Hunger is a sense no animal reads.** `BrainInput::Energy` swings 0.01–1.00 across the colony; zero species author a weight on it. An ant at 1% energy and one at 100% behave identically, and the specimen page prints *CANNOT FEEL HUNGER* | census mean 0.68, sd 0.40 | one `.ron` line, weight picked by paired measurement |
| 2 | **Lamp placing/moving/removing is a fully built verb with zero callers** — the lab's only light control is total brightness | `lamp_near` has no caller even in tests | a tool off the bar (the bar is measured full) |
| 3 | **The parameter pages describe the ant whatever animal is armed** | `COLONY_SPECIES = "ant"` hardwired at three sites while `spec.colony_species` is in hand | three one-liners |
| 4 | **Five of the eight stockable animals are stale forks** missing ten instincts, a hidden unit, and carrying superseded weights | tuple diff | mechanical |
| 5 | `LightHere` — the strongest spatial gradient in a sealed bed, unread | census 0.05–0.99 | one line |
| 6 | The SHELTER readout is a null wearing a measurement: its counter sits inside a gate on a price nobody sets | `exposed on 0 of 33,848 ticks` | move one line |
| 7 | The alarm plane (512 deposits per 4,000 frames) has no overlay | live, invisible | one enum variant |
| 8 | Every animal pays a synapse each tick for a temperature input that is identically zero in the lab (no heat source) | census 0.0000 | leave (lava exists outdoors); note |
| 9 | The jump, plus its four counters, which only `filmstrip` reads — a wired hop would be invisible to the lab's own instruments | known | one line + labstats |
| 10–22 | prey-near on the beetle (held for an owner verdict), threat senses (need an eye first), the caste channel (`Provision`, never fired), scent spread at zero so two colonies are never strangers, moisture senses unread, `Transpire` off by recorded decision, `worm.ron` half a species, three scenario placements unused by any scenario | — | see the audit |

Rows 1, 3, 4, 6, 9 and the hopper species are in the wiring lane now. Rows 2 and 7 are queued behind phase 1 (row 2 collides with the call-back lane's files).

## 5. Phase 1 plan and ownership

Four Sonnet lanes in separate worktrees, file-disjoint by the architecture map's split; the coordinator merges, gates and lands on `claude/lucid-hamilton-dgfpnc`.

| lane | delivers | owns |
|---|---|---|
| wiring | hunger wire (measured), `hopper.ron` with the jump, flight counters in `labstats`, species-threaded params pages, the five forks repaired, the shelter counter freed, `worm` resolved | `assets/species/*`, `params.rs` (registry), `labstats.rs`, the include list, one line of `creature.rs` |
| A identity | the event spine (`LogEvent` carries lineage and generation; `LINE 0 ENDED` fixed), line names from a pure hash of (seed, lineage), `GroupSplit` / `LineMilestone` / `LineRecord` events bounded per lineage, `born_with` on every individual, the log as sentences ≤ 42 chars with a `LINES / ALL / PINNED` filter defaulting to the chronicle | `world.rs` log region, `creature.rs` birth/bud, `plant.rs` seed/germinate, `names.rs`, `plainspeak.rs`, `params::story`, `ui.rs` log rows |
| B call-back | `OFF / LINGER / STOP` reaction on line events, 4 s cooldown, camera and pin to the subject, key `T`, a BOX page row | `time.rs`, `mod.rs` advance, `ui.rs` BOX rows |
| C marks | a full-replace marker per living animal in its colony colour, key `Y`, `labshot marks=`, before/after PNGs with the animal count. **The first form — a solid 3x3 square over the animal — was rejected by the owner on card `20260909T193830388Z-0a8f39` the same evening** (*"I don't like the new"*): it made the animals findable by hiding the animals. Rebuilt as `Off / Halo / Tick` (a ring around the body in world cells, or a dot above the head, neither touching a body pixel), **default Off** until the owner judges a zoomed three-way card. The lesson, recorded: the animals should be visible *as themselves* — this is the owner's stated number-one issue (*"I don't want all creatures to look like a chain"*) wearing a marker's costume, and appearance gets its own lane after phase 1. **And a correction to the report this direction started from**: the owner, same evening — *"Movement is honestly the biggest thing that makes creatures visible. So if you are just viewing static frames that explain why they are so hard to see."* `what-is-missing` §2 measured 0.016% on a contact sheet; a two-cell animal that moves is findable in play and invisible in a still. The gap is real at pause and on stills, and smaller than measured. **Any visibility claim in this project is judged on a moving sequence (`filmstrip gif=1` or a frame sequence on the card), never a sheet** — the review skill already says so and the report did not follow it | `ui.rs` Watch region and draw block, `labshot.rs` |

Documentation (README status sections, `wiki/`, `Reports/README.md`, this report) is the coordinator's, written once at the end so no lane touches a contested doc file.
