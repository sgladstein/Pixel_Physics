# Independent critique: "Teaching the colony" (2026-10-25)

*Reviewer stance: a sim-game designer reading cold, with a working knowledge of
ant biology and ML. Read in full: the plan, the engine brief, CLAUDE.md's ethos
and optimisation sections. Spot-checked: queen.md (§0, B2-B7, §6, U1-U7, §9-10),
keeper.md (§2 games), biology.md (claims the plan cites), the Creatures report,
`mind.rs`, `options.rs`, `creature.rs` (`link_price`), `assets/minds/std_pool.txt`,
`how-the-ant-works.md` §12, `play-as-ant-slice0` §2, `dead-ends.md`.*

**Verdict in one paragraph.** The research synthesis is good: the five
channels and three clocks are the right way to answer "more complex than
follow me". The plan has three problems. First, its keystone does not do what
it promises on the shipped ants: a sense that starts at weight 0 does
nothing until training puts weight on it, and the shipped pool has no weight
on the one memory sense that already exists. Second, it never names the
moment-to-moment loop, and the first keeper verb a player tries is designed
to fail. Third, it argues for the keeper on engine convenience, then makes
the keeper's teaching depend on the most expensive and least certain piece
of the plan. Meanwhile it drops most of the queen researcher's best and
cheapest ideas.

---

## 1. Will this be fun? Two walkthroughs

### The first 10 minutes (as the plan recommends them)

The plan's opening: a nuptial flight lands a queen in the keeper's garden, and
the player "may take her body for the founding".

- **Minutes 0 to 3, good.** You walk, dig a chamber, and lay. Digging is
  physical, with debris and a visible hole. This is the strongest stretch in
  the plan, because the verbs deliver at once.
- **Minutes 3 to 8, the claustral wait.** Eggs become larvae become workers.
  A real founding queen seals herself in and waits. The game's only drama
  here is "four beetles kill a lone queen every time". That is a binary
  fail state, which breaks the owner's first law. Slice 0 itself says to keep
  beetles out at the start, and then there is no drama at all. Either the
  player is a spectator of a timer or dies without recourse. The queen
  researcher's fix is foraging trips out of the chamber, with risk and a
  graded wound (wounds heal since 2026-10-03). The plan doesn't carry it over.
- **About minute 8, the handoff.** The first workers hatch, and the plan
  never says what happens to the player's body. If the player now becomes
  the keeper, they lose the one body they have learned, at the moment their
  colony first becomes interesting. That is the worst possible time to
  change genre. "One fiction" is a claim, not a design: a god-hand gardener
  and an ant body are two fictions stapled together.
- **Minutes 8 to 10, the keeper's first teaching verb fails.** The plan makes
  this a feature: "early on ants sniff your trail and turn away (today's
  measurement, now a stage)". Its own §6 lists "the first verb a player
  tries doing nothing" as a fun-killer. The other keeper verbs (water the
  soil, place a lamp, plant) pay off over many minutes. The first keeper
  minutes are landscaping, which is the exact failure the plan names in §3.

**What the opening needs:** one verb that gives an instant, legible,
per-ant response.

- *Sugar bead on an ant:* it eats and you see a "+" on it.
- *Carry a starving ant to food:* it eats, walks home, and comes back.
- *The queen's bite or feed in the act:* a knockback, or a gold sheen.

Only "carry an ant to food" is marked **now**, and section 3 shows that is
wrong on the shipped pool.

### The 5th hour

Assume steps 1 to 3 are built (memory, odour tags, tandem runs). Step 4 is
probably built; step 5 is not.

