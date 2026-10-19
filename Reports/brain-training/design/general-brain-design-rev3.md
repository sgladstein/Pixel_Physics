# A general brain for the ant (and what comes after it)

Oct 17, 2026 · @Scott Gladstein

## Short answer

**Recommendation: one general brain is the goal, built in two stages, with a cheap test first of the one claim it rests on.** The end state is the design in this doc. The world lists what the ant could do right now: step one of 8 ways, dig that cell, drop the pellet here, feed that nestmate. The brain picks the job from how the ant feels and what it remembers, then picks that job's target by scoring each one the same way. The body (ant, bee, butterfly) declares what it can sense and do, and the brain wires to whatever the body offers. A new species is a new body file, not a new brain.

**Why two stages.** Tonight's re-runs and a second, independent review changed the picture.

- **Today's brain trains well.** It follows a trail on unseen layouts (0.65-0.77 alone, against 0-0.15 with its senses cut). Six copies trained together learn to share one lane (0.82). Its trained foragers are not better than the game ant yet: one learned to forage on one side only and its colonies starve on the other, and the other brings home more food but loses about a third of its founding ants.
- **Its real gap is choosing targets.** Hand code picks which cell is dug, where spoil and food are put down, and who is fed. Its senses are not the problem I first said: it already reads scent to each side and scent strength, and it has 30 free input slots.
- **The full brain is expensive.** Today's brain is about 4% of a measured lab frame (2.28 ms at about 540 ants). The full design would be roughly 20 times that, close to doubling the frame before the option lists are built. Training it up the whole ladder is weeks on this machine, not days.
- **Its first checkpoint could not tell the two apart.** The trail arena involves no target choice, and today's brain is already near that arena's ceiling.

**So: stage 1** keeps today's brain as the part that decides *whether* to dig or drop. It replaces the hand rules that pick *where* with a small evolved scorer, the same shared scorer this design uses. It is tested in a new arena where target choice is the whole game: where to put spoil, and which cell to dig. Three arms run in it: the hand rules, stage 1, and the blind and legality-only controls. That is a few hundred weights, days of compute and little frame cost. **Stage 2**, the full brain, is built only if stage 1 shows that evolved target choice beats the hand rules, and only inside a frame-cost gate.

The pieces of the end state, in one line each:

1. **Body contract.** Each species declares named senses and verbs. A new sense starts with zero weight. A new verb stays off the list entirely until a gene switches it on, so adding either never changes a trained brain.
2. **Job first, then target.** A small part of the brain picks the verb from hunger, load, memory and the like. One shared scorer then rates that verb's targets from what each would meet, and it knows which verb it is scoring for. It is the same scorer for all 8 directions, so a rule learned facing left works facing right.
3. **The engine's own choice rule, made to say "avoid".** Picks are weighted, best most often, using the squared-score rule the ant uses today. Scores are shifted so "bad" and "never" stay different, and the leftover randomness is set as a stated chance. No exponentials in the ant, so its choices stay identical on every machine.
4. **Context and memory.** Hunger, load, at-nest, alarm and how each is changing, plus 12 memory units that each evolve how fast they forget. The body supplies a home vector and a last-food vector.
5. **Jobs from drives, not switches.** A pull towards the job it was already doing stops flickering. A personal number fixed at birth lets clones split the work.
6. **Engine keeps physics, brain owns policy.** For this ant, the rules that decide which cell to dig or where to drop go away. Rules about what is physically possible stay.

**Decisions for you:**

1. **Build in two stages, stage 1 first?** Recommended. Stage 1 is the target scorer on today's brain, in a target-choice arena, in days. The alternative is building the full brain now: weeks of training, about twice the lab frame, and its core claim untested until late. Either way, you see pictures before stage 2 starts.
2. **Let this ant drop the hand-written policy rules and keep only physics?** That means spoil keep-inside, the dig vetoes, the door column and the brood placement rules. Stage 1 drops only the ones it replaces. Recommended, because otherwise training cannot reach half of what the ant does. Two scope notes: where eggs are laid stays body biology until workers can carry eggs, and the dig score has to change so that a colony cannot score "room" by piling a roof instead of digging (see *How we will know*).
3. **For this ant, make hunger a strong sense instead of an override?** Your rule for the hand-built ant is "hunger overrides every rule". Here training would have to learn to eat first. If it doesn't, a starving-ant floor comes back as a labelled stopgap. Recommended, because an override is exactly the kind of hand rule the brain cannot learn around.

**Already ruled, not asked again:** colony jobs select whole colonies of clones, and the rig may mutate between runs (your 10-17 rulings).

## What we learned tonight

**Today's brain is small and wired by hand, and it does not decide most of what the ant does.** It has 34 senses, 8 memory units and 17 outputs. About 11 of the 34 senses always read zero or a constant for the ant: sight is off, and Energy reads full for any fed ant. In the game, the brain's outputs are probabilities, and hand code picks the target. Which cell is dug, where a pellet lands, which food is taken and who gets fed are all chosen by rules in the engine. Its senses are better than I first wrote: it reads scent to the ahead-left and ahead-right, scent strength ahead, and bearings to kin, threats and home. 30 of its 64 input slots are free. Its one sense of change over time reads only while laden, so trained brains that put weight on it are leaning on an input that is mostly zero.

**Today's brain can be trained, and it can follow a trail.** I first read the trail results as a hard limit. That was an inference, and the review caught it. The re-runs (rig fixed, trail shape now varied per layout, 12 unseen layouts, which are 6 trail shapes each run mirrored) show:

| On unseen trails | Alone | Six copies in one lane |
| --- | --- | --- |
| Trained old brains (3 from blank, 1 from the game brain) | 0.65-0.77 | 0.12-0.18 |
| The same brains, senses cut | 0.00-0.15 | 0.00-0.06 |
| Hand-built gradient follower | 0.58 | 0.35 |
| Walks one way, ignores the trail | 0.30 | 0.28 |
| Old brain trained as six copies (1 seed, 40 generations) | | 0.82 |
| The same, senses cut | | 0.00 |

So the trained brains read the trail and choose the right side. Six copies jam in a one-lane arena: they walk only 28-60 cells in the whole run. Training had mixed six different brains in an arena, while the honest test used six copies, and that hid the jam. **Once trained as six copies, the old brain learns not to jam**: 0.82 on unseen trails as a group, against 0.12-0.18 before, and 0.00 with its senses cut. So jamming was a training mistake, not a limit of the brain.

**Foraging reacts to the world, but the trained foragers do not beat the game ant.** Averaged, trained colonies scored 14.6 and 15.0 against the game ant's 8.4, and 0.0 and 0.3 fully blind. Layout by layout, with deaths of the founding ants counted (12 unseen layouts, heap east on even ones and west on odd ones):

| Colony | Food score, mean | Spread (sd) | Founding ants kept |
| --- | --- | --- | --- |
| Trained, seed 1 | 14.0 | 15.2 | all 12 on east layouts; **none** on west layouts |
| Trained, seed 2 | 12.2 | 7.5 | about two thirds |
| Game ant, own walk | 8.3 | 3.1 | about 93% |

Seed 1 learned to forage east only, and on west layouts its whole colony died: a one-way habit that averaging over both sides rewarded. Seed 2 forages both ways and brings home more food than the game ant, but it loses about a third of its founders, which the old score hid because hatching brood refilled the head count. The game ant also spends food raising brood, which the score does not count. So the score is fixed first (build step 0), and the comparison is redone after.

