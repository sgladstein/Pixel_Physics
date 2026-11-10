# Theme and motivation: who you are, what the mould is to you, why you care

Brainstormer: theme. Written 2026-11-08 from `context.md`, `src/mould/mod.rs`, the probe report and the playtest review. No code was changed or run.

## The best three, one line each

1. **The Wanderer (my bet).** You keep one named mould and walk it across a run of dishes from a kitchen table to a forest floor. The food under it runs out, so it has to move on, and you choose how. It is a pet and a journey at once, and the network you built is what carries over.
2. **Lifeline.** The dish holds things that cannot move and cannot eat on their own, such as sprouts. Your mould is their only supply line. This turns the owner's favourite moment, a bridge to a place with no food, into the goal itself.
3. **Little Railways.** Food piles become tiny villages that light up when the mould joins them. It is the Tokyo rail experiment as a toy. Its goals are the clearest of the three, but the mould becomes a tool, so it is better as a puzzle mode than as the heart of the game.

**The framing that fights the mechanic is spread-and-conquer, where the goal is to cover the map.** In this model the mould cannot cover anything. Each pile feeds at most 40 eaters, which is about 300 walkers, so a mould that spreads beyond its food starves grey. What makes it beautiful is a thin, short network, and a coverage goal rewards the opposite: a fat sheet. A **pet with need meters** (hunger bar, happiness bar) fights it more quietly. The mould's body already shows what it needs (yellow is fed, grey is starving), so a meter only covers up the best feedback the game has.

---

## What the mould gives a theme to work with

Every framing below is judged on whether it turns these into feelings, rather than painting a skin over them. All of them are real behaviour in this model today.

- **It reaches.** Exploring fingers go out from the front. To a player that reads as curiosity, as the mould wanting something.
- **It flinches.** The lamp moves the mould out from under the light in 0.02 to 0.07 s. A tube bends around the light like a rope pushed with a finger. That reads as shyness.
- **It is hungry.** Oats pull the front over in 0.5 to 0.7 s. That reads as appetite, and as trust: you put food down and it comes.
- **It gets stronger where it is used.** Tubes thicken along the paths it travels. To a player that reads as habit, and as memory.
- **It hurts when cut off.** Food only travels through tube, out to 70 cells from a pile the mould is eating. Beyond that it is unfed, it greys, and it dies over seconds. This is the strongest emotional beat in the game: a slow, visible loss that you could have prevented.
- **It can only reach so far.** A tube more than 70 cells from food starves at its tip. A bridge into nowhere is fed only by what is behind it.
- **The food under it runs out.** At full table, a pile loses about 144 food a second (40 eaters x 0.03 x 120 steps), so a 20,000 pile lasts about 2 minutes 20 seconds. As a pile empties its scent fades. The mould's home is temporary already. Today the 90 s dish ends before anyone notices.
- **Loops survive.** A ring of tube still feeds everything after one cut. That reads as resilience the player built.

The last two are the hidden gift. A dish where the food runs out is a dish that tells the mould to move on. That gives us a reason for the next dish without any clock.

---

## 1. The Wanderer: keep one mould and walk it home

**The fantasy.** You found a slime mould and you are keeping it alive as it migrates across the world, one patch of ground at a time.

This is a real hobby. People keep *Physarum* as a pet on agar and feed it oat flakes. So "you, with a torch and a pocket of oats" is not a stretch, and the opening can say so. You name the mould on the first screen.

**What you want, and what you fear losing.** You want it to arrive somewhere good: big, whole and well fed. You fear losing pieces of it. Every walker that starves is part of *your* mould, and the run remembers how big it is.