- **Lessons decay as fast as ants turn over.** Ants live about 40,000 frames
  and a generation is about 12,000. Every per-ant lesson dies with its ant
  unless it cascades (tandem, shared meals) or becomes instinct. If the
  Baldwin effect does not arrive inside the play session (section 4 argues
  it won't), the 5th hour is *renewing lessons*: the Tamagotchi rot the
  plan itself warns about, with a teaching skin.
- **The spoilt colony is babysitting.** The plan lists "fed on a timetable,
  stop playing and they starve" as a delightful surprise. It is *Creatures
  2*'s failure exactly, and the Creatures report names it the
  single most-repeated complaint. A moral the player can only learn by
  watching their colony starve is not a surprise; it is churn.
- **The goals don't fit the main game.** Garden commissions ("keep beetles
  off the vegetables", "move the colony out from under the patio"),
  seasons, nuptial flights and several colonies in a garden all describe an
  outdoor game. The owner's main game (2026-10-02) is a sealed box under
  grow lights. In the box there are no vegetables, no patio and no incoming
  flights. The keeper fiction that fits is the *formicarium keeper*: you
  buy a mated queen in a test tube, which is the real hobby's founding, and
  you shape the box.
- **One winning recipe looks likely.** Feed generously at the door, so trust
  is high. Then lay keeper trails to everything. The result is Pikmin with a
  lag. The 60 to 80% compliance band is a target with no mechanism behind
  it: nothing in steps 1 to 3 caps compliance.
- **Where the hour is good:** the field notebook, the assimilation ladder
  (if it moves), dialects, and haunted ground. These are real long-tail
  hooks.

### The moment-to-moment loop

The plan never states one. The best one implied is a shaping loop, as in
dog training or falconry:

1. Place a reward (flavoured food, a treat, a trail).
2. Watch one ant respond.
3. Read its page line.
4. Stretch the task (a training ground at greater distance).

That can work. But as written it is **act, wait 30 to 60 s, then open a page
to read a sentence**. A loop whose payoff is text on a page, behind a click,
among 300 near-identical 3-pixel ants, is weak. Players need **ambient
legibility** instead:

- tint ants by what they have learned;
- an overlay of taught ants' paths;
- pin and follow one ant;
- a pupil's "aha" turn shown on the ant itself.

None of these is in the build order. Time control (fast-forward through
brood cycles) is assumed by the "minutes" clock but never planned.

**Is it strong enough?** It is not yet. The *queen's* body clock (carry
brood and see the colony reorganise; feed or bite a worker in the act; lead
a tandem pupil) is a stronger second-to-second loop than anything the keeper
has. The plan scores the queen at 2 to 3 "later" only because it left those
verbs out.

---

## 2. Does it answer the owner's question?

**Partly.** §1 (five channels, three clocks) and §4c to §4d answer "how is
it more complex than follow-me" well. Tandem chains, nursery imprinting,
meals that carry a smell, and the dream replay are the right kind of
answer.

**But then the recommendation dodges it.** The owner asked *how the queen
teaches*. The plan's answer is "nobody teaches by leading, not even a real
queen", followed by "so be the keeper". The case for the keeper is "it sits
on what the engine already does well". That is a build-convenience
argument, not a fun or fit argument. It is also backwards on cost:

- **The keeper only becomes a teacher through the riskiest pieces.** It
  needs in-life learning with trained weights (step 1), smells with
  identities (step 2), and a trust model. Without them the plan admits the
  keeper is a landscaper.
- **Many of the queen's levers need no learned weights.** Brood as gravity
  (queen.md C1: "carry exists; brood scent exists"), presence damping
  (B6), provision (hook exists), and the dream over existing life records
  (U3) act on composition, placement and selection. *Selection on the ~2,800
  existing weights works today*, with no weight-zero problem. The queen's
  genetic teaching is the one channel that is guaranteed to have an effect.

**What the plan dropped from the queen research.** These are its strongest
in-life and outside-the-box answers to the owner:

- **B2 and B3, royal feeding and the bite.** A reward or punishment applied
  *during* the act, so credit assignment is clean. It is Dungeon Keeper's
  slap, with knockback: a verb that delivers.
- **B6 and U6, presence damping and royal absence.** The nest wakes up when
  she leaves, and workers start laying their own genomes. Her body's
  position becomes a graded strategic lever with genetic consequences.
