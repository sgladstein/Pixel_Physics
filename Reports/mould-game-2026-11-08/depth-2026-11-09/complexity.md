# Mould: adding depth through interactions (2026-11-09)

Game designer brief for Scott's ask: *"our tools/actions, events, the environment and the slime mold responses are relatively simple, mostly cause and effect, one dimensional ... some of these should interact in ways [that add] complexity, depth, and discovery."* Read from `src/mould/` at `f8df782ce` (mod.rs Params, play.rs, level.rs, wander.rs, organ.rs, tune.rs, look.rs), the probe report's *Food that flows* section, the 2026-11-08 brainstorm folder and `Reports/dead-ends.md` (the three mould entries). Nothing in the repo was edited or run.

## Summary

**Add one thing, water, and let four things the game already has react to it. Most of the depth comes from those reactions, not from new tools.**

1. **Water melts salt.** Salt only moves through wet agar. A dry wall stays sharp. Wet it, and it opens into a gap with a wide, weakly salty halo the mould can cross but pays to cross. So a wall becomes something you can dissolve, and the storm's rain softens every wall on the dish.
2. **Dry mould crusts instead of dying, and water wakes it.** The crust is the real organism's dormant form, the sclerotium. Crusted tube holds its shape and its food and survives the storm's cut. So a player can protect a trunk by drying it with the lamp before the storm, and the rain that follows wakes it. That is a second answer to the storm, besides building a bypass.
3. **Hunger flips the lamp.** A starving branch under light stops fleeing the lamp and climbs into it to fruit. The fruit releases spores, which the storm's wind carries over salt, and they sprout only where the agar is damp. A dying branch becomes a way across a wall.
4. **Wet agar carries scent further, rots oats faster and speeds food along the tube.** So where you drop water changes what your oats and tubes do. This one rule fills more cells of the matrix than any other (39 of 85).
5. **The mould learns salt, and the lesson travels.** Fed mould that stands in weak salt slowly gets used to it. The tolerance spreads to neighbouring walkers by the same rule that already spreads food. It survives the crust and rides the spores, so a trained mould can be carried into the next dish.

**Why this creates discovery:** each verb has one obvious meaning and at least one second meaning that only shows when it meets another field. The lamp pushes, and also dries. Water feeds growth, and also melts walls. The storm cuts, and also rains. Nobody needs to be told these. The player finds them by doing one thing near another, and the dish shows the result in its own pixels.

---

## 1. The few rules underneath

The matrix in §2 has 85 filled cells. None of them is a special case. Each one is what happens when two of these rules touch the same cell. The first four rows of the table below already exist in the code; the rest are new.

| Field (per cell, unless noted) | Exists? | Who writes it | Who reads it |
|---|---|---|---|
| `light` | yes | lamp | push (exists); **evaporation (new)**; fruiting |
| `trail`, `scent` | yes | walkers, food | sensors (exist); **scent keep rises with wet (new)** |
| `sap`, walker load | yes (`flow`) | piles, stream | economy (exists); **conductance x wet (new)** |
| `wall` | yes (`flow` level 3) | food moving | sensors, conductance (exist); **frozen under crust (new)** |
| **`wet`** 0..1 | **new** | water drops, rain, fed tube (optional); lost to light, salt, dry spells | pace, scent, oats decay, wound heal, sap conductance, crust, spore landing |
| **`salt`** 0..1 | **new**: replaces the on/off `Ground::Salt` | salt tool, level walls; moves only through wet | entry (wall above a threshold), upkeep tax, scent loss, drying, learning |
| Walker `state` | **new** | rules R5 and R6 | Active / Crust / Fruit |
| Walker `tolerance` | **new** | R7 | salt threshold and tax |
| Spore particles | **new** | fruiting bodies | wind, landing |
| Walker `strain` (last) | **new** | dish setup | sensing, fusion |

The rules, in plain words with the formula behind each:

- **R1. Water moves and dries.** `wet` spreads slowly (a 5-point spread every 4 steps) and evaporates at `evap * (1 + dry_light*light + dry_salt*salt) * weather`. So the lamp dries what it shines on, salt dries its own edges, and a dry spell dries everything. Water drops and rain add to it. **The baseline is set so a dish nobody waters behaves exactly as today** (see §6, "baseline").
- **R2. Salt moves only in water.** `salt` spreads between two cells at a rate of `salt_bleed * min(wet_a, wet_b)`, so a dry line never moves. A cell with `salt > salt_wall - tolerance` cannot be entered, which is today's wall. Below that, the cell can be crossed but taxes upkeep (R4). Scent fades in salt at `1 - salt_scent*salt`, so at full salt it is today's "scent can't cross salt" rule, now graded.
- **R3. Water sets the pace.** A walker's chance to step, and its chance to divide, scale with `pace(wet)`, which is 1.0 at the baseline, lower when dry, slightly higher when damp. Scent keeps `+ wet_scent*(wet - base)` more per step, so oats smell further on damp agar. Oats drain at `1 + wet_rot*(wet - base)` times their rate. A wound heals at a rate that scales with `wet`. Sap conductance scales with `0.5 + 0.5*wet`.
- **R4. Salt costs.** A walker pays `upkeep * (1 + salt_tax * max(0, salt - tolerance))`. This is research.md's "light as terrain" idea (its borrow #4), moved onto salt. Light stays a pure push, because the light-as-sensor-penalty version is a measured dead end (`dead-ends.md`, `light_weight`).
- **R5. Too dry, it crusts.** An active walker on a cell with `wet < crust_wet` crusts with a chance per step of `crust_rate * dryness * (1 - load/want)`, so dry, poorly fed walkers go first. A crusted walker does not move. The lamp's push and the snip skip it. It keeps depositing trail at `crust_trail` of the normal rate, **so a crusted tube stays a tube** (the light-weight dead end's lesson: "an empty stretch is a cut"). It pays `crust_upkeep` (for example a tenth of normal), keeps its load, and survives a storm strike with chance `crust_tough`. When `wet > revive_wet` it wakes, with a chance that rises with wet. A walker left crusted longer than `crust_life` seconds dies, so a long drought still kills, slowly.
- **R6. Hungry and lit, it fruits.** A walker whose load is below `fruit_hunger` and whose cell has `light > fruit_light` turns to Fruit: **the lamp's push on it reverses**, so it is stepped toward the lamp, not away. Where `fruit_gather` or more fruiting walkers sit within a small radius for `fruit_seconds`, they become a fruiting body: real cells on the dish (a new `Ground::Stalk`) that release spores. The number of spores rises with the food and the walkers it took. Spores drift on the wind, one cell a step. A spore that lands on agar with `wet > germ_wet`, `salt < germ_salt` and some scent becomes a walker with a small load. Everywhere else it fades.
- **R7. It learns salt.** A fed walker standing in salt gains tolerance at `learn * salt`, up to `tol_max`. Tolerance fades with a half-life of `tol_half_life`. Neighbouring walkers even out their tolerance by the same rule that already evens out food (`Params::share`), so a lesson spreads through the tube. It does not fade while crusted, and spores carry it.
- **R8. Weather sets the rates (events).** The **storm** keeps its one aimed cut (`STORM_CUTS`; see dead-ends) and adds **rain**: drops are real particles that fall and add to `wet` where they land, not a drawn overlay. It also adds a **wind**: one direction for the storm, which moves spores and pushes the scent spread by one cell downwind. A **dry spell** is a new event, warned like the storm, that multiplies `evap` for a few seconds.

**What creates most of the combinations: `wet` (39 of the 85 filled cells) and `salt` (23).** Hunger, which already exists as the walker's load, creates the state changes. Everything else is links the code already has.

**How this sits with the flow work (2026-11-09).** `flow` is off by default. `wet` works under either rule. Four cells need `flow` on: wet speeding food along the tube, dry tube starving its tips, walls kept under crust, and tolerance shared through `share`. With `flow` off, tolerance can be held per piece instead (the census already labels pieces). If the owner keeps `flow`, those four cells come for free. If he drops it, nothing else in this plan breaks.

---

## 2. The interaction matrix

`*` marks a new item. The codes are the shared field that makes each pair interact: **W** wet, **S** salt, **L** light, **F** food (scent, sap, load), **M** memory (wall, tolerance), **H** hunger and state, **P** spores, **K** strain, **C** cut ground. `ex` means it already happens today. A dot means nothing worth building. The table is the upper triangle, read as row x column. The pairs are spelled out below it.

| | 2 Oat | 3 Salt | 4 Snip | 5 Wat* | 6 Strm | 7 Dry* | 8 Wet* | 9 Slt~* | 10 Wall | 11 Sap | 12 Crst* | 13 Frt* | 14 Tol* | 15 Str* |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| **1 Lamp** | W | WS | WC | W | W | W | W | WS | M ex | F | WH | LH | . | LK |
| **2 Oats** | | S ex | CF ex | WF | WF | WF | WF | SF | M | F ex | WF | HF | MSF | FK |
| **3 Salt** | | | . | **WS** | WS | S | WS | S | MS | SF | WS | SP | SM | SK |
| **4 Snip** | | | | WC | C ex | WC | WC | . | MC ex | FC ex | H | FLH | . | CK |
| **5 Water** | | | | | W | W | W | WS | . | WF | **WH** | WP | . | . |
| **6 Storm** | | | | | | W | W | WS | MC ex | FC ex | **HC** | P | . | CK |
| **7 Dry spell** | | | | | | | W | . | . | WF | WH | H | M | . |
| **8 Wet agar** | | | | | | | | WS | . | WF | WH | WP | . | . |
| **9 Salt in agar** | | | | | | | | | . | SF | SW | SP | SM | . |
| **10 Wall** | | | | | | | | | | MF ex | MH | . | . | . |
| **11 Sap/load** | | | | | | | | | | | FH | FP | FM | FK |
| **12 Crust** | | | | | | | | | | | | H | M | . |
| **13 Fruit/spores** | | | | | | | | | | | | | MP | PK |
| **14 Tolerance** | | | | | | | | | | | | | | MK |