**How the behaviour becomes the beat.**
- *The food runs out.* The pile the mould sits on drains over about two minutes, and its scent fades as it goes. The mould gets restless. Its fingers reach further, and the yellow at home dims. You do not need a timer to say "time to go". The mould shows it.
- *Starving grey* is now the price of moving. When the mould leaves an old pile, the tubes behind it go grey and die back. If you lead it well, a thin trailing line dies, and that is a graded and acceptable loss. If you lead it badly, half the body dies. That is the spread the owner asks for: not lived or died, but how much of it came with you.
- *Lamp and oats* stop being "tools" and become how you lead an animal: a light to shoo it, food to coax it.
- *Thick tubes* are the route you chose. The thicker the trunk to the exit, the more of it makes it through.

**A single dish.** Take "The Kitchen Table", the first one. The dish is the same 256x160. On screen, the mould sits on a crust of bread at the left. Two crumbs lie in the middle. On the right edge is a crack in the table, glowing faintly: the way on.

In the first 30 seconds the bread is already shrinking. You drop an oat between the bread and the first crumb, and the mould leans over within a second. You hold the torch over a finger that is wandering off to the bottom edge, and it bends back.

The dish ends when you choose, or when the last food runs out. When the front reaches the crack you press Go: every walker *fed* at that moment crosses into the next dish. The rest stay behind. **Winning is a number, not stars:** "Mabel arrived with 1,140 of 1,500." Hazards fit the fiction. The storm becomes a footstep or a drip from the tap, warned 5 s ahead as now. A patch of sunlight drifts slowly across the dish, and it is simply the lamp's push code driven by the world instead of by you.

**The arc.** Each dish is a stretch of the journey: kitchen table, back step, garden path, log, leaf litter, and somewhere damp and dark to rest. Three ideas carry through:
- **It grows.** Its size at the start of each dish is what arrived from the last one.
- **It remembers.** The tube shape at the exit is laid into the next dish's left edge as faint trail, so the mould starts each stretch in the shape you left it in.
- **It has offspring.** At the end of the journey it fruits. The number of stalks is how well kept it was, so it is graded again. You pick one spore, and its child starts a new journey, slightly different. The model already has genes in `Params`. `sensor_offset` (how far ahead it feels, now 9) sets how long its fingers are. `divide` sets how fast it grows, and `deposit` how thick its tubes get. Change one and the network looks different. A child that reaches longer but grows slower is a real change in how it plays.

**Where the bridges fit.** The fact behind the bridges is the 70-cell reach. A gap of bare table wider than the mould's reach cannot be crossed on habit alone. You have to build a bridge: oats as stepping stones to pull it across, and the torch to keep it narrow. The oats are eaten in a few seconds, so the bridge stays alive only while it is joined to food at one end or the other. Getting a bridge to stay fed across a gap is the skill the whole game teaches.

**What it needs that does not exist.**
- An exit edge and a Go. The "fed at the moment you leave" test is the existing `fed` array.
- Carry-over between dishes: walker count, plus the exit-edge trail laid into the next dish. Today `Dish::new(seed)` starts fresh.
- An end that comes from food running out rather than from `DISH_SECONDS`. Piles already drain, so this is mostly removing the clock.
- A drifting sunlight patch: `push_light` with a scripted lamp position.
- Naming, and a final count shown beside a picture of the mould.
- Fruiting and a child built from `Params` (the probe brief left fruiting out on purpose).

**Why it is my bet.** It answers the owner's question in his own words: you are the caretaker, and it gives the caretaker a direction. It uses the economy the probe already has (food runs out, and starving is graded) as the story engine, instead of adding a story on top. It keeps the free play he enjoyed, because there is no clock forcing your hand. And every dish ends on a number about *your* mould, which is the thing a player keeps coming back for.

---

## 2. Lifeline: your mould is the only way food reaches them

**The fantasy.** The dish is full of small things that cannot move. Your mould is what feeds them, and it does so by choice, at its own cost.

**What you want, and what you fear losing.** You want every sprout on the dish to flower. You fear the moment a lifeline goes grey and the sprout at its end starts to wilt.