- **C1, brood as the nest's centre of gravity.** Instant and cheap.
- **U1, a queen-scent language with no authored meaning.** The colony evolves
  to answer the player's consistent use. Its own author called it "the
  strongest outside-the-box answer". It is also the honest version of
  weight-zero senses: slow, genetic, and meant to be.
- **U4, your old colony becomes the rival.** Roguelite structure with
  inheritance.

**Steelman for the queen as the main seat.**

1. It is what the owner asked about, in both halves: the queen and the
   genetics.
2. It has a body (`sim::steer` is built), with a stake. If she dies, it's
   over, so every trip out is a decision.
3. Its genetics are natural to the fiction, not bolted on through
   beekeeping tools.
4. Its second-to-second verbs exist or are cheap (carry, lay, feed, bite,
   tandem, plug a door). Its clock-to-clock link is designed: the fast clock
   produces the inputs to the slow one.
5. The "after founding she only lays" problem is answered by B6 and U6
   (leaving is a lever), by C1 (where she puts the brood moves everyone), and
   by the dream (the egg-laying stretch becomes the selection interface).
6. Its version of the world-shaping verbs comes from inside: she carries
   seeds (U5, the dowry) and her daughters plant them.
7. It shares the plan's whole engine, so keeper tools can be added later as
   a "god hand" mode.

**The strongest case against the queen seat:**

- the slow clock may be too slow to see;
- the dream risks becoming a menu;
- crossover and stored fathers are new;
- a queen who leaves and gets killed ends the game in a binary way.

These are real. They are no worse than the keeper's dependence on learned
weights that don't exist yet.

**My read:** the plan should present this as a genuine fork with the costs
stated honestly. The fork is *the keeper teaches by conditioning
individuals; the queen teaches by shaping who exists and where they grow
up*. It should test the fork with two faked 10-minute slices (section 6,
change 2), not settle it by argument.

---

## 3. Engine and buildability: weight zero does not survive "see it in a minute"

**The core problem.** A new sense entering at weight 0 changes no choice
until offline selection or training puts weight on it. The plan knows this
("Must be present in offline training, or founders ignore it") but spends
one clause on it. It is the largest cost and risk in the whole plan.
Concretely:

1. **The existing memory sense is unwired in the shipped ants.**
   `assets/minds/std_pool.txt` is marked `layout=4`. `mind::enter_last_food`
   zeroes every scorer unit's weight on `f::FOOD_WAY` when it reads a
   layout-4 file. `options.rs` says so outright: "a pool moves only as
   selection wires it". So on today's lab ants:
   - "Carry a lost ant to food — **now (mostly)**" does nothing;
   - §2's "any event that writes it … is a teaching channel with no new
     learning rule" does nothing;
   - step 0(a) does nothing;
   - the queen researcher's #1, "tandem running writes the existing
     last-food memory, so no plasticity is needed", does nothing.

   Each of these will reproduce slice 0's null result. Before any verb is
   built on `FOOD_WAY`, print the pool's weight on that slot. Then either
   train a layout-5 pool where last-food pays (food that moves within a
   life), or hand-seed the weight as a positive control. A unit test already
   does this (`mind.rs` ~4239 sets it to 8.0).

2. **Selection only wires a memory sense if memory pays in the training
   arena.** Theory (Stephens 1991: learning evolves when the world varies
   between generations but holds still within one life) and the training bed
   (food 25 columns out, fixed, per the `link_price` doc) point the same
   way: the bed doesn't reward memory. Selection will leave the weight at 0.
   Worse, every live link is billed every tick (`synapse_fraction` ×
   `link_price`; about 0.85 J a tick for about 1,900 links against an idle
   body's 0.1), and `zero_dead` cuts unread inputs on rung entry. So
   **the metabolic bill actively selects against listening to a sense that
   rarely pays**. `foodflip=` is the right arena, but it has to be *the
   training bed*, not only the test.

3. **The keeper's smell cannot be trained at all as designed.** If each
   identity (keeper, mint, rue, predator) is its own learned value and its
   own brain input, each needs its own trained weight. That needs a training
   bed that contains a keeper. There is none. The trust mechanic, the
   plan's justification for the keeper seat, would ship at weight 0 and do
   nothing.

4. **"Default on if neutral" plus weight 0 means a dead feature ships.** A
   sweep will read neutral, because nothing moves, so the feature ships
   on and does nothing. That is CLAUDE.md's "a change that moves nothing"
   trap. The plan's own proof for step 1 (learners beat frozen ants on
   foodflip) is right; it just has to be the gate, not one test on a list.