**Every score we wrote was gamed within a few generations.** Ants scattered the food heap, flew food home with a verb meant for other species, and let a colony shrink from 12 to 8. A new brain will find new holes. The checks that caught these (unseen layouts, a senses-cut copy, a hand-built brain that should pass, pictures) apply again, but they have to be rewritten for the new brain's layout (see *Build plan*).

| What decides it today | Brain | Hand code |
| --- | --- | --- |
| Whether to step | yes | pace and depth rules |
| Which way to step | nudge of 45 degrees, plus tumble | turn-around, homeward pick, falls, crossings |
| Whether to dig | yes | lean, company, roof and spoil vetoes |
| Which cell to dig | no | straight ahead plus widen, down and face turns |
| Whether and where to drop spoil | whether only | keep-inside, ring, lift, site |
| Who gets fed, and how much | whether only | neediest nestmate, fixed share |
| Larvae fed | no | the larva's own tick takes food from a touching adult |

The code map behind this is in code-map.md. The second review (`design/review-rev2.md`) checked every number above against the logs. The re-run logs are in `brain-training/rig-2026-10-17b/`.

## Design goals

The brain should let new behaviour evolve that nobody wrote, in any creature, and stay cheap and watchable. In priority order:

1. **Fun to watch.** Its choices should read on screen: you can see why an ant turned, and a colony's strategy is something a viewer can name. This comes first, per the project's north star.
2. **General.** One brain design for every creature. An ant, a bee and a butterfly differ in the senses and verbs their body declares, not in brain code.
3. **Can evolve new behaviour.** Training can reach every decision the creature makes. It can add memory and connections over time without throwing away what it learned.
4. **One brain, many jobs.** Foraging, digging, nursing and guarding come from one brain switching on what it senses, never from separate specialists.
5. **Cheap.** It fits the frame budget at lab colony sizes (about 540 ants) and stays deterministic. New per-cell work is limited to one scent plane, the building scent, whose cost is measured with `pherocost`.
6. **An answer in days, the whole ladder in weeks.** The first go/no-go (stage 1) should answer within days on one machine. The full ladder takes weeks here, less with cloud runners (see *Learning*).

Non-goals: biological accuracy for its own sake, and learning during an ant's life (left for later, see *Learning*).

## The body/brain contract

**Each species file declares its senses and verbs as named, typed channels, and the brain only ever talks to channels.** Karl Sims' creatures, Framsticks and the Bibites all work this way. It is what lets one brain design serve an ant now and a bee later.

| Channel type | What it is | Ant examples | How the brain reads it |
| --- | --- | --- | --- |
| context | one number about the creature itself | hunger, crop fill, load type, at-nest, alarm here, age, clocks | straight into the scorer and memory |
| option feature | one number per candidate action, describing what that action would meet | scent at the cell stepped into, footing, diggable, roofed, home direction | into the shared scorer, once per option |
| verb | an action the body can take, with its own list of legal targets, and optional settings such as an amount or a direction | step, turn around, dig, pick up, drop, eat, feed, attack, rest | the job head picks the verb; each legal target becomes one option |
| emit | something the body can do alongside its verb, at a strength | lay scent A, B or alarm | one output each, read every decision |
| body memory | a quantity the body keeps, like a compass | home vector, last-food vector | read as context and as option features |

Three rules make it safe to grow:

- **Stable names, not positions.** The genome stores weights against channel names. Today's brain grows safely by appending into reserved slots, but it is capped at 64 per kind, cannot reorder, reads only straight ahead, and every tool around it (the rig's bans, senses cut and crossover, the species files) assumes that fixed layout. Names are what let a bee's body and an ant's body share one brain design.
- **Missing reads absent; new enters switched off.** Each group of senses has a "present" flag, so a body without that sense reads "absent", not a misleading zero. A new sense starts with every weight 0. A new verb is not offered at all until a gene switches it on: its options are not built, not counted in the pick, and not in any summary the memory reads. A strongly negative score would not be enough, because the pick gives every score at or below zero the same small chance, and any extra candidate shifts which option a given random draw lands on.
- **The body can switch channels on and off by heritable trait.** A blind lineage's sight channel reads absent until it evolves eyes. This is how a body can evolve alongside the brain later.
- **The body decides what is legal.** Whether a step fits is the body's question (a long ant, a wide ant and a flier answer differently), so the contract names the body's own legality check, and the brain never sees an illegal option.
- **Life stages are bodies.** A caterpillar and a butterfly are two bodies sharing one genome. Each stage's channels are named by stage, so one genome can hold both without the caterpillar's weights steering the butterfly.

## How it decides

**Each decision, the engine lists what the ant could legally do. The brain picks a job from how the ant feels, then picks that job's target by scoring each one the same way.** Choosing among scored options is how the best-supported ant navigation models pick a way to go (Baddeley 2012). Creatures picked a verb first and then an object to apply it to.

One decision, step by step:

1. **List the options.** The engine builds every legal (verb, target) pair:
   - **steps**: each of the 8 directions the body can enter, plus **turn around** (for a long body, reversing is a flip, not a step into its own neck);
   - **digs**: each neighbouring cell the jaws can cut;
   - **drops**: each free neighbouring cell;
   - **pick-ups**: each item in reach;
   - **feeds**: each nestmate or larva in reach;
   - **rest**, with no target.

   A typical ant has 10-25 options.
2. **Describe each option.** Every option gets the same feature list (see *Senses*): what is in that cell, scent there and beyond, footing, roof, which way home is from there, and who is there.
3. **Score every target with one shared scorer.** The ant's context and memory are folded in once per decision. Then the same small layer of 16 units runs on each option's features, plus which verb the option belongs to, and gives one number. Without the verb, a step and a drop aimed at the same free cell would score the same. A feature that means nothing for a verb (footing, for a feed) is read as absent, not as zero.
4. **Pick the job.** Each verb gets a score from three things: context and memory through its own weights; a pull towards the verb chosen last time; and the best of its targets' scores, minus a small learned offset per verb. A verb with no target (rest, turn around, eat from the crop) uses a learned constant instead. "Hungry, so eat" is then one weight, not a pattern the scorer has to discover. Counting only the best target keeps a verb with 6 targets from being picked 6 times as often; some bias towards verbs with more targets remains, and the offset is there to absorb it.
5. **Pick the target** among that verb's options, by score.
6. **Act, emit, remember.** The ant does the chosen option, lays each scent at a strength from its own output, and its memory units update from context, the best and average option scores, and what it just chose.

**How a pick is made.** Steps 4 and 5 use the engine's own rule, `choose_weighted`: each candidate's chance is its score plus a floor, squared. As it stands, every score at or below zero gets the same floor, so the brain could not say "avoid this" more strongly than "not this". Since this brain has no hand rules left to forbid things, that matters. So scores are shifted up before the pick (from -1..1 to 0..G), and "bad" and "never" stay ordered. The floor is then set as a chance, not a constant: at full confidence a job the brain does not favour is drawn about 0.1-0.3% of decisions. Without that, a laden ant 100 decisions from home would drop its load on the way about two times in three. The genome can sharpen or soften the scores within set bounds, but cannot remove the floor. Two records say why:

- `brain.rs` forbids exponentials and other transcendentals in anything a decision reads, because they are the cross-platform determinism trap.
- Replacing the probabilistic pick with "always take the best" is a recorded dead end (`dead-ends.md`, P-10): it kills exploration, so ants keep moving but stop finding anything. A floor that is too big is also recorded: an ant stepped into thin air 16% of the time when small scores sat near the floor (`dead-ends.md`, step scoring "Added, not multiplied").

Every value inside the brain is squashed with `x / (1 + |x|)`, as today.

**What can be computed early.** Today the engine can compute an ant's senses and brain one step ahead, in parallel, and keep the result if nothing nearby changed. About two thirds of that early work is thrown away at 450 ants; a filter that would cut the waste exists but is off, because it measured worse. So the cost figures below assume no early compute at all. If it is used, the split is: the option features times their weights (the bulk of the work) can be computed early; the context part, the squash, the pick, the act, the memory update and the emits stay in order. Reads that change whenever anyone acts (food in the store, contact rates, room per ant) are in the context part for that reason, and the one random value comes from a hash of the seed, the frame and the ant, not from the world's shared random stream.

**Actions that take time.** Digging a cell or feeding takes several ticks. Once started, the ant finishes it unless alarm or a fall interrupts. The brain decides again when it is done.

Why this shape:

- **It hands targets to the brain.** Which cell to dig, where to drop and whom to feed become choices training can reach, instead of hand rules.
- **One rule works in every direction.** A rule learned facing left already works facing right. With 8 separate direction outputs, evolution would have to find it 8 times.
- **Drives choose jobs directly**, which is what *Jobs and switching* relies on.
- **It works for any number of options.** A flier with 16 directions, or an ant facing 3 food items, reuses the same weights.
- **It can be drawn.** The target scores can be shown as a rose around the ant, and the job as a label, so a viewer sees why it turned.

## Senses

**Two kinds of sense: what each option would meet, and what the ant knows about itself.** Today's brain mixes the two in one flat list read only along the current heading, so a trail to the side is invisible until the ant turns towards it. Every value is scaled to about -1..1. A sense that reads the same for every ant in every arena is left out until a body can vary it, since it gives mutation nothing to work with. The tables below are the core; the full list, checked against real ants and the engine, is in Verbs and senses, checked.

**Per-option features (first ant, about 14).** Each is read at the option's target cell. Most are already computed by the engine and simply never reach the brain.

| Feature | What it says | Engine source today |
| --- | --- | --- |
| material class | air, soil, sand, food, spoil, nest, stone, water, creature | the cell |
| scent A, B, alarm | trail strength there, minus here (a difference, as real antennae compare) | trail planes |
| scent ahead | the same, 2-3 cells further that way | trail planes |
| footing | can stand there; would fall; climbing | step and foothold checks |
| diggable, hardness | the jaws can cut it, and how firm it is | jaw strength and soil resistance |
| roofed | ground somewhere above that cell | the nest census rule |
| depth | rows below the surface there | surface row |
| home direction | how much this option points home (cosine times distance, squashed into -1..1) | the body's home vector |
| last-food direction | the same for the last place food was found | new body memory |
| crowding | ants in or next to that cell | creature cells |
| same colony / stranger | who is in that cell, if anyone | colony id |
| back the way it came | this option undoes the last step | last heading |

**Context (about 30 with the full list).** Wetness and temperature come from coarse 16-cell blocks, so per option they are read smoothly between blocks; otherwise neighbouring options read identical. The core: bias; hunger (energy below full, not clamped at full as today); crop fill; load type (none, food, soil, brood, corpse, each its own input, so a larva and a body can go to different places); at nest; inside, meaning roofed over; alarm here; stillness; age; a personal number fixed at birth (see *Jobs*); a fast clock and a slow clock the brain can reset; one random value; and the colony's need for food, read from the nest store, which ants do sense by sharing food mouth to mouth.

**Others.** A nestmate, a stranger or another species is just another target in an option: feed that one, step towards that one, attack that one. Its features include colony, what it carries, whether it is hungry and its species. Seeing others is therefore part of the same scorer, with no separate wiring.

## Verbs and senses, checked

**The list was checked three ways:** against what real ants do and sense (ethogram.md, 55 sources), against every verb and sense the engine already has (engine-inventory.md), and against what a bee, butterfly, worm or beetle would add. It was also checked against the world review, the digging study and the engine review. Each entry is marked **must** (the first ant), **should** (next, once the first ant trains), or **later** (other species, or once something else exists).

**What the check changed.** The verb list mostly held. Its two gaps were that pick-up and drop need an object type and a destination, and that rest must be a real choice. Most of the gaps were in the senses: the ant must be able to tell *who* it touches (nestmate, stranger, laden, hungry, larva, dead), read scent by strength and not only by difference, and feel the cues real ants dig by (moisture, up and down, head-room, a building scent, room per ant).

### Verbs

| Verb | Target | Tier | Engine today | Why |
| --- | --- | --- | --- | --- |
| walk | one of 8 headings | must | exists; brain only nudges | flee, patrol, search and follow are all walk under different senses |
| turn around | none | must | hand code (flip or back out when boxed in) | a long body cannot step into its own neck, so reversing is its own verb |
| rest | none | must | only "did not move" | real workers idle a lot, and idlers at a crowded dig face raise the digging rate by preventing clogs; it must be chosen and must pay |
| dig | a cell the jaws can cut | must | exists; hand code picks the cell | the soil pellet goes into the jaws |
| pick up | food, soil, brood, corpse | must | food yes; brood carrying exists as a hand rule, off by default; corpses no | each object gets its own place: spoil heap, brood pile, store, cemetery. Those places are what you see |
| drop | here, or **deposit onto a structure** | must | exists; hand code picks the site | deposit is how building happens, and it marks the cell with building scent |
| eat | food in reach or in the crop | must | exists |  |
| feed | adult nestmate or larva | must | hand code picks the recipient; larvae fed by touch on the brood side | separate targets, because a hungry larva and a hungry adult ask in different ways |
| attack | stranger or prey | must | exists; hand code picks the target | banned in tonight's arenas; trained in a rival-colony arena added before the lab step |
| pick up a nestmate | trapped, injured or moving nestmate | should | no | dramatic and nearly free: it is pick-up with a new object |
| freeze | none | should | no | feign death; found in ants and beetles, cheap and easy to read |
| touch | a nestmate in reach | should | contact happens only by bumping | starts a contact on purpose, which is how ants read each other's task and hunger |
| cut | food too big to carry | should | no | splits a carcass into loads, like dig on food |
| lay egg | brood cell | later for the brain | body biology, gated by food | stays body biology in the first build (see *Risks*) |
| fly, land | heading | later | exists for the flitter | bee, butterfly, beetle, winged ants |
| oviposit | host plant | later | no | butterfly |
| burrow through | solid soil | later | the worm, by hand code | worm: moves through soil without carrying it |
| collect nectar or pollen | flower | later | nectar yes, pollen no | bee; pollen would make pollination real |

**Emits**, alongside any verb:

| Emit | Tier | Engine today | Why |
| --- | --- | --- | --- |
| scent A, scent B, alarm | must | exist | no built-in meaning; evolution assigns them |
| building scent | must | **missing** | real ants mark placed soil, and the mark draws more soil; its lifetime shapes the whole nest (Khuong 2016). It is the cue the digging study found missing: put spoil where spoil already stands |
| no-entry mark, ask for food, vibration | should | missing | real ants use all three; vibration works underground where scent is slow |
| dance (direction and distance) | later | missing | bee |

### Senses

| Sense | Read as | Tier | Engine today |
| --- | --- | --- | --- |
| Scent A, B, alarm | per option, **strength and difference**, plus 2-3 cells ahead | must | computed; only the difference along one heading reaches the brain. Strength matters because alarm reverses: weak draws ants in, strong makes them attack or flee |
| Building scent | per option | must | missing (comes with the emit) |
| What is in the cell | per option: air, soil, spoil, food, brood, corpse, stone, water, ant | must | known, never sensed. Spoil must read differently from soil, the brain-side half of the world review's "spoil its own colour" |
| Footing, fall, climb | per option | must | known, never sensed |
| Can cut, and hardness | per option | must | known, never sensed |
| Soil wetness | per option, by **level** | must | per-cell wetness exists, but the ant sees only a blocky 16-cell field. Real ants dig best in moist soil, less in wet, and stop short of standing water |
| Up and down, slope, depth | per option and context | must | known, never sensed. Real ants use gravity to carry spoil up and out |
| Head-room and roof | per option: open cells above in body heights; roofed or not | must | known (roof checks), never sensed. The roof cue real ants use, and the world review's top item, "am I inside" |
| Who is there | per option: nestmate or stranger; laden; hungry; larva and its hunger; dead | must | all known; only the neediest nestmate's hunger is sensed. This is how harvester ants decide how many foragers go out |
| Contact rates | context, a slow average of meetings by kind | must | hand code meets returning foragers; nothing sensed |
| Crowding and room per ant | per option and context | must | crowding sensed; room per ant known, unsensed. Room per ant is the real stop rule for digging |
| Home vector | per option and context | must | partly kept, sensed only as a cosine when laden |
| Last-food vector | per option and context | must | **missing** |
| Light | context: inside or outside | must | sensed, but the day is divided out |
| Temperature | context and per option | must | sensed weakly; real ants move brood along warmth |
| Self | hunger, crop fill, load type, age, wound | must | hunger clamped at full; load type merges spoil and food; age and wound unsensed |
| Colony | food in the store | must | known, unsensed |
| Change over time | for scent here, alarm, hunger, crowding, home distance | must | missing (see *Memory*) |
| Clocks, personal number, one random value | context | must | missing |
| Depth or nest air, to find the nest core | per option | should | depth known; no air field |
| Vibration, with direction | per option | should | **no vibration anywhere in the engine**; structural collapse data could feed it |
| Time of day | context | should | known, deliberately divided out today |
| Being followed | context | should | missing; needed for tandem running |
| Wind, rain, weather | context | later | known, never sensed by any creature |
| A familiar view | per option | later | route memory (see *Learning*) |
| Colour, ultraviolet, optic flow, taste on the feet | per option | later | bee and butterfly |

### Checked against the three project reviews

- **World review**: the inside sense, per-ant home and last-food memory, and trails with range are all must above. Door clearing goes, as a policy rule. More scent planes is the building scent plus the should emits. Warmth and wetness with depth are must senses, but the world also has to make them vary. Roofs that hold and scent stopping at rock are world fixes, not brain ones.
- **Why digging fails**: "carry spoil away and set it where it stands" is drop-deposit as a chosen target, plus the building scent. "Head underground toward cover" is the roof, head-room, light and depth senses on each option.
- **Engine review**: trail choice, job and nursing were decided by code. They become walk options scored on scent, the personal number, and feed with a larva as target.

### What other species add to the contract

Four things the ant does not need: **fly** (bee, butterfly, beetle), **life stages with different bodies** (caterpillar and butterfly; the ant's brood is a simpler case), **oviposit** on a chosen plant, and **moving through solid ground** (worm). Each is a verb or a body declaration, so none changes the brain. Two more the contract must allow: **verb settings** (an amount, a distance, a flight height, a dance's direction), and **bodies of different shapes**, whose legality check the body supplies.

**Cost of the full list.** About 40 option features and 30 context senses, about 2,800 weights in all (see *Size, cost and growth*). The engine work is mostly exposing data it already computes. Three things are new: the building scent, the last-food memory, and contact rates.

## Memory

**Two kinds: memory the body keeps for the brain, and memory the brain evolves for itself.** Insects have dedicated circuits for the compass and the home vector, and evolving those from generic units is slow and unreliable. A small evolved robot brain did learn to go home that way, but failed from some starting points (Floreano and Mondada 1996). So the fixed-function parts belong to the body, and the brain learns when to trust them.

**Body memory (not evolved, read as senses):**

- **Home vector.** Updated on every step and reset at the nest. Real ants do this by path integration. Ours can be exact, or a little noisy if exactness makes it too easy.
- **Last-food vector.** Where food was last picked up, relative to here. Real ants remember a feeder; nothing in the engine stores this today.
- **Last heading** and **steps since the nest**, both already kept.

**Brain memory (evolved): 8 to 16 units, each with its own forgetting rate.** Each unit blends its old value with a new, squashed one at a rate its gene sets. A rate near 1 makes a latch or a slow odometer, and a low rate makes a fast filter. One gene per unit buys both, and it replaces today's fragile pairs of large opposite weights that one mutation can break. Each unit reads context, a summary of the options seen, and what the ant just chose, so it can remember "I am carrying soil out" without being told.

**Senses over time: how things are changing, not just what they are.** Three layers, cheapest first:

- **Change since the last decision, given as a sense.** For a few context senses (scent here, alarm here, hunger, crowding, distance from home) the body also supplies how much each moved since the ant last decided. "The trail is getting stronger as I walk" and "I am getting hungrier" become inputs the brain can use from generation one. Real ants follow trails partly this way, by comparing over time as well as left against right. About 5 extra inputs.
- **Trends over longer spans, learned by the memory units.** A unit that forgets slowly and one that forgets fast, reading the same sense, together tell rising from falling over many steps. The brain can evolve this for any sense, without us choosing which.
- **Change across space, per option.** The option features already give scent there minus here, so the brain sees which way a trail gets stronger before it steps.

Route learning during life, the one thing real ants learn, is left for later (see *Learning*).

## Jobs and switching

**Jobs come from drives and commitment inside one brain, and the colony splits work because each ant is slightly different.** There is no job variable and no specialist brain.

- **Drives are senses, read by the job head.** Hunger, load, colony food need, crowding and alarm are context inputs, and each verb has its own weight on each, so "hungry, so eat" is one weight. That works in Creatures, Polyworld and the Bibites. On its own it makes ants dither between jobs, so two more pieces are needed.
- **Commitment.** The job head pulls towards the verb chosen last time, at a strength the genome sets within bounds. That keeps a job, not just a heading. A slow memory unit can hold a longer mode, such as "carrying spoil out", which takes many different verbs in turn. Real ants stick with a task for a while, and an ant that flickers between two jobs is unreadable on screen.
- **Individual difference.** Clones in one colony need a reason to differ, or they all do the same job at once. Each ant gets a personal number fixed at birth, plus its age and its own history. The brain can use these as thresholds, which is the standard model of how real colonies divide labour (Bonabeau 1996): each ant has its own threshold for each task stimulus. Training then evolves the spread of thresholds, not just one ant.
- **Modules later.** If one scorer cannot hold enough different jobs, a mutation can copy the scorer, and each copy gets a "preference" output. Whichever copy prefers most decides. This beat a single network on multi-task games (Schrum and Miikkulainen 2014), and evolution decides what each copy does. It is a growth path, not part of the first build.