**How the behaviour becomes the beat.**
- *Sinks, not only sources.* Today every node is food the mould eats. Lifeline adds the other kind: a sprout that *draws* from the mould. Because the mould has one shared store, feeding a sprout costs the mould size. A sprout that is joined takes upkeep for, say, 40 walkers. That is a real caretaker's trade, and the player can see it: one more sprout means a slightly smaller mould.
- *Starving grey* now has two victims: the tube, then the sprout at its end. The sprout wilts in stages (bud, leaf, droop, dead) over 10 to 15 seconds after it is cut off. That gives you time to rebuild, and it makes the death graded.
- *Loops* matter for someone else. A sprout fed from two sides survives the storm, and you can see why.
- *The 70-cell reach* sets the puzzle. A sprout 60 cells from food can be fed. A sprout 100 cells out needs food planted halfway, or a second pile joined from the far side.

**A single dish.** Take "The Seed Tray". Two food piles sit at the top and four sprouts in a row along the bottom, each about 50 cells from the nearest pile. In the first 30 seconds the mould swells onto the first pile. You drop oats toward the nearest sprout, and a tube reaches it. The sprout's bud opens and a tiny leaf appears. Food is now visibly flowing to it: that needs pulses riding the tube, which the probe left out as "dye". Then the second sprout. The dish ends on a count: three sprouts flowering, one in leaf.

**The arc.** The sprouts you raised become the next dish's ground. A flowered sprout drops seeds, and they become food piles (or more sprouts) in later dishes. Over a game you turn a bare tray into a little garden, and the mould is how it happened. This is the only framing where the world visibly changes because of you, dish after dish.

**Where the bridges fit.** Perfectly. The owner liked building bridges where there was no food. In Lifeline, a bridge to a place with no food *is* the goal. Every sprout sits exactly where there is nothing to eat.

**What it needs.**
- A sink kind of node. It is joined when the existing `fed` search reaches its cells, it charges the shared store, and it grows when paid and wilts when not.
- Drawing a sprout in stages.
- Pulses along fed tube, so delivery is seen, not inferred.
- Carry-over of sprouts into seed piles, if there is an arc.

**Risk.** The mould becomes the means and the sprouts become the point, and the owner said he connects to the mould. The fix is to keep the mould the hero: it is the one acting, and it pays for every sprout out of its own body.

---

## 3. Little Railways: lay the roads between villages

**The fantasy.** The dish is a tiny landscape of villages, and the mould is the road builder that joins them.

This is the famous 2010 experiment, in which the mould redrew the Tokyo rail map, played as a toy.

**What you want, and what you fear losing.** You want every village joined by the shortest roads, with spare routes. You fear a village going dark.

**How the behaviour becomes the beat.** When the mould reaches a pile, the village there lights its windows. When a village is cut off, its lights go out one by one as the tube greys. Thrift (already a star, at 1.15x the shortest network) means something in plain words: "short roads". Toughness means "no village cut off by a flood". Thick tubes are busy roads, and they could carry little pulses like trains.

**A single dish.** A real-looking little map: a river (salt, already in the game) and six villages. A seventh village is founded at 0:55, just as the far pile turns up today. Win by joining all of them with short roads and riding out the flood. That is the current three stars with words that mean something.

**The arc.** A map per region, ending on a recognisable country. It needs almost nothing new: a reskin of piles and the storm, lights on joined piles, and maps drawn by hand instead of seeded.

**Where the bridges fit.** Bridges are literal: the road over the river.

**Why it isn't my bet.** It has the clearest goals and costs the least, but you do not care about the mould. You care about the villages. It is the right shape for a puzzle pack inside one of the other two, maybe "the mould's other life" dishes. It is not where the owner's attachment lives.

---

## 4. The Lab Mould: a gardener in the terrarium

**The fantasy.** The mould lives in the lab, the engine's sealed box of soil with real plants and ants. It is the box's recycler: it eats what dies and keeps the box alive.

**What you want, and what you fear losing.** You want a box in balance. You fear the rot piling up, or the mould and the ants starving each other out.