**What I would change in the representation:**

- **Make the learned layer identity-free at the brain.** Feed the brain a
  small number of *generic* option features. One is "this step climbs the
  gradient of a smell I have found good": Σᵢ vᵢ·∇sᵢ over a small fixed
  palette of odour channels. Another is its negative twin for "bad". Then
  one evolved family of 16 links serves *every* smell, including ones that
  never appeared in training (the keeper, a new plant). It trains on
  odour-tagged food in moving-food arenas. This is the single most
  important change: it turns "train a weight per cue" into "train once,
  teach anything".
- **Keep per-identity priors as genes**, so the Baldwin story has something
  to assimilate. Give each odour channel a heritable initial valence. "Born
  knowing mint is food" then means that gene has drifted up. Without it, an
  identity-free reader leaves nothing for assimilation to act on. With
  per-identity inputs only, nothing is trainable. You need both halves.
- **Hand-seed the read weight** at a modest value in the founder, as
  `enter_rung` already hand-writes weights. Then lessons work on day one,
  and evolution tunes "how much to listen" from there. Or put the learned
  value into a *body-level pull with a heritable gain*, as the many
  `*_pull` terms in `chooser_step` already are. That is cheaper, works on any
  founder, and keeps teachability a gene. Either way, say which and why.
- **Reconcile the verb tables with the layer.** "Learned: dig here",
  "dinner time", "skirt the spot where nestmates drowned" and "superstition
  at a pebble" all need *place* or *time* memory. The layer as listed holds
  smell values, task biases, a contact count and a reward average. Add two
  or three remembered points with a valence (generalising `last_food`), or
  cut those verbs.
- **Price the odour planes.** "Smells with identities" across food, every
  plant species, the keeper and predators means scent planes, and every
  plane is a per-cell diffusion cost every frame. Commit to a fixed palette
  of about 3 to 4 channels and quote `examples/ascii.rs` worst-frame cost.
