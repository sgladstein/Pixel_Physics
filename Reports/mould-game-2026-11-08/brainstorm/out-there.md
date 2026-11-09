# Out there: what else the mould could be (2026-11-08)

Brainstormer: surprising directions. Read: `context.md`, `src/mould/mod.rs` (the model, step by step), `play.rs` header, the probe report's *What was built* and *What the owner's play found*, the playtest review, the 2026-11-07 synthesis, and `Reports/dead-ends.md` (grepped for mould and Jones). Nothing was run or edited.

## The best three

1. **Mould Organ.** A pulse runs from the mould's heart along every tube, and each food node rings a note when the pulse reaches it. Tube length is time, so bending a tube with the lamp bends the rhythm. *Why: every push and pull you already enjoy becomes something you can hear, at once, with no instructions needed.*
2. **Dry Rot.** Inside the full falling-sand world, you are a fungus in a timber tower. Wood eaten by your tubes softens, so you choose where the tower breaks and where it falls. *Why: the network decides which beam goes, and the engine's own collapse is the payoff.*
3. **Tug.** Two moulds share one dish, gold against violet, and each is pushed off by the other's trail. Your lamp pushes both, and cutting a rival's trunk starves everything beyond it. *Why: a fight you can only win by redrawing the network, and a cut gives a slow, visible comeback.*

If I could build only one next, it would be **Mould Organ** (reasons and a first test at the end).

---

## What the fun is, exactly

This is how the model gives the fun the owner described, in the terms the code uses:

1. **You set the pressure, and the mould draws the line.** The lamp pushes and oats pull, but both work on fields, not on orders. Each walker turns toward the strongest of three sensor readings 9 cells ahead, so the route is the mould's own choice. The result does what you asked and still surprises you. It also answers fast enough to feel like touch: oats move the front in 0.5-0.7 s, and the lamp clears what is under it in under 0.1 s.
2. **It remembers.** Walkers follow trail and lay more of it. A tube that is used gets thicker and stays, and one that is not used fades (each step keeps 90% of the trail). So a gesture turns into a structure. **That is why a bridge with no food at either end can survive: walkers keep travelling it, so it is a road that exists because it is used.** It also has a real limit. Food only reaches 70 cells of tube from a pile, so a bridge that runs further than that from food goes grey and starves.
3. **It is a rope, not a drawing.** Since 2026-11-08 the lamp moves trail and walkers outward, one cell at a time (`push_light`), so a tube bends round the light and stays joined. The lamp is unlimited and acts continuously, which makes it a sculpting tool.
4. **What is joined lives, and you can see it.** The feeding search makes the network's shape visible: yellow tube is fed and grey tube is starving. A cut fades over seconds rather than vanishing, so being connected matters, and the cost of losing a connection comes gradually.
5. **It has taste.** Left alone, it tidies a messy network into short paths and loops. You are working with something that has its own idea of what looks good.

**The test for each idea below:** it has to keep the first four. If the mould does not choose its own route, does not remember, cannot be pushed like a rope, or being joined does not decide what lives, the idea is a reskin.

---

## The seven ideas, near to far

### 1. Constellations (near: still slime mould, a goal nobody would expect)

**Pitch.** The dish is a night sky, and its food piles are stars. Each night you coax the mould into one constellation, including lines and bends the mould would never choose by itself. When the figure holds for five seconds it lights up and goes into your star chart.

**First 30 seconds.** A faint ghost of the Plough lies over seven food stars. On its own, the mould joins the two nearest stars by the shortest line. But the handle bends where a shortest path would not, so you lean the lamp against the tube until it sits in the ghost line, and it stays there because the tube remembers.

**Why the network is the heart.** The target is built to be *not* what the mould wants. Some lines are not the shortest. Some figures are loops. Some lines end in nothing, which makes them bridges with no food that starve past 70 cells. The whole game is working against the mould's taste, gently.

**Hook.** The chart fills up, and each night is harder. A free "ink" mode lets you draw your own figure and keep the picture.

**Reuses / needs.** It reuses all of the model, and the thrift census is close to an overlap measure already. It needs ghost figures, a score for how much of the ghost line is covered by fed tube (minus tube that lies off the figure), and a chart screen.

**Risk.** Asking for precision from a wobbly medium may feel fiddly, which is the exactness trap the owner warns about. The tolerance has to be generous: about one tube width (12 cells).

### 2. Tug (versus or co-op)

**Pitch.** Two moulds, gold and violet, grow from opposite corners toward the same five piles. Each one's sensors read its own trail minus the rival's, so a tube is a wall to the other side. Your lamp pushes both, so you can shove the rival's trunk off a pile or lead your own across it.

**First 30 seconds.** You play a friend (two mice, or split touch), or you play the scripted plan from `play.rs` driving violet. Both blobs open toward the piles, and at about 15 s they meet in the middle. The tubes press against each other and bend like two ropes leaning on one another.