105 pairs: 85 have something (10 of them already happen today), 20 have nothing. The bold three are the headline discoveries (§4).

### The pairs, row by row

**1 Lamp**
- x Oats: the lamp dries the oats' patch, so they smell a shorter way and last longer (R1, R3).
- x Salt: a baked line stays sharp, and rain can't melt a wall the lamp has dried.
- x Snip: a cut under the lamp stays open longer, because wounds heal with wet.
- x Water: two hands on one field. Water a strip, then dry its edges with the lamp, and you get a damp road with hard sides that the mould runs fast along.
- x Storm: after the rain, the lamp dries back whatever you need hard again.
- x Dry spell: the lamp in a dry spell scorches, and mould crusts in seconds. Keep it away.
- x Wet agar: R1, light speeds evaporation.
- x Salt in agar: a dried halo stops bleeding.
- x Wall (already happens): a bent tube springs back to its wall when the lamp lets go. Hold it long enough and the old wall fades while a new one grows, so **the bend sets**, like bending wire. `wall_pull` and `wall_half_life` already decide how long that takes.
- x Sap: pushing a trunk thins it, and less food gets through.
- x Crust: the push skips crust, so the lamp sweeps the live mould off a crusted skeleton and leaves the skeleton behind.
- x Fruit: **hungry mould under the lamp stops fleeing and climbs in** (R6).
- x Strain: the lamp pushes both strains. Herd one into the other to force a meeting.

**2 Oats**
- x Salt (already happens): scent dies in salt, so a lure placed through a wall fails.
- x Snip (already happens): oats past a cut rescue the far piece.
- x Water: wet oats are a long-range lure that goes off quickly. Dry oats are a short lure that lasts.
- x Storm: rain makes every oat on the dish a long lure for a while. Wind drags its scent downwind.
- x Dry spell: oats smell only close by, so the mould must be led right up to them.
- x Salt in agar: an oat inside a salt halo is a weak lure.
- x Wall: an oat at the end of an old wall pulls the mould back along it fast.
- x Crust: **oats don't wake crust, water does.** Water plus oats wakes it and feeds it.
- x Fruit: feeding a fruiting crowd calls the fruiting off, because their load rises above hunger.
- x Tolerance: an oat just past thin salt trains the mould. Fed walkers standing in salt learn.
- x Strain: a contested oat draws both strains, and they meet there.

**3 Salt line**
- x Water: **water melts a line into a gap and a wide, weak halo** (R2).
- x Storm: the rain softens every salt wall on the dish, including the level's.
- x Dry spell: walls stay sharp.
- x Wet agar: salt moves only in water. Salt also dries its edges, so a line has a dry margin.
- x Wall: salt laid across an old wall makes a front that keeps pressing into the salt along the wall. You can see the frustrated bulge.
- x Sap: food that crosses salt is taxed.
- x Crust: mould hugging a salt line crusts first, because salt dries its edges.
- x Spores: **spores fly over salt.** They don't sprout on it.
- x Tolerance: salt is the thing tolerance learns.
- x Strain: a line keeps two strains apart until it melts.

**4 Snip**
- x Water: a wet cut heals fast. x Dry spell, x Wet agar: a dry cut stays open.
- x Storm (already happens): the storm is a big snip.
- x Wall, x Sap (already happen): a cut tube regrows along its wall, and the far side lives on what it carries.
- x Crust: crust shrugs off a snip, with chance `crust_tough`, the same knob as the storm.
- x Fruit: **cut a branch and light it, and it fruits instead of greying.**
- x Strain: cut the rival's trunk (out-there.md's *Tug*).

**5 Water**
- x Storm: drops and rain add up. Too much water bleeds every salt line on the dish, so flooding has a cost.
- x Dry spell: water is the answer, and its stock is limited.
- x Salt in agar: water dilutes a halo, so it spreads wider but weaker.
- x Sap: a wet tube carries food faster. Water a long tube to feed its tip (needs `flow`).
- x Crust: **water wakes crust.**
- x Spores: spores only sprout on damp agar, so water the place you want them to land.

**6 Storm**
- x Dry spell: a dry spell followed by a storm wakes all the crust at once, the biggest revival in the game.
- x Salt in agar: rain spreads every halo.
- x Wall, x Sap (already happen): regrowth runs along the walls round the scar, and the cut stops the food.
- x Crust: **a crusted tube survives the strike.** The scar is not laid on crusted cells, so the tube holds across the line, and the rain wakes it.
- x Spores: the wind carries spores far, downwind.
- x Strain: the storm aims at the cut that costs the most food, whoever owns it.