The test that this works is a colony doing several jobs at once with ants that stay in each one for a while, measured as time spent in one job before switching (see *How we will know*).

## Signals and other creatures

**Scent is a verb with no built-in meaning, and every other creature is just another target the scorer can rank.** That keeps communication and contact between species inside the same brain, with no special wiring for either.

- **Generic scent planes.** The ant can lay 2-3 scents (A, B, alarm) at a strength it chooses, while it does whatever else it does. The brain is not told that B means food. Evolution decides which plane becomes the trail, which the alarm, and which the recruitment call, as with the three pheromone colours in the Bibites. Robots evolved food and poison signals from a plain light this way (Floreano 2007).
- **Kin recognition is the gate.** Honest signals and food sharing evolve among relatives under colony selection, and deception evolves under selection of single ants (Floreano 2007, Mitri 2009). So colony odour is the most important social sense, and colony jobs are selected as whole colonies of clones. Without it, feeding a stranger is a free meal for the stranger. In arenas that mix different brains, "same colony" does not mean related, so signalling is trained only in clone arenas.
- **Others as options.** A nestmate, a rival or a beetle appears as the target of an option: feed it, step towards it, attack it, avoid it. Its features say colony, species, what it carries, whether it is hungry, and whether it is alarmed. A new species needs no new wiring; it needs a species feature the scorer has not seen before, which starts at zero.
- **Between species, expect cheating.** When interests conflict, evolved signallers hide information and bluff (Mitri 2009). For a game that is a feature: a rival colony that learns to fake your trail is a story a player can watch.

A bee's waggle dance would be a later emit that carries a direction, read by nestmates as an option feature. It fits the same contract, but nothing in the first build needs it.

## Size, cost and growth

**About 2,800 weights for the full "must" list of senses and verbs, with room to grow.** That is about 3.5 times the roughly 780 weights today's brain can train, and far more than the 54 the shipped ant actually uses. Stage 1 is a few hundred.

| Part | Size | Weights |
| --- | --- | --- |
| Option scorer: about 40 option features into 16 units | 40 x 16 | 640 |
| Which verb the option belongs to, into the same 16 units | 9 x 16 | 144 |
| Context and memory (about 30 + 12) into the same 16 units | 42 x 16 | 672 |
| Scorer unit biases and output | 16 + 16 | 32 |
| Job head: one row per verb (9) reading context, memory and its best target, plus an offset | 9 x 45 | 405 |
| Memory: 12 units reading context, two option summaries, the chosen verb and each other | 12 x 53, plus 12 forgetting rates | 648 |
| Emits: 4 scents (A, B, alarm, building) and 2 clock resets | 6 x 43 | 258 |
| Sharpness bounds and the pull towards the last job | 2 | 2 |
| **Total** |  | **about 2,800** |

The option features are counted from the must list in *Verbs and senses, checked*: material as 9 kinds, 3 scents each read 3 ways, building scent, footing (3), cutting (2), wetness, up/down/depth (3), head-room and roof (2), who is there (7), crowding and room (2), home, last food, temperature and back-the-way.

**Cost, against a measured frame.** At about 540 ants the whole lab frame is 2.28 ms on one thread, and today's brain is 4.2% of it, about 0.1 ms (`needscost`, 2026-10-06). The new brain is about 20,000 multiply-adds per decision against today's roughly 1,000, so about 20 times today's brain: around 2 ms, close to doubling the frame. Building the option lists is extra and probably costs more than the network: about 25 options, each with about 40 features, roof and head-room scans and room per ant. That is an estimate until step 5 measures it.

**The gate.** Before any brain behaves, the new ant at about 540 ants must fit within 1.3x the game ant's worst frame. If it doesn't, the levers in order are: rank the verbs on context alone and score targets only for the top three; build each option's features lazily; shrink the scorer to 8 units. Stage 1 is far cheaper, because it scores candidates only when the ant has already decided to dig or drop.

**How it grows, without starting over:**

- **New senses enter at zero.** A new option feature costs one column of 16 weights, not one per direction, because the scorer is shared. Each feature group also has a "present" flag, so a body that lacks a sense reads "absent" rather than a misleading zero.
- **New verbs enter switched off.** A new option at score zero would steal picks from the trained ones at once. So a new verb is not offered until its switch gene is on; mutation can flip it. A unit test checks that adding a switched-off verb leaves a trained brain's choices bit-identical, draw for draw.
- **New hidden and memory units enter at zero outgoing weight**, so adding one changes nothing until mutation finds a use for it.
- **A small cost per non-zero connection** in the score keeps the brain sparse and makes modules form (Clune 2013). Modular brains also forget old tasks less (Ellefsen 2015). It also stops what today's runs show: champions piling weight on inputs that are always zero for this ant (`Made`, `ThreatBearing`), because nothing selects against it.
- **Growth by mutation stays off at first.** NEAT-style growing networks were rejected for this project before (`creature-direction.md` D4): unreadable, slow to start, and needing extra machinery. Your 10-17 ruling frees this line from the old rules, but those failures are real. So the brain stays a fixed size until it is shown to be full, and then grows only by the zero-entry steps above, never by rewiring.

## Learning

**Evolution only, in the rig that already works, with a much bigger search than tonight's, climbing a ladder of arenas where every old arena stays in every generation.** Learning during an ant's life comes later, in one place.

**The search budget.** Tonight's runs used populations of 24-48 for 30-40 generations with a big mutation step. That was enough for today's brain on a single arena, but the review is right that it is far too small for about 2,800 weights. The project's own survey of what worked elsewhere (`prior-art.md` §8) gives the starting point:

- **population 100-200**;
- **8 layouts a generation**, made of 4 shapes each run mirrored, so a one-way habit cannot win on lucky layouts;
- **keep the top 10-20%**, one or two unchanged;
- **a small mutation step**, about 0.1 of the weight scale, on 10-20 weights per child;
- **crossover off** at first.

Before choosing the layout count, the rig measures how much one fixed brain's score spreads across layouts (the noise bar).

**What that costs.** Measured unit: a forage generation at population 24 with 4 layouts takes about 18 s on this machine (4 threads). Dig runs 20,000 frames to forage's 6,000. Scaled to population 100 and 8 layouts, 100 generations, one seed, at today's per-ant cost:

| Rung | Time per seed |
| --- | --- |
| Trail | about 1 h |
| Forage | about 4 h |
| Dig | about 14 h |
| Suite at the dig rung (trail, forage and dig every generation) | about 19 h |
| The lab box, 100,000 frames at about 540 ants | about 7-13 h **per generation**, so it is a final check, never a training rung |

Multiply by the seeds and starts (3 seeds and 2 starts each), by the old-brain arm trained the same way, and by the new brain's own cost per ant. **The whole ladder is weeks on this machine, not days.** Stage 1 trains only the target-choice arena, which is about dig's size. The cloud runners you approved (told to you first) are what make the later rungs practical.