- **The returner-contact count was already built and left off for harm.**
  `PIXEL_PHYSICS_RECRUIT` (Gordon's contact rate, built 2026-10-09) was
  shipped off because more adults starved (442 vs 298/323, 10 of 12
  seeds). The plan offers this mechanism as new and as "the biological
  answer to idle fed ants" without citing that. It needs a dead-ends grep
  and a stated reason it would come out differently this time.

**Does "teach it and see it in a minute" survive?** Only for a pool that
has evolved, or been given, weight on a generic learned-value input. Then a
one-trial update moves the next trip's choices: one trip is roughly
1,000 to 2,000 frames, so seconds to a minute at 1× depending on frame rate,
which the plan never states. On today's pool the answer is no.

**One more untraced foundation.** The plan explains the hand-laid trail
failure as realistic ("naive ants follow only 62 to 70%"). Slice 0 shows
that even the *positive-control ramp*, "far stronger than any trail a queen
can lay", moved nothing, and no worker was traced. A 62 to 70% choice in a
two-way test is a clear effect above chance (50%). Zero effect from a strong
ramp is not realism; it means trail B is not read in a way that recruits, or
the count is blind. Learned trust multiplying a signal the brain ignores
still gives zero. **Run the worker-level trace before building trust on top
of trails.**

---

## 4. Wrong or overstated claims

| Claim in the plan | Problem |
|---|---|
| A hand-laid trail doing nothing "is realistic" (62 to 70%) | Backwards. 62 to 70% is a strong effect above chance, and it comes from 20 nestmates marking in *Lasius niger*. Mass-recruiting species follow trails far more strongly; the plan's own "ant mill" entry shows that. The engine's null result is unexplained, not realistic |
| Tandem running is "the first proven case of teaching in a non-human animal" | The source says "first *claimed*". It was contested soon after (Leadbeater, Raine & Chittka 2006). It is specific to *Temnothorax* and a few other genera, not ants in general |
| "Real ants learn these in one to three trials" | True for odour–sucrose pairing. Not true for task specialisation ("rig success to make a specialist"), which in *Ooceraea (Cerapachys) biroi* took weeks of repeated success. "Lifelong forager" is stronger than the source |
| Leaf-cutters' refusal lasts "longer than the ants that learned it live" | The source says longer than *many* foragers live. The persistence is colony-level (smells and the waste dump), not individual memory |
| "Creatures and Black & White players drifted from teaching to breeding" | Black & White has no breeding. The notes support "teaching was opaque or tedious" and "Creatures' community bred". The stronger fact the plan omits: **Black & White 2's designers let players *buy* creature miracles instead of teaching them**. The genre's own designers judged teaching-by-repetition too tedious, and that is the most important warning for this plan |
| "A lesson taught long enough turns into instinct"; "generation 1 needed teaching, generation 10 is born knowing" | The Baldwin effect is real but slow and partial. Hinton and Nowlan's toy needed tens of generations, a population of 1,000 and strong selection. Learning also *shields* genes from selection (the hiding effect). Within one colony, where any ant may lay and selection is weak, 10 generations is about 120,000 frames. And a cue that only exists in this game (the keeper's smell) has no gene to assimilate *into* unless one is added (section 3) |
| "As Belyaev's foxes became tame" | Belyaev ran truncation selection by humans (the tamest ~10% bred each generation), not a learning-to-instinct effect. The analogy holds only where the player (or strong survival) actively selects on teachability. In the keeper seat, who selects? |
| "In honeybees all single-father colonies died over winter; diversity beats the elite" | Mattila and Seeley 2007 is a small study of colonies founded from swarms. Fine as a design idea; stated as a law. Whether diversity beats a single elite *in this engine* is a hypothesis for step 4 to test, not a premise |
| "Bless a deed and that worker's line becomes a father" | A queen mates once and stores the sperm. Workers are female and can only add *sons* (males). That's a fine game liberty, but label it as one if realism is a stated value |
| Returner-contact count is new ("mem") | Built 2026-10-09 as `PIXEL_PHYSICS_RECRUIT`, shipped off for a measured harm |
| "Carry a lost ant to food: now (mostly)"; §2 last-food row | Not on the shipped pool (layout 4, weight zeroed); see section 3 |

---

## 5. What is missing

1. **A prototype step that fakes the learning.** The plan's proofs are all
   metrics. Before training any pool, script the "learned" response by hand:
   ants that got a treat get a hand-set bias. Then play it and post it as a
   review card. That answers "is the teaching loop fun?" in days, before the
   most expensive step. The owner's playtests have overturned three models;
   this plan is a fourth model.
2. **Ambient legibility and attention tools.** Learned-state tint, taught-path
   overlay, follow-cam, time control. Hundreds of tiny ants with text on a
   page don't carry a loop.
3. **What the colony does *for* the keeper.** A keeper game is satisfying
   when the creatures' labour feeds back into the keeper's goals: the
   Stardew or Viva Piñata economy. Examples: ants carry pests off plants,
   aerate soil, plant seeds, and the plants feed the ants. "Pay the guard"
   and "ants plant the garden" are hints. A loop where teaching *pays the
   player back* is missing, and that is what stops it reading as chores.
4. **Graded failure and recovery.** What does losing look like? A colony
   should shrink, show why, and be recoverable, not starve to zero. The
   current boom-bust and starvation make this urgent.
