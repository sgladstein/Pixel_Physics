# The evolution lab: what it is missing

*2026-09-09. Answers the owner's question — "examine the evolution lab game.
how can we make it more fun, better? think outside the box! what is it
missing?" — from a fresh look at the shipped box rather than from the round
history. Design of record remains
[`evolution-lab-design-guide-2026-08-30.md`](evolution-lab-design-guide-2026-08-30.md);
this report does not replace it, it answers the one question that guide's §9
explicitly refuses ("**whether it is fun.** Nothing here is a judgement about
play").*

---

## 0. The finding, stated first

**The lab is a finished instrument and an unstarted game.** Everything Gates
0–4 asked for is built and most of it is built well. What is missing is not
one thing called Gate 5; it is **four things underneath Gate 5, each cheaper
than the score, and the score cannot supply any of them.**

| what is missing | the box today |
|---|---|
| **Nobody in the box is anybody.** No names, no lineage identity, no history a player would retell | `RunLog` records `BORN / DIED / FIRST FED / FIRST SEED / LINE ENDED` against numeric ids |
| **Nothing happens *to* the player.** Running is uniform time with no shape and no interruption | no auto-pause, no event breakpoint, no regime schedule — grep for `auto_pause`/`pause_on` returns nothing |
| **Evolution is never shown as an event.** The one thing the game is named for is inferred from charts | no mutation is ever logged; `Mutat*` appears in `world.rs` and `lab/*` only as rate constants |
| **The animals cannot be seen.** The kingdom the whole game is about is 0.016% of the picture | measured below |

**And one relationship is missing that would do more than all four**: nothing
in this box needs anything else in it. Plants grow, animals eat plants, and
that is the entire ecology. There is no pollination, no dispersal, no
mutualism — `grep -niE 'pollinat|nectar|mutualis|symbio'` over `src/sim/` and
`assets/species/` returns **zero hits**. The box already grows **flowers no
animal has ever visited** and drops **fruit whose seeds nothing carries**.

**What this report is not.** It is not a bug list, and it deliberately does
not re-litigate §Z6 (the colony starves), which is filed, reproduced and
owned. §Z6 is upstream of everything here and nothing below substitutes for
it — but note that fixing it produces a box that is *alive and still not a
game*, which is why this is worth writing now rather than after.

---

## 1. What is already built — read this before proposing anything

The single most expensive mistake available here is proposing something that
exists. Measured against the shipped tree at `main`:

| built | where |
|---|---|
| Seven world verbs — `LOOK PLANT COLONY CULL SOIL WATER WALL`, plus `FOOD` and `PLACE` off the bar | `lab/ui.rs` `TOOLS` |
| A runtime parameters page reaching materials, species, traits, growth, reproduction, heredity, bed spec and simulation rules | `lab/params.rs` `Knob` |
| A rack of kept genetics — jars, `KEEP`, `PLACE`, a brood dial that drifts a release by N generations | `lab/ui.rs`, `lab/roster.rs` |
| Chambers and compartments — sealed partitions floor to ceiling | `lab/scene.rs` `compartments` |
| **Plainspeak** — an individual described in sentences rather than numbers | `lab/plainspeak.rs` |
| A run log that survives fast-forward, with a dropped-line count so an aged-out history cannot read as "nothing happened" | `sim/world.rs` `RunLog` |
| A biosphere page with decimated history over the *whole* run, kept whether the page is open or not | `lab/stats.rs` |
| A speed dial to 256x that reports **achieved** rate, never requested | `lab/time.rs` |
| Pin, marker and a follow camera | `lab/ui.rs` `RosterFollow` |
| A dead-individual record that outlives the slot | `sim/world.rs` `DeadRecord` |
| Species export **and** import — the lab reloads `assets/species/` at startup (`bin/lab.rs:192`), so the E8 loop is closed | `sim/species_export.rs` |

That is a lot, and none of it should be rebuilt. **The gap is not capability.
It is that none of this capability is pointed at a player.**

---

## 2. What the box actually looks like

`labshot frames=0,2400,12000,30000`, default bed, `RAYON_NUM_THREADS=2`.
Counts printed beside every stop, per `CLAUDE.md`'s standing rule:

| frame | plants | ants | births | deaths | biggest plant | roots reach |
|---|---|---|---|---|---|---|
| 0 | 8 | 52 | 0 | 0 | 2 | 0 |
| 2,400 | 7 | 52 | 0 | 0 | 153 | 10 |
| 12,000 | 124 | **4** | 1 | 49 | 322 | 20 |
| 30,000 | 206 | **13** | 13 | 52 | 318 | 25 |

**The picture is genuinely handsome** and this should be said plainly: grow
lights beaming down a dark chamber, brown soil, green plants with white
branching root systems visible below the surface. The roots are the best thing
on screen and nothing in the design documents mentions that they read well.

**Three things the picture says that no number in this repo says.**

1. **The animals are not there.** At frame 30,000 the box holds 13 animals of
   2 cells each: **26 cells of 163,840, or 0.016% of the frame.** I could not
   locate a single one by eye at any of the four stops, including frame 0 with
   all 52 alive. The design guide measured this as *"two dark cells at play
   zoom, findable only because it moves"* (§8.9) and treated it as a
   legibility problem to be answered by a page of numbers. It is worse than
   that: **it is the whole subject of the game being absent from the game.**
2. **Roughly 60% of the box is empty black air, permanently.** Plants reach
   about a third of the way up and nothing ever occupies the rest. The bed is
   512 wide and the interesting band is ~60 rows of it.
3. **The box becomes a garden.** By frame 30,000 the ratio is 206 plants to 13
   animals. Whatever the lab is about on paper, what it *shows* is a
   herbarium with an insect problem.

**One detail worth a second look, filed here and not chased**: at frame 12,000
the four surviving animals are **all four carrying food** while the colony
dies of starvation. Carrying and dying at once is a different failure from not
finding food, and `labforage` is the instrument. This belongs to §Z6's lane,
not to this report.

---

## 3. The four gaps, in the order they cost fun

### 3a. Nobody in the box is anybody

**This is the largest gap and the cheapest to close.** Every game that has
made people care about a simulated population — Creatures, Dwarf Fortress,
RimWorld — did it the same way: individuals have **names, parents, and a
record of what they did**. The lab has all three as data and none as identity.
`RunLog` already detects exactly the right moments: `FirstFeed` is *"the
moment a forager starts paying its own way"*, `LineEnded` is *"the last
individual of a founding line died"*. Those are the two best sentences in the
game and they are printed against an integer.

**What it wants**: founders get names, descendants inherit the line's name,
and the run log becomes prose. `FIRST FED  #4471` becomes *"Vernal-9 fed
herself for the first time"*. `LINE ENDED #12` becomes *"The Vernal line
ended, 14 generations from the founder."* Nothing new is measured. The entire
change is a name table and a formatter, and it converts a debug log into the
thing the player reads the game through.

**Why this is not decoration.** The guide's §5 score is *separation,
specialisation, persistence* — a good metric and, in its own words, *"possibly
a terrible readout"*. A named lineage is the readout. "Four separated
behaviour cells" is a number; "the Vernal line and the Kestrel line have not
interbred in 40 generations and neither survives the other's bed" is the same
fact and is a *story*.

### 3b. Nothing happens to the player

The Running phase is uniform. The player sets a dial, the box runs, the player
watches. There is no schedule, no interruption, and no reason to look at any
particular moment rather than any other. **Fast-forward with no event model is
a progress bar.**

Two absences, and they compound:

- **The box never calls you back.** No auto-pause on a notable event. The log
  already detects births, deaths, first feeds, first seeds and line
  extinctions; none of them can stop the clock or flash the screen. RimWorld
  and Dwarf Fortress both do exactly this and it is most of why their
  fast-forward is watchable.
- **The environment never changes on its own.** Grow lights are constant, the
  bed is constant, the walls are constant. Every "keep something alive" game
  has a **pressure schedule** — winter, drought, night — and the lab's whole
  air simulation (fan, heater, humidifier: guide §4) sits idle. A Running
  phase that is *"survive the dry spell"* has a shape; one that is *"run
  45,000 frames"* does not.

**Note the guide already priced the second half as nearly free**: the
equipment verbs *"switch on the air simulation, which today runs idle"*.

### 3c. Evolution is invisible as an event

The game is called an evolution lab and **it never once tells you that
something evolved.** Mutation happens, the dials that govern it are exposed
and tunable, and no mutation is ever reported. The player infers evolution
from a chart of aggregates.

**What it wants**: when a child is born differing from its parent, say how.
*"Vernal-9 born: gut +12%, sight −4%."* The diff is already computed — a child
is made by perturbing a parent's genome — and `plainspeak` already knows how
to turn a trait into a sentence. This is the single strongest legibility move
available in the whole lab and it is a formatter over data that already
exists.

It also fixes a specific dead end the repo has hit twice: three architectural
levers *fired* — 46–186 sympodial forks per shrub — and the owner's reading of
the sheets was that nothing had changed
(`Reports/plant-appearance-design.md`). A mechanism that fires invisibly is
indistinguishable from one that is dead. **Logging the mutation is the
counter beside the picture, applied to the genome.**

### 3d. The animals cannot be seen

§2 measured 0.016%. Three routes, and they are not alternatives:

- **A life overlay.** Render every animal as a bright dot with a short motion
  trail while Running. `render.rs` already carries `FieldOverlay` and
  `OrganismOverlay`, so the mechanism exists. It is not realism, it is a
  microscope filter, and the guide's own §2 says only that the lab must not
  draw as sky.
- **A magnifier as a *verb*, not a camera setting.** A lens the player drags
  over the bed, showing a circular inset at 8x with animals drawn large. This
  satisfies `CLAUDE.md`'s second law directly — *there must be a verb, and it
  must deliver something* — where a zoom slider does not.
- **A smaller bed.** See §5.

**Check the framing before concluding invisibility**, per `dead-ends.md`'s
`burrow_probe` entry — an image and a metric disagreeing is usually the
metric's fault. Here the metric and the image agree, and that same entry
records the residual finding in as many words: *"it remains true, separately,
that this feature is hard to see at play scale."*

---

## 4. The ecology has no relationships

**The guide's §5 is titled "scoring interesting behaviours and
relationships". The box contains no relationships.** One organism eats
another; that is the complete list of ways anything in this world interacts
with anything else.

And two halves of a relationship are already built and unjoined:

- **`CellType::Flower` is a terminal organ with a colour band and no
  function.** It is a fate a growing tip resolves to. No animal has any reason
  to approach one, and nothing happens if one does.
- **`windfall` is fruit that falls, piles at its angle of repose and rots.**
  Ants pick it up and carry it home. **The seed does not survive the trip and
  nothing germinates from a nest.**

**Closing either loop gives the box its first mutualism**, and a mutualism is
the thing that makes an ecology feel alive rather than adjacent:

- *Pollination*: a flower that only sets seed if an animal has touched it.
  Instantly, plants and animals need each other, isolating a compartment
  becomes an evolutionary event with a visible consequence, and the player has
  a lever with two sides.
- *Gut dispersal*: a seed that germinates **better** after passing through a
  gut. The colony becomes the plant's distribution network, and the player can
  watch a stand spread along a foraging trail.

**Price this honestly.** `dead-ends.md` records `seed_launch` on `tree`
failing because *"a dispersal lever priced per seed is a good trade only for a
species that is not already seed-limited"* — the same trap applies, and any
pollination requirement is a **new failure mode for reproduction** in a box
whose animals already die out. This is a mechanism to build behind a default
of *off*, or on a species that has seed to spare (the same report measured
+38% distant plants for `herb`, which does).

---

## 5. Three bets that are outside the box

### Bet 1 — the wide flat bed is fighting the game on three fronts

The bed is 512 wide, one open chamber by default, with a ~60-row band of
living space. That single choice is simultaneously:

- **the legibility problem** (0.016% animal, §2),
- **the divergence problem** (one panmictic pool; the guide's §5 score
  *requires* separated populations and partitions are the only source),
- **and plausibly half of the reach problem** (§Z6: one colony leaves 98% of a
  1024-wide stand standing and starves).

**A rack of small chambers — say 128 wide, camera zoomed 4x, four to eight of
them — attacks all three with one change.** Animals become 8 pixels. Isolation
becomes the default rather than a setting. A colony's foraging range covers
its own bed. And the shelf metaphor the game already uses (jars on a rack)
becomes the *world* metaphor rather than a menu.

The machinery is largely there: `compartments` walls a bed, `Chambers` is a
page over many, and the frame-cost line already measured *"walling a fanned
2048-wide bed into 16 compartments took it from 4.1x to..."*. **What is
untested is whether the camera and the UI survive it**, and that is a real
piece of work rather than a setting.

### Bet 2 — the player cannot cross two individuals, and every player will try

**Reproduction is asexual budding in both kingdoms**
(`plant-evolution-design.md`: *"asexual budding is the isolation"*). The shelf
lets you keep a jar and re-release it drifted by N broods. **It does not let
you cross two jars.**

This is the most conspicuous absence in the game relative to what its name
promises. Dog breeding, Pokémon breeding, pigeon fancying — the fantasy of a
breeding lab is *putting two things together and seeing what you get*, and the
lab's answer is currently "release it again and wait".

**This is not a small change and should not be sold as one.** Asexual
isolation is load-bearing for the plant line's whole clustering argument, and
adding recombination to the *world* would undercut it. But adding it to the
**shelf** would not: a `CROSS` verb on the rack that takes two jars and
produces a third is a *player tool*, not a change to how the world breeds. The
world stays asexual; the player gets the scissors. That is also exactly the
guide's own progression (§7b-i: selection only → mutagens → *"rare directed
splicing — a maybe"*), arriving earlier and cheaper than splicing.

### Bet 3 — the bed should be a record, and it nearly already is

`where-a-dead-plant-goes-2026-08-31.md` measured that **33% of everything that
dies is locked in `deadwood` for ever** — no `decays_into`, matter that can
never become soil or food again. It is filed as a defect, and as an *ecology*
it is one.

**As a game it is a gift.** This engine's soil is a physical medium; things
fall into it, rot in it and pile up. A box run for a million frames should
*accumulate its own history* — litter layers, old nest galleries, the buried
deadwood of a stand that died 200,000 frames ago. Digging into your own bed
and finding the tunnels of a colony you failed to save is the kind of thing
players tell each other about, and it costs nothing that the physics does not
already do.

**The bet is to make the strata legible rather than to make more of them** — a
soil-history channel on the overlay, or simply not sweeping the bed between
experiments and letting the player see what they are planting into. It also
answers the guide's §6 "start all over" question in the first law's own terms:
a failed experiment leaves a *layer*, not a blank box.

---

## 6. Everything, priced

Ranked by fun per unit of work, on my reading. **None of these is a call** —
§7.

| # | change | ~cost | what it delivers |
|---|---|---|---|
| 1 | **Names for founders and lines**, inherited; run log in prose | very low | Everything else in this table gets a subject |
| 2 | **Log the mutation** — "born: gut +12%" | very low | The game finally shows evolution happening |
| 3 | **Auto-pause on notable events** | low | Fast-forward becomes watchable |
| 4 | **A life overlay** — animals as dots with trails during Running | low | The subject of the game appears in the picture |
| 5 | **Sound.** There is none — no audio dependency in `Cargo.toml` at all | low–med | A living box that is silent reads as a diagram |
| 6 | **A regime schedule** — droughts, dark spells, cold, using the idle air sim | medium | Running phases get a shape and a stake |
| 7 | **`CROSS` on the shelf** — breed two jars (§5, bet 2) | medium | The verb every player will look for |
| 8 | **Pollination or gut dispersal** (§4), default off | medium | The box's first relationship |
| 9 | **A magnifier verb** (§3d) | medium | `CLAUDE.md`'s second law, applied to looking |
| 10 | **A rack of small chambers** (§5, bet 1) | high | Legibility, divergence and reach at once |
| 11 | **Commissions instead of a score** — a client wants an animal that survives condition X | high | §5's specialisation score wearing a costume the player can read; matches the owner's own "value scales with novelty" |
| 12 | **A museum of the dead** — `DeadRecord` as a place, not a row | medium | Failure becomes content; §6's graded loss with a face |

**Rows 1–4 together are perhaps a day** and would change what the game *is*
more than the score would. That is the report's actual recommendation.

---

## 7. What this report does not decide

- **Anything.** Everything above is a proposal. The owner's standing direction
  is *"stop balancing, start exposing"* and *"give me the tools... that is the
  game"* — several items here (regimes, pollination, commissions) add
  mechanism rather than access, and are exactly the kind of thing that
  direction is sceptical of. They are listed because the question asked was
  what is missing, not what is safe.
- **Whether §Z6 changes the answer.** A box whose colony survives a session is
  a different box and I have not seen one. Every judgement in §2 was taken on
  a bed whose ants die.
- **The order.** §6 ranks by my estimate of fun per unit of work, and this
  project's own history is that three separate models looked correct and were
  overturned by the owner's playtest. The ranking is a hypothesis.
- **Anything about the outdoor game.** The reverse of guide §8.8 is worth
  asking and is not asked here: not *"does the lab feed the main game"* — it
  does, the loop is closed — but **"does the main game feed the lab?"**
  Collecting a wild specimen outdoors and carrying it into the box is a
  motivation neither document has considered.