The rig's own mutation uses logarithms and cosines, so a training run in the cloud may not replay a local one bit for bit. The ant's choices stay identical; switching the rig to the uniform step `brain::mutate` already uses fixes the rest, and is part of step 0.

**Where training starts.** From blank brains, as you asked, and also from a hand-built brain in the new format that already does each arena's job roughly. The second start tests whether refining works. The first tests whether evolution can find the job from nothing. `brain.rs` records the old rule that "generation zero must already behave". Your 10-17 ruling lifts it for this line, and running both starts shows what it costs.

**Who is selected.** It depends on the arena, as you ruled. Solo skills select single ants; colony jobs select whole colonies of clones. Tonight showed this matters for scoring as well: a brain trained among strangers was judged among copies of itself, and they jammed. So an arena's training and its honest test now use the same setup.

**The ladder, one brain all the way up:**

1. **Trail.** Follow a scent line to its end, on whichever side it lies, alone and as a group of clones that must not jam.
2. **Forage.** Find food, bring it home and stay alive. Starts from the trail champion.
3. **Dig.** Make a roofed room and put the spoil outside it. Colony selection.
4. **Brood.** Feed larvae from the nest store and keep them together.
5. **The lab, as a check, not a rung.** The full sealed box: a colony that lives, grows and builds a nest, run on the champion at about 540 ants. Too slow to train in; it is where you judge the result. A rival-colony arena for attack comes before it.

**No forgetting.** Each new step starts from the last champion, and every generation runs every earlier arena as well (Gomez 1997). The score multiplies each arena's score against its starting level. Any old arena that falls below 80% of where it was cuts the total to a tenth. This is the rig's suite mode, already built; its starting levels are now set on their own layouts, never the honest-test ones.

**Scents need senders and receivers together.** A trail only helps if layers and followers evolve at the same time, which took the robot studies hundreds of generations. Today's brain has a hand rule that lays food scent only when carrying food. So at the forage step, the scent outputs start from a hand-built layer, which mutation is then free to change. Otherwise the forage bar has to start lower, and the doc would say so.

**Keep variety, not just the leader.** Every score we wrote was gamed fast, and a population that all copies one leader gets stuck on one trick. Later, the rig keeps the best brain for each kind of behaviour, sorted on two or three axes such as time spent digging and distance roamed (a method called MAP-Elites). That guards against one-trick winners, and a gallery of different-looking colonies is a game feature in its own right.

**Learning during life, later and in one place.** Real ants learn their routes, and that is the one thing they learn. Once foraging works, one module can store what a route looks like while the ant is laden, and recognise it on the way back, with its learning rate evolved (Ardin 2016, Najarro 2020). Everything else stays fixed within a life, which keeps behaviour readable.

## Physics vs policy

**One test sorts every hand rule: could a real ant have chosen otherwise?** If not (gravity, jaw strength, a body that will not fit), it is physics and stays. If so (where to dig, where to drop, who to feed), it is policy, and for this ant it becomes an option the brain ranks. All of this applies only to the new ant behind its switch. The game ant stays frozen with every rule it has.

| Stays: the world's rules | Goes: choices the brain now makes |
| --- | --- |
| Falling when unsupported; what a body fits into and stands on | The 45-degree turn limit; turn-around only by tumble |
| Which cells the jaws can cut, and how long it takes | Which cell is dug: straight ahead, widen, down and face turns |
| A pellet lands only on a free, legal cell | The six dig vetoes: spoil cue, trip, roof, mound roof, shaft room, company |
| Energy costs of walking, digging, carrying, laying scent | Where spoil goes: keep-inside, ring, lift up the column, site pick |
| Crop size, digestion, starvation, old age | Where food is dropped, and the store, harvest and meal holds |
| Birth needs food in the store (food caps births) | Needs-first: the hungry drop, the pack, the walled-in cut |
| Scent spreads and fades by the field's own rules | Lean ants may not dig; lean ants drop their load |
| Two ants cannot share a cell | Lay food scent only when carrying food |
|  | Homeward pick when laden; forage pace; the door column; the pull ladder |
|  | Who gets fed: neediest nestmate |
| Brood develop, hatch and starve by the brood's own clock; where the queen lays stays body biology until workers carry eggs | Brood carrying and spreading, nurse seeking (switches in `brood.rs`) |
|  | Larvae feeding themselves from a touching adult: becomes the adult's choice to feed that larva |

**Two judgement calls, and one decision.**

- **Swapping places with a nestmate** in a tunnel stays, offered as an option (step into a cell a nestmate holds) rather than done by hand.
- **Rest has to pay.** Resting costs less energy than moving, and a crowded dig face clogs if nobody waits. In arenas that top up energy, resting gains nothing, so those arenas stop topping up once rest is a verb.
- **The share amount** stays fixed, like a mouthful. The brain chooses whether and whom, not how much, until a reason to vary it shows up.
- **Hunger overrides every rule** is your principle for the hand-built ant, so changing it for this ant is decision 3 in *Short answer*, not a judgement call.

What this buys: training can reach every row on the right. Today it reaches almost none of them (see *What we learned tonight*). It also removes, for this ant, the stack of stopgaps added one picture at a time.

## How we will know it works

**Three questions, in order: does evolved target choice beat the hand rules, does the score mean what it says, and is it worth watching.** You judge the last one from pictures, not from me.

**1. The bars, arena by arena.** Each bar is set from a measured run, with headroom, never from a number picked in advance. Today's brain, trained the same way on the same seeds, is the arm to beat, and the game ant runs beside both.

| Arena | What the score must pay for | Today, measured | Must |
| --- | --- | --- | --- |
| **Spoil and dig targets (new, the first go/no-go)** | room dug below the old ground line and joined to the entrance, spoil left outside it, colony alive | not built yet | stage 1 beats the hand rules on room and spoil, with a picture of each hold-out |
| Trail (regression check only) | progress along the trail on whichever side it lies, alone and as 6 copies | old brain alone 0.65-0.77; 6 copies 0.12-0.18 trained mixed, 0.82 trained as copies (1 seed) | stay near the old brain; no target choice is involved, so it cannot decide anything |
| Forage | food stored in the nest, including food fed to larvae; meals of dead nestmates do not count; every death counted | old brain 12.2-14.0 but one-sided or losing a third of its founders; game ant 8.3, keeps 93% | beat the old brain on the corrected score |
| The lab (check, not trained in) | a colony that lives the whole run and builds a nest | game ant | a nest you would pick over the game ant's, at about 540 ants |

The forage and dig scores change in the rig before any new brain is trained (build step 0), and every table above is re-measured after that. The dig score only counts room below the ground line the colony started on and connected to the entrance, because once dropping onto a structure is a free choice, a colony could otherwise score "room" by piling a roof over a dip. The trail arena scores one side only and has no turn.

**2. The score cannot be faked.** Every score we wrote tonight was gamed, so each arena carries all of these:

- **Unseen layouts** the brain never trained on, which really differ in shape.
- **Blinded**: the champion with every sense cut, blind outside the brain too (home bias and chooser off).
- **Legality only**: verb preferences kept, scorer and memory zeroed. The list of what is legal is itself a sense (it only offers "pick up" when food is in reach), so this is the floor a champion must clear, not zero.
- **A hand-built brain that should pass**, run before any evolution, so a stalled run can be told from a broken arena.
- **A second number it cannot trade away:** ants alive in forage, spoil outside in dig, both sides in trail.
- **Training and the honest test use the same setup**, including copies in colony arenas.
- **Every log keeps the run's own settings line**, so which walk and which genome produced a number is never inferred later.
- **Pictures** every 5 generations, and **learning curves** for every run, as PNGs.