**How the behaviour becomes the beat.** Food piles are the real plants' dead leaves, which `rot_remains` already carries off at each species' half-life. Tubes run between them across real soil, and ants could walk them or cut them. A dish is a season; winning is a box still alive at the end. Bridges cross a crumbling soil face.

**What it costs:**
- The probe chose a private field on purpose. The engine's trail plane (`sim::pheromone`) is `u16`, chunk-bound and stepped on the world's interval, and the ants already use it.
- The mould never settles. A mould in the world keeps its chunks awake and costs the dirty-rect skip wherever it lives. Today it costs 2.6 ms a frame on a 256x160 dish, and that would rise with the area it covers.
- Herding with oats does not fit a sealed box. The player would have fewer verbs, and that is the second law broken.

It is a strong *later* direction for a mould that already works as a game, and the wrong first step.

---

## 5. Spread and conquer, and why it fights the mechanic

**The fantasy.** Your mould takes over the dish (or the kitchen, or the house), beating a rival to it.

**Why it fights the mechanic.**
- *Size is capped by food.* One pile feeds about 300 walkers. A mould cannot hold ground it is not eating from. "Take over" means getting fat, and getting fat means starving.
- *It rewards the wrong shape.* Coverage rewards a sheet. The beautiful thing, and the thing the owner played with, is a thin network of tubes and loops. The thrift star and a coverage goal point in opposite directions.
- *The emotional beat flips.* Starving grey becomes the cost of greed, not a loss you feel.

**The part worth keeping.** A **rival** as a threat inside another framing. In the Wanderer, a fairy ring of mushroom mycelium could grow from the middle of the leaf-litter dish and eat the crumbs first, so you have to reach the food before it does. That needs a second population with its own trail field that repels yours. Jones's model supports this, but the probe does not have it. Racing for food keeps the mould thin and hungry. Conquest makes it fat.

A sibling to avoid: **containment**, where you are the scientist keeping a mould out of the kitchen. It uses the verbs well (the lamp pushes, salt walls), but it makes the mould the enemy, and the owner said he connects to it.

---

## 6. A briefer one: Symbiosis with something that rides it

A small beetle can only travel along your tubes, and you build it roads. This is Lifeline with a passenger that moves. It needs a creature that walks a tube, and it splits the player's attention, so keep it as a guest in one Wanderer dish.

---

## What the good framings share, and two changes they all want

1. **Score the outcome as an amount, not as three yes/no stars.** The first law says an outcome is a spread. Three stars are three yes/no answers. "1,140 of 1,500 arrived", "three sprouts flowered and one is in leaf", or "five villages lit and one dark" are each a spread, and each is about the thing you care for. Stars can stay as a summary beside the number.
2. **Let the food running out be the clock.** Piles already drain in about two minutes and their scent fades. A dish that ends because the mould *has* to move on gives the middle minute a reason, and keeps the free play the owner liked at the start.

## The bet, plainly

**Build the Wanderer, and put Lifeline's sprouts in it as some of its dishes.**

The Wanderer answers "why do I care": it is your mould, it has a name, and it is going somewhere. It makes the starving grey matter: a piece of it dying is a piece of *it*, counted at the end of every dish. It uses what the model already does (draining food, the 70-cell reach, a graded starvation) as the reason to act, so the theme cannot be removed without the mechanics changing. And the owner's favourite moment, the bridge across nothing, is the skill it is built around.

Lifeline's sprouts are the best single idea here for *giving bridges a purpose*. As some of the Wanderer's dishes ("on the way, it passes a seed tray"), they add a reason to reach for someone else without taking the mould off centre stage.

The smallest first step to test the feeling is one change in the current build: replace the 90 s clock with an exit on the right edge, a Go key, and "Mabel arrived with N of M" in place of the stars. That is enough to show whether "lead it somewhere" feels better than "score this dish" before any of the rest is built.