**7 Dry spell**
- x Sap: a dry tube carries food slower, so the tips starve first.
- x Crust: the thin, far, lean fingers crust first. The crusting is graded by wet and by load.
- x Fruit: a dry, hungry branch under light fruits fastest.
- x Tolerance (memory): the mould learns the rhythm of repeated spells, see §3 *Anticipation*.

**8 Wet agar, 9 Salt in agar.** These are the field rules themselves (R1 to R3), already listed under the verbs. Their own cells: salty agar is drier, so mould crusts there sooner (9 x 12). Spores don't sprout in salt (9 x 13). Salt is what tolerance learns (9 x 14).

**10 to 15, the mould's own states**
- Wall x Crust: a crusted tube's wall does not fade, so **revived mould comes back to the same network**. Dormancy keeps the shape.
- Sap x Crust: crust holds its load. A woken crust is a small food reserve.
- Sap x Fruit: more food in the fruiting walkers makes more spores.
- Sap x Tolerance: tolerance is learned only while fed.
- Sap x Strain: food never crosses between strains until they fuse.
- Crust x Fruit: the two states exclude each other. Dry pushes toward crust, lit and hungry toward fruit.
- Crust x Tolerance: tolerance survives dormancy (Boussard et al. 2019).
- Fruit x Tolerance: spores carry tolerance, so the next dish starts trained (the Wanderer's carry-over).
- Fruit x Strain: a spore keeps its strain.
- Tolerance x Strain: when strains fuse, they pool their tolerance through `share` (Vogel & Dussutour 2016: habituation passes on by fusion).

---

## 3. Mould responses with more than one dimension

Each response below is graded, never on/off. Each is driven by a continuous quantity the dish already holds, so the outcome is a spread across the network: part crusted, part fruiting, part still feeding.

| Response | Driven by | What it looks like, graded | Real basis (fun over fidelity) |
|---|---|---|---|
| **Pace** | `wet` | Damp fronts lunge and dry fronts crawl. Same rule, different speeds across one network. | Real plasmodia need moisture to stream. Plain physiology, not one paper. |
| **Crust (sclerotium)** | dryness x (1 - load) | Fingers go amber and still, from the dry, lean tips inward. Some survive the storm and some die past `crust_life`. Woken crust comes back on the same walls. | Slow drying makes a dormant sclerotium. Water restarts it within minutes. Learning survives dormancy (research.md, rows *Sclerotium* and *Boussard 2019*; very robust). |
| **Fruiting and spores** | hunger x light | A starving branch turns toward the lamp, gathers into a stalk and bursts. The spore count rises with the food spent. Where they land and sprout is set by wind, wet and salt. | Starvation plus light makes it fruit, and a fruiting plasmodium switches from avoiding light to seeking it (research.md *Fruiting*; robust in outline). |
| **Salt habituation** | salt x fed time | A front that bulges against weak salt, then crosses a little further each pass. The lesson spreads down the tube and fades if unused. | Repeated exposure teaches it to cross a repellent, and the aversion returns after days without (Boisseau et al. 2016). Real tests used quinine and caffeine; salt bridges were the setup. Moderate. |
| **Fusion and rejection** (last) | strain x contact | Same strain: two networks meet, and their trails and food join. Different strains: a contact line where walkers die slowly, leaving a brown seam, or fusion with a slow takeover. | Same-strain plasmodia fuse. Mismatched ones refuse, or fuse and one destroys the other within hours. Learning passes on by fusion after about 3 h (research.md *Fusion*). |
| **Anticipation** (stretch) | repeated dry spells | After three spells at a fixed interval, the outer fingers start crusting **just before the fourth is due**, even if it never comes. The habit fades and one spell brings it back. | Saigusa et al. 2008. Moderate, one group. |
| **Keeps its ground damp** (optional) | fed tube adds `tube_wet` to `wet` | A healthy network irrigates its own agar and resists drought. A starving one dries and crusts. Your network is also a water system. | Invented, but true in spirit: the plasmodium is mostly water. A knob, off by default. |

**Which earlier ideas get stronger once they interact.** Each of these was proposed alone in the 2026-11-08 brainstorm. Alone it is one more rule. Linked through `wet` and hunger, it gains a verb and a counter-verb.
- **Sclerotium** (research.md #6) changes from a nicer way to die into the storm defence, the dry-spell answer and the thing water does.
- **Fruiting** (research.md #8, and the probe brief's "starve-to-jump over salt") changes from an end-of-dish flourish into the only crossing of a wide salt sea. Its landing is steered by wind and water.
- **Anticipation** (research.md #3) needed something to show. Pre-emptive crusting is visible.
- **Light as terrain** (research.md #4) moves onto salt halos. The "cheapest path, not shortest" puzzle then comes from the player's own melted walls rather than fixed sunny patches, and the lamp stays a push.
- **Fusion and learning** (research.md #7) and **The Rival / Tug** (near-term-challenges.md #8, out-there.md #2) both gain salt lines that melt, spores that cross, and fusion that shares a lesson.
- **Storm Season** (near-term-challenges.md #5) gains a second answer, crusting, beside the bypass loop.
- **The Wanderer** (theme.md #1) gains a carry-over that is not just a walker count: spores and tolerance travel too.
- **Mould Organ** (out-there.md #3, `organ.rs`): once food in the tube is real (`sap`), water speeds the stream, so a watering can becomes a tempo control. That needs the organ's pulse to ride the sap rather than its snapshot geodesic, so it is a later step.
- **Two foods** (research.md #5) does not need any of this and is not in the plan. It sits beside it without conflict.

---

## 4. Three discovery moments

None of these is explained in advance. Each is a sequence a player falls into while doing something else.

### A. "The wall melted" (water x salt)

Behind the Wall, with the new water tool on the bar. The food is behind the level's salt line, with the gap at the bottom. The player drops water near home to speed the mould up, since the card's tool hint says "WATER: DAMP AGAR, THE MOULD MOVES FASTER". They drop a second drop nearer the wall, because the front is crawling. Over the next 3 s the line's white edge goes soft. The salt is spreading into the wet patch: the agar beside the line takes on a faint white grain, and the line itself thins where the drop touched it. The front reaches the wall where the cell's salt has fallen below `salt_wall` and pushes into the faint halo. It slows, and its tube goes pale yellow because the halo taxes upkeep. Then it crosses. The player has opened a gate the level did not draw. They also see the cost: the far side is fed by a lean tube through salt, and the halo is still widening while the agar is wet. On a later try they work out the refinement: one drop on the wall, then the lamp on the halo to dry it, which stops the bleed (lamp x salt x water). A short gate, not a wide one.

*What the screen showed, all from real cells:* the salt grain spreading (the salt field), the front slowing (pace and tax), the pale tube (load). *Counters:* `salt_opened` (cells that dropped below `salt_wall` this dish), `halo_cells` (cells with 0 < salt < wall).

### B. "Bake it before the storm" (lamp x dry x crust x storm x rain)

A free dish, 1:25. The purple warning line falls across the trunk to the far pile. The player has no oats left for a bypass. They do what everyone does at first: hold the lamp near the trunk to push it off the line. It bends, but not far enough. The lamp's light has also been drying the ground, and the walkers it pushed to the rim of the light sit in that dried ground, so they go amber and still. The player sees crust for the first time and thinks they have broken something. The storm strikes. The scar is laid everywhere along the line **except where the crust is**: the tube holds across the scar in a crusted stretch. Rain comes down, with drops landing as dark spots on the agar. The crust wakes over about 2 s, back on its own walls. Through the bridge the crust held, the far pile is still joined: the toughness star, with no bypass.

The second time, they bake the trunk on purpose. The third time, a dry spell has already crusted half the network when the storm arrives, and the rain wakes all of it at once. That is the biggest revival in the game, and nobody told them it would happen.

*Counters:* `crusted`, `crust_survived_storm`, `revived`, `crust_died`. *Check against the old rule:* the dead-end entry for `STORM_CUTS` says to re-test when "the mould heals across a scar". Crust holding across the scar is exactly that, so the toughness star's rate must be re-measured (§5, item 3).

### C. "It flew" (snip x lamp x hunger x fruit x wind x water)

A dish with a wide salt band, far too thick to melt with the water stock, and food beyond it. The player has been losing a long finger anyway, and it is going grey. Bored, they wave the lamp over the dying grey tip, expecting to push it about. Instead, **the grey walkers come toward the light**. The push has reversed, because they are hungry. They pile into the lamp's core. Held there for 2 s, the pile rises into a small orange stalk, which is real `Ground::Stalk` cells on the dish, and it bursts into a few dozen pale dots that drift. The wind is from the left, so they go right, over the salt. Most land on dry agar and fade. Three land on the damp patch the player had watered earlier for a different reason, and they sprout: new walkers, beyond the salt, next to the food's scent. A second mould starts on the far side.

The player has learned, without being told, that a starving branch is a resource, that the lamp means two things, and that water decides where spores land. On the next dish they water the landing site first, then cut and light a branch on purpose (snip x lamp).

*Counters:* `fruiting_bodies`, `spores_released`, `spores_sprouted`, and where each sprouted.

---

## 5. Challenge dishes that need a combination

Each follows level.rs's rule: doing nothing scores 0, a random hand at most 1, the intended hand 3. Bars are set from `mouldprobe level=N` with headroom, never guessed.

### 1. Salt Gate *(medium; water x salt x lamp)*

- **Goal:** "REACH THE FOOD INSIDE THE SALT RING." **Twist:** "WATER MELTS SALT."
- **Layout:** home on the left. One rich pile on the right, inside a closed salt ring with no gap. Tools: water x3, lamp, oats x3. No storm.
- **Doing nothing fails:** the ring is shut.
- **The obvious move fails:** flood the ring. It opens, but the halo spreads over the pile, taxes every walker that eats there, and the piece grows lean. The score is walkers fed inside the ring at the bell, so a flood gets 1 star.
- **Intended solve:** one drop on the ring nearest home, oats just inside the gap, and the lamp on the halo to stop it spreading. Stars on how few salted cells the feeding piece stands in, which is a graded number read from the salt field.
- **Why it's a combination:** water alone opens the gate badly. Water plus lamp opens it well.

### 2. Dry Season *(medium-hard; dry spell x lamp x water x crust x storm)*

- **Goal:** "KEEP THE MOULD AND ALL FOUR FOODS THROUGH THE DRY SEASON." **Twist:** "DRY MOULD SLEEPS. WATER WAKES IT."
- **Layout:** four piles in a ring. Dry spells at 0:30, 0:55 and 1:20, each warned 5 s ahead on the timeline the storm already uses. A storm at 1:40 brings rain. Water x2, regrowing one per 25 s.
- **Doing nothing fails:** the first spell crusts the far fingers, the second kills what stayed crusted past `crust_life`, and two piles are lost.
- **The obvious move fails:** spending water on every spell keeps everything awake but can't cover all four foods. The unwatered trunks starve while active, because dry tube carries food slowly.
- **Intended solve:** let the far trunks crust on purpose, which costs little and keeps their shape, and water the core. At 1:40 the storm's rain wakes the crust, and the network comes back on its old walls. Stars: walkers at the bell, and foods joined at the bell.
- **Why it's a combination:** crust looks like failure until the player sees the rain undo it. The dish teaches that the crust is a choice.
- **Stretch, anticipation:** the spells are evenly spaced, so a mould with anticipation on starts crusting the fourth spell's fingers before it comes. The player watches the mould prepare by itself.

### 3. Over the Salt Sea *(hard; two answers)*

- **Goal:** "GET MOULD ONTO THE ISLAND." **Twist:** "THE SEA IS TOO WIDE TO CROSS. OR IS IT?"
- **Layout:** the island holds food, ringed by a salt band about 24 cells wide, too much salt for the water stock to melt. A steady wind from home's side. Thin salt strips near home, with food behind them. Tools: lamp, oats, water x2, snip x2.
- **Answer one, spores:** water the island's shore, cut a branch, starve it under the lamp, and fruit it upwind. One star for any spore sprouting on the island, three for a piece there of 150 or more walkers at the bell.
- **Answer two, training:** lead the mould across the thin strips with oats, again and again, until its tolerance lets it push into the sea's edge. Then a single drop thins the sea enough for a trained front to cross where an untrained one could not.
- **Why it's a combination:** each answer needs three verbs, and the card says neither. A dish with two answers is where players start trading tips.

*Already designed and strengthened by this plan:* **Storm Season** (near-term-challenges.md #5) gains crusting as an alternative to the bypass. **The Rival** (#8) gains a melting salt fence, and spores give a way into the rival's half.

---

## 6. Build plan, ranked by depth gained per cost

Frame-cost reference, from the probe report: sim plus draw is 3.33 ms a frame under the old rule (sim 1.13) and 5.01 ms with `flow` (sim 2.57). The window runs 2 sim steps a frame. Each cost below is an **estimate to be measured** with `mouldprobe`'s frame timer: a 3x3 spread over the 41k-cell dish is the unit, and the trail and scent spreads already run two of them every step.

**Baseline rule for every item:** with the switch on but the new verb unused, every scripted hand on every dish scores exactly what it does today. `wet` starts at the baseline everywhere, and each factor in R3 is exactly 1.0 there, implemented as a skipped branch so no extra random draw shifts the sequence. A test plays `mouldprobe level=all seeds=1..5` and asserts identical counts with the switch on and off. This is what lets each item ship with its switch on (CLAUDE.md: a feature that measures neutral defaults on) without re-tuning the three dishes.

| # | Item | Depth / cost | Files | New field or state | Frame-cost risk | Knobs (tune.rs rows) | How we know it fired |
|---|---|---|---|---|---|---|---|
| 1 | **Wet + Water tool** (R1, R3) | Highest: 39 matrix cells from one field | `mod.rs` (Params, `wet` field, `wet_tick` in `step`, factors in `move_walkers`, `deposit_food`, `diffuse` scent keep, `heal_wounds`, `spread_sap`, `Tool::Water`); `look.rs` and `render.rs` (agar value from wet, icon, cursor preview disc); `tune.rs`; `level.rs` (`Tools::water`) | `wet: Vec<f32>`; `Tool::Water`; `water_left` | Low. One 5-point spread every 4 steps, about a quarter of one trail spread per step. Evaporation is a multiply on cells away from the baseline, held in a bounding box. | `wet` (switch), `wet_base`, `evap`, `dry_light`, `dry_salt`, `wet_spread`, `wet_pace`, `wet_scent`, `wet_rot`, `water_r`, `water_stock` | Counters: `water_drops`, `wet_cells` (> base + 0.2), `dried_cells` (< crust_wet). Picture: paired A/B, oats laid at equal distance on a wet and a dry side, with front arrival times printed. Debug overlay of `wet` on a fixed dark-to-bright ramp, as a full replace (CLAUDE.md rule). |
| 2 | **Crust** (R5) | Very high per line: storm defence, dry-spell answer, revival | `mod.rs` (`Walker::state`, skips in `move_walkers`, `push_light`, `kill_in` for snip and storm, `strike` leaves crusted cells unscarred, the economy functions); `look.rs` and `render.rs` (amber, never yellow or grey) | `Walker.state`, `crusted_at` | Negligible: a branch per walker | `crust` (switch), `crust_wet`, `crust_rate`, `revive_wet`, `crust_upkeep`, `crust_trail`, `crust_life`, `crust_tough` | Counters: `crusted`, `revived`, `crust_died`, `crust_survived_storm`. Picture: a filmstrip of a lamp-baked trunk through the storm, against an unbaked twin, with foods lost printed beside each. **Re-measure the toughness star** over 5 seeds x 4 hands: the `STORM_CUTS` dead end says to re-test once the mould heals across a scar, and this is that. |
| 3 | **Salt as a dissolving field** (R2, R4) | High: makes walls a puzzle piece | `mod.rs` (`Ground::Salt` becomes `salt[i]`; `sense`, step-entry check, `spread` scent fade, `stroke_dab`, economy tax); `level.rs` (level walls laid at 1.0, unchanged on dry agar); `look.rs` and `render.rs` (white by concentration) | `salt: Vec<f32>`; a bounding box of non-zero salt | Low. The salt spread runs only on cells with salt > 0 and wet > 0, inside its box. | `salt_wall`, `salt_bleed`, `salt_tax`, `salt_scent`, `salt_r` | Counters: `salt_opened`, `halo_cells`, `salt_tax_paid`. Picture: one salt line, dry and watered, at 0, 3 and 8 s. **Guard:** Behind the Wall with no water scores as today on 5 seeds. |
| 4 | **Weather: rain, wind, dry spell** (R8) | Medium-high: links the events to every field | `mod.rs` (`phase_tick`, `strike`; a schedule of events, which Storm Season needs anyway); `level.rs` (events per dish); `look.rs` (the timeline, which already exists for the storm) | rain-drop particles (`Vec<(x, y, vy)>`); `wind: (f32, f32)`; `evap_mult` | Low. A few hundred drops for a few seconds. Wind is one extra shifted read in the scent spread. | `storm_rain`, `rain_drops`, `wind`, `dry_spell_secs`, `dry_spell_mult` | Counters: `rain_landed`, `dry_spell_steps`. **Drawn-only flag:** rain streaks and wind arrows over the dish must be the real drop particles and real drifting spores, not a drawn overlay. A wind indicator in the HUD strip is chrome and fine. |
| 5 | **Fruiting and spores** (R6) | High for discovery, medium cost | `mod.rs` (state `Fruit`, push sign in `push_light`, stalk forming, spore list, sprouting); `wander.rs` (spores and tolerance carried to the next dish); `look.rs` and `render.rs` (stalk cells, spore dots) | `Ground::Stalk`; `spores: Vec<Spore>` | Low. Hundreds of particles, and stalk detection only among lit cells. | `fruit` (switch), `fruit_hunger`, `fruit_light`, `fruit_gather`, `fruit_seconds`, `spores_per_food`, `germ_wet`, `germ_salt`, `spore_life` | Counters: `fruiting_bodies`, `spores_released`, `spores_sprouted`. GIF of discovery C. **Lamp check:** the trunk-hold test that made the push lamp ship (5 of 5 held) must still hold on fed mould with fruiting on. Only hungry walkers flip. |
| 6 | **Salt tolerance** (R7) | Medium: a slow lesson the player builds up | `mod.rs` (`Walker::tolerance`, carried with `share` under `flow`, or per piece otherwise; read in the salt threshold and tax) | `Walker.tolerance` | Negligible | `learn`, `tol_max`, `tol_half_life` | Counters: `trained_walkers` (tol > 0.3), `salt_cells_crossed`. Picture: the same thin strip crossed on passes 1, 3 and 5, with how far into the salt each front got printed. |
| 7 | **Anticipation** (Saigusa) | Medium: a delight, but needs 2 and 4 | `mod.rs` (a per-dish learned interval and confidence) | 2 floats | None | `anticipate` (switch), `anticipate_learn`, `anticipate_fade` | Counter: `pre_crusts` (crusting in the 3 s before a due spell), and a positive control: skip the 4th spell and the counter still fires. |
| 8 | **Strains: fusion and rejection** | Highest cost; it unlocks The Rival | `mod.rs` (`Walker::strain`, a second trail field or a strain tag per cell, the census per strain); `play.rs` (the rival bot); `level.rs` | `trail_b` or a tag; per-strain stores | **Real.** One more trail spread per step, an estimated +0.3 to 0.5 ms a frame, measured against the 3.33 ms frame and the 17 ms worst frame | `strains`, `fuse_same`, `reject_harm`, `share_across` | Counters: `fusions`, `rejection_deaths`, `tolerance_passed`. Picture: two networks meeting, same strain and other strain, side by side. |

**One optional knob, `tube_wet`** (fed tube adds to `wet`). Two lines in `wet_tick`. Ship it off, because it changes item 1's balance, and let the owner turn it on in the panel.

**Discovery journal (UI chrome, cheap):** the first time each counter above fires in a save, the game names what just happened on a small card: "YOU FOUND: WATER MELTS SALT." The counters double as the measurement. The card names the event *after* it happened, so the discovery is the player's. It is stored next to `mould-progress.txt`.

---

## 7. Risks: where complexity hurts readability, and how to handle it

- **Colour budget.** Fed yellow and starving grey are the game's main readout (look.rs, theme.md), and no new state may use them. Proposed: `wet` darkens the agar (value, not hue); salt is a white grain whose density follows concentration; crust is amber-brown; a fruiting stalk is orange; spores are pale dots. Check every new colour on a phone-size still before building the next item.
- **The lamp means two things.** The push is the owner's first love and must not change for fed mould. A fruiting flip on mould that looks fed would read as a bug. Only walkers below `fruit_hunger` flip, and those are already drawn grey, so "grey mould comes to the light" is the readable rule. The lamp's cursor preview reads the real loads under its reach and says "HUNGRY MOULD WILL CLIMB IN" only when that is true. **Drying by the lamp must be slow enough that a 1 to 2 s herd never crusts anything.** Test: the existing trunk-hold and sweep scripts, with crust counted.
- **The lamp could become an eraser again.** Crusting under a long hold could look like the 2026-11-08 complaint ("an eraser, not a herding tool"). The guards: crust keeps laying trail, so the tube never parts (the dead end's mechanism); crust wakes once the patch rewets from the agar around it; and `dry_light` = 0 turns the lamp's drying off from the panel.
- **Too many causes for one effect.** A front can slow for four reasons: dry ground, salt tax, hunger, crowding. One held key, the field glass, should show the field under the cursor as a full-replace ramp (wet, then salt, then load), with the number printed. It is a debug readout made into a player tool. Labelled as a lens, it is chrome, not a drawn mechanic.
- **Discovery against instruction.** The owner needs clear options and a challenge. Each new tool gets the *first* meaning on its hint line ("WATER: DAMP AGAR, THE MOULD MOVES FASTER"). Each challenge card names the twist ("WATER MELTS SALT") only on the dish built around it. The second meanings are left to find, and the journal names them once found. Free dishes add one new switch at a time, gated by the dish picker's stars, so a player never meets five new systems at once.
- **Spread versus luck.** Spores and rain are random, and a 3-star solve must not depend on a lucky landing. Bars come from `mouldprobe` over at least 5 seeds per hand, quoting the minimum the intended hand scores, never the mean. The owner's rule wants a spread, not a slot machine.
- **Frame cost compounds.** Items 1 to 7 are each small, but together they could add the cost of a third spread per step. Measure after each item against a settled dish, where the dirty-rect skip and the light box work, and quote the worst frame, not only the mean.
- **The old drawn pulse.** With `flow` off (today's default), the Night look still draws a pulse from the feeding search's distances (`look.rs` `pulse`), which is the drawn-only effect the owner disliked. Nothing in this plan depends on it. Turning `flow` on, which draws the real sap, removes it.

---

## Sources and links

- Biology and models behind each response: `/mnt/project-files/game-design/mould-game-brainstorm-2026-11-08/research.md` (rows *Sclerotium*, *Fruiting*, *Salt ... habituation*, *Fusion*, *Anticipating periodic events*, *Light avoidance*), with the papers cited there: Boisseau, Vogel & Dussutour 2016; Vogel & Dussutour 2016; Boussard et al. 2019; Saigusa et al. 2008; Nakagaki et al. 2007; Tero et al. 2010; Jones 2010 and 2012.
- The parallel research file on Sage Jenson, Georgios Cherouvim and Jones 2010 (`/mnt/project-files/game-design/mould-depth-2026-11-09/sources-research.md`) did not exist when this was written (checked twice). If it adds steering knobs from those sources (sensor angle and offset as moods, deposit as appetite), they fit R3 as more factors driven by `wet`.
- Dead ends checked: `Reports/dead-ends.md`, entries `STORM_CUTS`, `deposit_food` and `Params::light_weight`. All three are respected above, and the first is flagged for re-measurement in build item 2.