**3. It reads on screen.** Built with the brain (step 3), not after, because flicker only shows up in them:

- **Stay time**: how long an ant keeps one job before switching. Short means dithering.
- **Choice margin**: the gap between the best and second-best target. Small gaps with a random pick look aimless.
- **The score rose**: an overlay drawing each target's score around the ant, with its job as a label, so you can see why it turned.
- **A strategy you can name.** Polyworld's watchers named theirs ("edge runners", "dervishes"). If a colony's play is something you can name, it passes. That one is yours to judge, on review cards.

**Cost.** Worst and mean frame with a lab colony of about 540 ants, against the game ant, using `needscost` and `antcost`, inside the 1.3x gate.

## Alternatives considered

| Option | What it is | Verdict |
| --- | --- | --- |
| **Today's brain plus an evolved target scorer** | Today's brain keeps deciding whether to dig, drop or feed. Each hand rule that picks where is replaced, one at a time, by the weighted pick over candidate cells, scored by a small evolved scorer from each cell's features | **Chosen as stage 1.** A few hundred weights, the rig's tools keep working, tonight's champions are its start, and it tests the core claim in days. It is not the old "tune the hand rules" idea: the scorer reads features of each cell, not settings of our rules |
| **Keep today's brain, train it harder** | Use the rig as it is, with a bigger search | It trains well, but hand code still picks every target. It is the arm both stages must beat |
| **Today's brain plus target choice by hand-tuned rules** | Evolvable settings on the hand rules that pick targets | Trains settings on rules we wrote, so the ant cannot find a dig pattern or drop place we did not think of. That is roughly the game ant |
| **The full general brain now** | This doc's end state, built first | Weeks of training, about twice the lab frame, and its core claim untested until late. Stage 2, if stage 1 earns it |
| **Grow today's brain (NEAT)** | Let units and links evolve on the same inputs | Rejected for this project before (D4: unreadable, slow to start) |
| **A big network** | Thousands to millions of weights, trained by evolution strategies (Salimans 2017, Such 2017) | Needs populations near 1,000 and far more compute, and is unreadable |
| **Reinforcement learning** | Gradient training of the policy | Needs far more samples and a training stack the game does not have. Colony credit is hard to assign, and selection is what you asked for |
| **Attention over options** | A transformer-style reader of a variable set (Tang and Ha 2021) | The shared scorer is its simplest form. Add attention only if options need to see each other |
| **An insect-circuit model** | The real central complex and mushroom body (Stone 2017, Ardin 2016) | Built for navigation only. We take its two best ideas, the body's home vector and later route memory |

## Risks and open questions

**Risks, each with what we do about it:**

- **More freedom means more ways to cheat.** Dropping the policy rules hands the brain every loophole those rules were closing. The controls in *How we will know* are the answer, and each new hole found becomes a second number for that arena.
- **It may look worse than the game ant for a while.** A brain without the hand rules starts knowing nothing. The game ant stays the default until the new one beats it on the lab bars, so nothing you play gets worse.
- **The full brain may cost too much at 540 ants.** The network alone is estimated at about 2 ms, close to today's whole frame, plus building the options. Stage 1 avoids most of this. Stage 2 measures option building as a shadow first and must pass the 1.3x gate, with levers ready.
- **Computing ahead may not pay.** Two thirds of today's early work is already thrown away at 450 ants, and the options read a wider area. The cost figures assume none; anything early compute saves is a bonus, measured with `antcost`.
- **Training is slow.** Forage is about 4 hours per seed at the bigger search and dig about 14, and the suite at the dig rung about 19. The whole ladder is weeks on this machine. Stage 1 keeps the first answer to days; the cloud runners carry the rest.
- **Crowds.** Six trained copies jammed a one-lane trail tonight, until the arena trained them as copies; then they learned to share it. In tunnels crowding is the normal case, so every colony arena trains copies, and the lab step checks the brain at full density.
- **The world may undo good choices.** The digging study found dug spoil crumbles back into the hole when piled steeply. A brain that puts spoil in the right place still loses if the world rolls it back. Some world fixes from the world review may be needed for any ant, and the rig will show which.
- **Choice can freeze or blur.** The pick keeps a floor stated as a chance and its sharpness has bounds, so it cannot collapse to "always the best" (a recorded dead end) or drift into stray jobs. Stay time and choice margin, built with the brain, show either failure.
- **Larva feeding moves.** Today a larva feeds itself from a touching adult on the brood's own tick. A "feed this larva" choice moves that into the adult's turn, which changes tick order and the energy ledger. It is scoped as its own step at the brood rung.

**Open questions, for training to answer rather than for us to guess:**

- Is an exact home vector too easy, so that ants never need trails? If so, add noise to it, as real path integration has.
- Does one scorer hold five jobs, or does it need the module copy earlier than planned?
- How many memory units does it use? Start at 12 and watch the connection cost prune them.
- Does the queen use the same brain? Today birth is body biology gated by food, and the first build leaves it so.

**How it sits beside the other ants.** Stage 2 is a fourth ant, beside the game ant (frozen), the new walk (frozen as a comparison) and the textbook ant being rebuilt in layers. The textbook lane is also working on digging, so findings about the world are shared both ways. Whichever ant wins on the lab bars becomes the game ant.

## Build plan

Nothing from step 1 on starts until you answer the decisions in *Short answer*. Each step is checked before the next, and the game ant is untouched throughout.

**Step 0, in the rig, now (no new brain).**
- Forage counts deaths of the founding ants (done), food fed to larvae, and stops counting meals of dead nestmates.
- Mirrored layouts are scored by their worse side, so a one-way habit cannot win on the average.
- Dig counts only room below the old ground line and joined to the entrance, and pays for spoil outside it.
- Each arena gets a noise bar, every log keeps its settings line, and the rig's mutation uses the uniform step (all three done; the bar is `noise=1`).
- Then re-measure every table in this doc, and train today's brain at the bigger search on forage and dig. That is the arm to beat.

**Stage 1: does evolved target choice beat the hand rules?**
1. **The target-choice arena.** Dig and drop spoil, scored as in step 0, with a hand-built scorer that should pass.
2. **The target scorer on today's brain.** When today's brain decides to dig or drop, the candidate cells are scored by a small evolved scorer and picked with the shifted, floored rule. The hand pickers for dig cell and spoil site are off for this arm, behind a switch. Per-ant memory for it sits in a side table on the world, keyed by ant, behind the same switch.
3. **The readouts:** stay time, choice margin and the score rose.
4. **Train it:** 3 seeds each, from today's champion and from blank, beside the hand rules, with all four controls. **I come back to you here** with pictures, curves and the frame cost, and you decide whether stage 2 goes ahead.

**Stage 2: the full brain, only if stage 1 wins.**