**Why the network is the heart.** The border is the network itself. You cannot attack directly. You win by growing across the rival's supply line, so their far side goes grey and dies over seconds, or by luring their front off a pile with oats dropped on your side. Every fight is a change to the network's shape. **The co-op version** puts one mould under two players, one holding only the lamp and one holding only the oats. Push and pull become a conversation between two people.

**Hook.** Comebacks. A losing player can cut the leader's trunk and watch an empire go grey. Rounds of 90 s, best of three.

**Reuses / needs.** It reuses all of the model, and the plan player can be the AI rival. It needs a second trail field and a colour per walker (twice the spreading cost, which is trivial at 256x160), sensors that subtract the rival's trail, a food store and census for each side, and two cursors.

**Risk.** Two self-organising growths on one screen are hard to tell apart; the 2026-11-07 synthesis rejected rust-against-mould for exactly that reason. The fix is strong colours and moulds that never mix, so where they meet is always a visible line.

### 3. Mould Organ (music)

**Pitch.** Every two seconds a pulse leaves the mould's heart and races out along every tube, and each food node rings its note when the pulse arrives. Tube length is time: push a tube aside with the lamp and its note comes late. Meanwhile the mould slowly tidies itself toward shorter paths, so your groove tightens on its own.

**First 30 seconds.** A dish with a heart and three coloured bell nodes, low, middle and high. The mould reaches the low bell, and you hear *dum ... dong*. You drop oats toward the high bell, a tube follows, and a third beat joins the bar. You lean the lamp on that tube and hear the high note drift later, into a swing. No instructions needed.

**Why the network is the heart.** A note's timing can only be set through the length of its tube, and that can only be changed indirectly: bend it with the lamp, shorten it with oats, cut it with Snip. Every rule of the model turns into music:
- **A loop** gives two routes to one bell, so the bell rings twice, an echo.
- **A thick tube** carries a stronger pulse, so its note is louder.
- **A starving branch** fades its note out gradually, which is law 1 of CLAUDE.md, heard.
- **A bridge with no food** is a delay line.

**Hook.** "Match the rhythm" puzzles: a ghost rhythm plays, and you shape the network until it plays the same. The score is graded: how many notes land within a sixteenth of the beat. Free play is a toy you leave running, and it drifts slowly as the mould keeps working.

**Reuses / needs.** It reuses the whole model, and the feeding search is most of the pulse already. `fed[i]` is "cells of tube from the nearest food", which is an arrival-time map. The pulse needs the same search started from the heart alone, which is cheap at 256x160 once per pulse. Sound exists: `src/audio/synth.rs` has tones, and `examples/soundtrack.rs` renders the same synth offline to WAV, so the owner can hear a result on an Artifact page. It needs a heart node, the travelling glow (lit where the distance from the heart equals speed x time), bell nodes, snapping each arrival to the beat, and target rhythms.

**Risk.** A tube's edges wobble, so its length changes from frame to frame and the timing jitters. Rough numbers: at one cell per step (120 cells/s) and 120 beats a minute, a sixteenth is about 15 cells of tube. Wobble of a few cells disappears once arrivals snap to the beat. If the wobble turns out to be 15 cells or more, it is mush. That is the first thing to measure.

### 4. Desire Lines (city: not mould at all)

**Pitch.** You run a new park that has no paths. People walk between the gates, the café and the pond, and the grass wears into paths where they go. You cannot lay a path. You can only place things: a fountain draws people, a flowerbed blocks them, a busker's crowd pushes them aside. Once a path is worn deep enough, you may pave it, and paving is forever.

**First 30 seconds.** A green lawn with three gates and a café. Dots start walking, and within seconds brown lines wear across the grass. You put a fountain in the middle of the lawn and the paths bend to pass it.

**Why the network is the heart.** Desire paths really are Jones trails: people follow worn ground and wear it more. The new verb is **paving**, which turns a tube into permanent ground at a price. The rest of the network keeps flowing round what you have frozen, so the decision is *when* to commit and *which* path. The score is graded: trips completed and how fast, against the lawn kept green (thrift becomes "grass saved").

**Hook.** Seasons grow the park into a town. A bus stop appears halfway through (the late food), and a rainy day makes muddy paths spread wider. Each year your old paving shapes the new streets: SimCity grown, not built.