5. **Queen pheromone and royal presence.** Researched in queen.md, dropped
   from the plan. It's a cheap colony-wide channel. Also patrollers choosing
   the exit (Greene and Gordon) as a keeper or queen verb.
6. **Becoming a nestmate.** Colony odour is learned. A keeper who smells of
   the nest (rubbed nest material) is accepted, which is how real
   myrmecophiles do it. That is a more natural "trust" than a trust number,
   and it ties into the guest seat.
7. **Cruel play.** Creatures shows that every verb that can hurt will be
   used to hurt (fling, bitter dust, drowning). Decide now whether that is
   supported, legible and recorded (graves), and make sure it can't poison
   a shared colony.
8. **Collective decisions as the teaching target.** Most of the plan
   conditions individuals. The interesting emergent behaviour (house moves
   by quorum, recruitment rate, where the brood lies) is colony-level.
   Teaching that shifts a *threshold* the colony decides with, such as the
   quorum for a move or the contact rate to go out, would reach the
   "unexpected" the owner wants more often than per-ant treats.
9. **No step looks before it measures.** CLAUDE.md's Method has "look first"
   and "verify live" as steps 1 and 3. The build table has neither, and no
   review card anywhere.

---

## 6. Top 5 changes, ranked

1. **Fix the weight-zero foundation before any teaching verb.**
   - Show the shipped pool's `FOOD_WAY` weight (it is 0: `layout=4`).
   - Make the learned layer feed the brain *identity-free* features (a
     valence-weighted smell gradient over a fixed 3 to 4 channel palette),
     plus heritable per-channel priors so assimilation can happen.
   - Hand-seed the read weight, or route it through a heritable-gain
     `chooser_step` pull, so lessons work on day one.
   - Make moving-food (`foodflip`) a *training* bed, so selection keeps the
     weight against the link bill.
   - Gate the default-on decision on "learners beat frozen ants", never on
     "neutral".
2. **Prototype the fun with a faked learner, as two 10-minute slices.**
   - One is the keeper/formicarium seat, the other the queen seat.
   - Both have a scripted learned response, and both get ambient overlays.
   - Post them as a blind review card before building steps 1 to 4.
   - This settles §8 decision 1 by play, not argument.
3. **Re-open the seat decision honestly.**
   - Restore the queen research's dropped verbs (B2/B3 feed and bite in the
     act, B6/U6 presence and absence, C1 brood as gravity, U1 the
     un-authored scent language, U4 your old colony as the rival).
   - Re-score the queen's minute-to-minute with them in.
   - Correct the cost argument: the queen's composition levers need no
     learned weights, while the keeper's teaching needs the riskiest piece.
   - Fit the keeper fiction to the sealed lab box (formicarium keeper,
     test-tube queen), not an outdoor garden.
4. **Design the first 10 minutes around one verb that delivers instantly.**
   - Remove "your first trail is ignored" from the opening.
   - Replace the binary beetle death with graded danger on foraging trips.
   - Define what happens to the player's body at first hatch. Recommended:
     *keep it*.
   - Name the second-to-second loop and its on-ant feedback.
5. **Trace and de-risk the foundations the plan builds on.**
   - Run the worker-level trace of trail B (slice 0's own recommendation)
     before trust is layered on trails.
   - Grep dead-ends for every reused mechanism (`PIXEL_PHYSICS_RECRUIT` is
     already there, with a measured harm).
   - Price the odour planes in worst-frame cost.
   - Turn "the spoilt colony" from a celebrated surprise into a guarded
     failure: graded decline, a visible reason, and recovery.

**The single biggest risk:** the plan ships a memory layer that is
"bit-identical when off" and "on because neutral", and it does nothing. The
lab ants have no weight on any learned sense. The training bed gives no
reason to grow one, and the link bill punishes trying. The keeper's smell
can never be trained, because no keeper exists in training. The player's
first lesson then lands on an ant that cannot hear it. That would be slice
0's trail result again, a model that looked right on paper, at the centre
of the game.