5. **The option list as a shadow on the game ant.** It is built every tick and used by nothing, so its cost can be measured at about 540 ants with `needscost` before any new ant behaves. It is its own small PR, and it has to pass the 1.3x gate.
6. **The body contract and the brain:** named channels with present flags, verbs switched on by gene, the job head, the shared scorer with verb identity, memory units, emits and the bounded pick. A new act path does the chosen option with physics rules only. The per-ant state sits in a side table on the world (the saved state is `OrganismState` in `organism.rs`). Includes the unit test that a switched-off verb leaves every choice bit-identical.
7. **Up the ladder:** trail as a check, forage, dig, then brood. Each rung starts from the last champion and keeps the old arenas, and the new brain must beat today's brain trained the same way.
8. **The rival arena and the lab check** at about 540 ants. Then the cloud runners, told to you first.

Step 0 goes into a rig PR of its own, after the current one lands. Stage 1 is one PR with its results. Stage 2's steps each get their own PR with pictures.

## What stage 2 has taught us so far (Oct 17, evening)

The plan's direction stands. These are the places where building it found the plan's text ahead of the code, or a rule that needed a number.

- **Steps 5 and 6 are built** (PR 713). The option shadow alone costs 1.25-1.31x the frame at about 405 ants, which is at the 1.3x gate before any brain runs. Measure it again at about 540 ants before a mind goes into the lab (step 8).
- **Running cost has to be billed the way the game bills it, in every arena.** A mind now pays its brain tax, its jaw and its plate on every tick, falls included. The trail arena raises the energy budget so that nobody starves, and every tax is a fraction of that budget, so for a while it billed big brains a million times over. That quietly capped the mind at about 700 connections. It also made the first forage trial forget trail following once its brain grew past that. With the bill fixed, the same trial kept trail following.
- **Each rung's score, controls and step-up bar** are in the ladder notes, corrected after an independent review: `ladder/trail-rung-controls.md`, `scarce-forage-arena.md` and `dig-from-scratch.md`. The rung bar is unchanged: beat today's brain trained the same way. The trail rung also has to reach the hand-built follower (0.86 as 6 clones, with 80% going the right way), not beat it.
- **Food caps births has to be on before the forage rung trains.** Laying stays body biology, gated by food (Verbs table), but every food gate is off on main. An adult at the nest with a bank over about 1,100 J lays whatever the food. The gate to use is FOOD_BRAKE. As shipped it does not bite on a 12-ant colony: it is off below 30 adults, and its burn leaves out eggs and larvae. So it gets a per-world setting and a brood-aware burn, and the three reference arms are re-run under it.
- **The forage arena is scarce food**: a drip 80 columns out, 12 ants, births on. The score is adults alive times founders kept, where only a founder's own old age counts as kept. It trains at 12,000 frames and is checked at 24,000.
- **Jaw work costs energy in the mind as in the game.** Feeding is billed now; digging and biting will be billed when they act.
- **No forgetting, in numbers.** An old arena under 80% of its start cuts the total to a tenth, as written. The arena a rung is learning is not capped, so a start near zero can still climb.
- **Training curves plot every generation** (owner, 16:56): the hold-out runs each generation from generation 0.
- **The trail rung passed** (owner accepted, 18:47). From blank, the mind reaches 0.94 on all three seeds; today's brain reaches 0.84, 0.69 and 0.78. The mind runs past the trail's end to the wall, which the score does not count against it. Food at the end is what stops it on the forage rung.
- **The cost per connection was not built, and the brain bloated without it.** Every mutation that lands on an empty slot makes a connection, so a mind grows by 25-30 connections a generation whatever it scores. The trail champions reached 1,095-1,160 connections, against about 300 for today's brain. The trail arena never takes its taxes off the body: an f32 at 1e9 cannot lose a bill of about 1. So nothing pushed back, and the forage rung started with a brain costing about 5x an idle ant's upkeep. The rig now has the plan's cost (`wire=`) and a rarer new connection (`grow=`). The trail rung reruns with them before forage trains.
- **The hand-built forager has to be traced, not assumed.** As first written, it random-walked near the nest and never ate. Fixed, it scores 2.75 on 8 hold-outs, against 16.3 for the game ant and 0 for a blank mind: a weak but real pass arm.

## Sources

The full survey, 39 sources with notes on each, is in `/mnt/project-files/brain-training/design/prior-art.md`, and the code map in `code-map.md` beside it. The project's own records this doc follows: `brain.rs` (no exponentials; generation zero must behave), `Reports/dead-ends.md` P-10 (no argmax pick) and `creature-direction.md` D4 (no NEAT), and the two adversarial reviews (`design/review-rev23.md` of the first draft, `design/review-rev2.md` of the second). The outside sources it leans on:

- Baddeley et al. 2012, [route navigation by scene familiarity](https://pmc.ncbi.nlm.nih.gov/articles/PMC3252273): choosing a way by scoring each heading.
- Stone et al. 2017, [path integration in the bee brain](https://doi.org/10.1016/j.cub.2017.08.052): the home vector as a body circuit.
- Ardin et al. 2016, [mushroom body route memory](https://pmc.ncbi.nlm.nih.gov/articles/PMC4750948): the one place ants learn.
- Bonabeau et al. 1996, [fixed threshold model of division of labour](https://pmc.ncbi.nlm.nih.gov/articles/PMC1688885).
- Floreano and Mondada 1996, [evolved homing in a robot](https://infoscience.epfl.ch/record/63879).
- Waibel et al. 2009, [team composition and level of selection](https://infoscience.epfl.ch/record/111745); Floreano et al. 2007, [evolved communication](https://infoscience.epfl.ch/record/99661); Mitri et al. 2009, [information suppression](https://pmc.ncbi.nlm.nih.gov/articles/PMC2747196); Ferrante et al. 2015, [evolved task specialisation](https://pmc.ncbi.nlm.nih.gov/articles/PMC4527708).
- Gomez and Miikkulainen 1997, [incremental evolution](https://cs.tau.ac.il/~ruppin/ALseminar/helman.html); Stanley et al. 2005, [NERO](https://www.cse.unr.edu/~bdbryant/papers/stanley-2005-tec.pdf).
- Clune et al. 2013, [origins of modularity](https://arxiv.org/pdf/1207.2743.pdf); Ellefsen et al. 2015, [modularity against forgetting](https://pmc.ncbi.nlm.nih.gov/articles/PMC4383335); Schrum and Miikkulainen 2014, [modular networks in Ms. Pac-Man](https://arxiv.org/pdf/1604.07806).
- Stanley and Miikkulainen 2002, [NEAT](https://nn.cs.utexas.edu/pub-view.php?PubID=114); Mouret and Clune 2015, [MAP-Elites](https://arxiv.org/abs/1504.04909); Such et al. 2017, [deep neuroevolution](https://arxiv.org/abs/1712.06567v3).
- Sims 1994, [evolving virtual creatures](https://course.ccs.neu.edu/cs5100f12/resources/reading/sims_creatures.pdf); Yaeger 1994, [Polyworld](https://shinyverse.org/larryy/Yaeger.ALife3.pdf); [The AI of Creatures](https://www.alanzucconi.com/2020/07/27/the-ai-of-creatures/); [The Bibites](https://the-bibites.fandom.com/wiki/Input_and_Output_Neurons).
- Najarro and Risi 2020, [evolved Hebbian plasticity](https://arxiv.org/abs/2007.02686v5); Tang and Ha 2021, [permutation-invariant sensing](https://arxiv.org/abs/2109.02869v2).