**Reuses / needs.** It reuses walkers, trail and scent (the destinations). The lamp becomes the busker, salt the flowerbed, Snip a fence. It needs walkers with a destination that pick a new one when they arrive (today's walkers have no goals; scent does all the work), a paved ground type with a permanent trail floor, and a fixed crowd instead of food-paid growth. That last one changes the economy, and it is the biggest change here.

**Risk.** It could still be a reskin. It earns its place only if paving and destinations change what the player decides.

### 5. Dry Rot (inside the full Pixel Physics world)

**Pitch.** Seen side on in the falling-sand world, you are a fungus inside a timber watchtower, and every tube you grow softens the wood it runs through. The lamp pushes the rot and wet wood pulls it. You choose which beams go, and so whether the tower falls onto the weir you were sent to block or onto the village.

**First 30 seconds.** A wooden tower on a cliff edge, a village below on the left, a weir below on the right. The mould starts in a damp sill at the base. You splash water up the right leg and the mould races up the wet wood. The beam darkens, and at about 25 s the right leg sags under the tower's load.

**Why the network is the heart.** Which beam fails is decided by the network's shape: thick, busy tubes rot fastest, and starved ones stop. **The player never deletes a cell. They only decide where the network runs.** The wood is also the food, so a strong network is a destructive one. The collapse is the engine's own graded breakage (blocks, cobbles, grit), so law 1 comes free, and law 2 is the whole game: you strike the tower by growing.

**Hook.** Every collapse is different, and big. The puzzles name a target ("drop it in the river", "take the bell tower, keep the chapel"), and fire is a counter-tool that burns thin, dry tubes away.

**Reuses / needs.** From the mould it reuses the walker, the trail fields and the feeding search, with `Ground::Agar` meaning "wood or damp soil" instead of "dish". From the engine it reuses wood and rootwood, the structural collapse (`structural.rs`, `load.rs`), water and fire. It needs a softness value per cell that weakens load-bearing wood and, past a threshold, turns it to rot. **One trap is already recorded:** `Reports/dead-ends.md` (the *creature-direction §13b* entry) found that Jones sensors fail for a walker on a *surface* in side view, because both side sensors land in open air. Walkers *inside* a solid are a flat 2D region again, so rot inside wood is safe, but rot creeping over a surface would hit that wall.

**Frame cost risk, named.** Private float fields at world size (512x320) mean two spreads of 164k cells a step, likely a few ms, which has to be measured with `ascii`. The bigger costs are world writes. Any cell the rot changes wakes its chunk and breaks the dirty-rect render skip, and any load-bearing cell that changes can set off a structural recompute. The rule: the mould is drawn as an overlay that writes nothing to the world, and softening is committed in batches (one beam at a time, about once a second). The number to quote is `ascii`'s worst frame on a rot scene.

*Its constructive twin, **Root Bridge**:* train a fig's roots across a gorge along bamboo scaffolds you lay, the way living root bridges are grown in Meghalaya. Ants walking across thicken the roots, and a flood tests the bridge at the end of the season. It is the owner's "little bridges" made literal and load-bearing, but it is slower to pay off, and running water is the engine's most expensive thing to keep awake.

### 6. Grow a Brain (far: neurons)

**Pitch.** The dish is the inside of a small creature's head, with eyes on one rim and legs on the other. Whatever you join with mould becomes its wiring. Beside the dish, the creature walks a little world. A thick tube from the left eye to the right leg makes it turn toward light.

**Why the network is the heart.** These are Braitenberg vehicles, where behaviour comes from wiring: thickness is strength, starving is forgetting, and Snip is an injury. You never program it, you only grow it.

**Hook.** Watching your wiring act out, in puzzles like "get it to the sugar". In the first 30 s it walks in circles until you lure an eye-to-leg tube, then it steers.

**Reuses / needs.** It reuses the model. It needs a rule that turns joined, fed tube into motor output, and a second view. It must be its own creature and must not touch the shipped ant's evolved brain, which the lab depends on.

**Risk.** Two screens, and a long gap between your action and its consequence. The hardest of the seven to read on a phone.

---

## Ranked by how much I'd want to play the first ten minutes

| Rank | Idea | Why that rank |
|---|---|---|
| 1 | **Mould Organ** | Every gesture is heard within a beat. It needs no lesson, and it is a toy before it is a game, which is how the owner already plays this. |
| 2 | **Dry Rot** | The biggest payoff of any idea here, built on the engine's best mechanic. Ranked second only because the first ten minutes depend on a costly build being smooth. |
| 3 | **Tug** | With a friend, the comeback cut is a story every round. Alone against the plan player it is weaker. |
| 4 | **Desire Lines** | The most surprising fit with the real world, and paving is a genuinely new decision. Slower to pay off. |
| 5 | **Constellations** | Lovely and calm, and the closest to the owner's "bridges with no food". It risks feeling fiddly. |
| 6 | **Root Bridge** | Great theme, slow payoff, costly water. |
| 7 | **Grow a Brain** | The best idea on paper and the worst first ten minutes. |

## What I'd prototype next: Mould Organ

- **It is the cheapest true test.** It touches no shared engine file, it reuses the whole model plus a heart-first version of the existing feeding search, and the synth and the offline WAV render already exist. A first version is a few days of work.
- **It answers a question nothing else here can:** does steering a network feel good when the reward is a sound rather than food joined? If it does, the mould has a second life as a toy, and the main mould game could borrow it: a pile you join could ring, which is law 2 for free.
- **Step 0, before any sound:** on a settled dish with a heart and three nodes, measure how many cells the tube distance from the heart to each node wobbles over 10 s. Under about 5 cells, the timing reads. Over about 15, stop: the medium is too noisy to play music on.
- **Show it as a page:** one GIF of the glow running, with the matching WAV beside it, and the count of notes that land on the beat against the count that do not.
