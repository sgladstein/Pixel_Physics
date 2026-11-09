# Slime mould: the real organism, existing models, existing games (research, 2026-11-08)

Research brief for the mould brainstorm. It covers what real *Physarum polycephalum* does, what other people's models capture that ours does not, which games have already tried this ground, and what we should borrow first.

**How to read the marks.** **[V]** means I saw it in a source: a search result quoting the paper, an abstract, or a publisher or repository record. **[I]** means I am inferring it, usually about our code or about a game. **[M]** means it comes from my own background knowledge and I could not check it in this session.

**A caveat on sources.** This session's network blocks page fetches. pmc.ncbi.nlm.nih.gov, arxiv.org, eprints.lib.hokudai.ac.jp, pdodds.w3.uvm.edu and itch.io all failed to resolve. So every **[V]** below comes from search-engine summaries of the paper, or from SciSpace abstracts, not from my reading the full text. Numbers that matter are flagged where only one summary gave them.

**Our model, for comparison** (`src/mould/mod.rs`, read directly). It is Jones (2010): 600 to 1,800 walkers on a 256x160 dish, one per cell. Each has three sensors 9 cells ahead at ±45°, turns 45°, steps one cell, and lays 5 units of trail. The trail spreads by a 3x3 mean and keeps 90% per step. Food scent is a second field that keeps 97% per step. A cell counts as "tube" when its trail is above 1.0. The game adds an economy: a breadth-first search from eaten piles along tube cells, out to 70 cells. Walkers beyond that are unfed and starve slowly. Each pile feeds at most 40 eaters. The lamp *pushes* trail and walkers outward. The sensor-repulsion lamp was tried and reverted: it cut tubes on 5 of 5 seeds (`Reports/dead-ends.md`, the `light_weight` entry). The probe report already lists "the pulse beat, spores and fruiting, dye riding the tubes, a rival mould" as left-out next steps (`Reports/mould-probe-2026-11-07.md`, *Left out*).

One structural fact shapes everything below. **In our model a tube is just trail, and trail is forgotten in about a second.** The real organism's tubes have walls that thicken over minutes and last for hours. Most of what is missing comes from that gap.

---

## What we should borrow, ranked

Ranked by what it adds for the player against what it costs us.

**1. Tubes that remember: a slow "wall" under the fast trail.** Real tubes that carry flow grow thicker. Tubes that don't shrink and vanish, with a 1 to 3 minute delay, and sometimes whole clusters vanish at once [V, Marbach et al. 2023]. Tero's model is exactly this rule: a tube's capacity grows with the flow through it and decays otherwise [V, Tero 2007, 2010]. Tubes also store a memory of where food was [V, Kramar & Alim 2021].
*For the player:* the network they built *stays built*. Trunks visibly fatten over the dish, so there is a graded middle between thin new tube and fat old trunk. A storm cut leaves a ghost wall that the mould regrows along first. "I made that bridge and it's still there" is the caretaker feeling.
*In our code [I]:* add one per-cell field, `wall`, with no spreading. Each feeding pass (every 6 steps) walks the BFS parents back from every fed walker to its pile and adds to `wall` on each cell it passes. That is a flux count, roughly 1,800 × 70 = 126k additions per pass. `wall` decays slowly, and sensors read trail + k·wall. Draw tube width from `wall`. Cost: one more 41k-float field and a back-walk. **Moderate.** It touches the tube definition, so the `light_weight` and `fed_scent` dead-ends must be re-tested, because both say *re-test when tube stops being a trail threshold*.

**2. A visible heartbeat: shuttle streaming.** Real cytoplasm surges back and forth through the tubes. The period is about 100 s (90 s to 3 min). Branches don't all reverse at once: the reversal travels across the network as a wave whose length is about the size of the organism [V, Alim et al. 2013]. A food stimulus sends a front of stronger contraction out along the tubes [V, Alim et al. 2017].
*For the player:* the mould looks alive even when it stands still. Food is seen *arriving*: a bright pulse rides out from each pile, and a cut tube's pulse visibly stops at the scar. It is the "dye riding the tubes" idea, but grounded in the real organism.
*In our code [I]:* render-only first. Brightness = sin(2π(t/T − d/λ)), where d is the BFS distance we already compute and T is about 2 to 3 s at play speed. **Easy, about one render function, no sim cost.** A gameplay version, where food reaches the far end only on the beat, is **moderate**.

**3. The mould learns the rhythm of the weather (Saigusa 2008).** Plasmodia given three cold, dry pulses at a fixed interval slowed down by themselves when the fourth pulse was due, even though it never came. After the habit faded, a single pulse brought it back [V, Saigusa et al. 2008].
*For the player:* a caretaker story with a payoff. You protect it through the first storms, and later it *flinches before the storm* and pulls in its fingers. Watching it anticipate is a reward you earned. It also means a dish can be planned around a rhythm.
*In our code [I]:* a per-dish learned phase. Each storm or cold snap at interval τ raises a confidence. When confidence is high, the mould cuts division and pulls walkers inward (raise `crowd`, lower `divide`) just before the predicted time. Pershin & Di Ventra (2009) give a two-variable circuit model for it [V]. **Easy to moderate.**

**4. Light as terrain: the mould pays to avoid it.** Lit ground isn't a wall. The mould takes a *longer* path to keep its exposure low, and the tube kinks where it enters the light [V, Nakagaki et al. 2007]. Tero et al. (2010) used light to stand in for mountains and lakes on the Tokyo map [V].
*For the player:* sunny patches on the dish make the "shortest network" puzzle a *cheapest* network puzzle, with a graded answer. A trunk across sun costs upkeep, and a detour costs length. Moving a shade-board changes the best answer.
*In our code [I]:* put light into the *economy*, not the sensors, because the sensor way is a measured dead-end. A walker in a sunlit cell pays k× upkeep, so a sunlit tube starves first and the mould drifts round it. **Easy:** one multiply in `economy`.

**5. Two foods, one diet (Dussutour et al. 2010).** One strain prefers a 2:1 protein-to-carbohydrate diet. Given eleven foods, none of them at that ratio, it spread itself across several to reach the mix [V].
*For the player:* "join everything" becomes "join the right *pair*". Oats (sugar) and something meaty (protein) each feed the mould only in balance with the other. That gives a reason to build a particular bridge and to leave a near pile unjoined.
*In our code [I]:* piles get a type, and the store becomes two stores. Upkeep draws from both, and growth is capped by the scarcer one. **Easy.**

**6. Drying out instead of dying (sclerotium) and coming back.** Slowly dried plasmodium turns into a hard, dormant crust, the sclerotium. Add water and it starts moving again within minutes, and is growing within about a day [V, lab protocols]. A sclerotium can be revived after up to about 3 years [V]. Habituation learned before drying is still there after a month of dormancy [V, Boussard et al. 2019].
*For the player:* a starved region doesn't vanish. It goes amber and crusty and can be *rescued* by a drop of water or a new tube. That is the owner's graded death made into something the player can act on, and it can carry the mould from dish to dish.
*In our code [I]:* unfed walkers become crust cells, a new `Ground` kind that holds stored food, instead of dying at `starve`. Re-feeding through a tube, or a water tool, turns them back into walkers. **Moderate.**

**7. Fusion, and learning that passes on.** Two plasmodia of the same strain fuse into one when they meet [V, Collins & Haskins, and others]. Different strains refuse, or fuse and then one destroys the other within about 6 hours [V, post-fusion incompatibility literature]. A mould that has learned to cross salt passes that on to a naive one by fusing for about 3 hours, but not after 1 hour [V, Vogel & Dussutour 2016].
*For the player:* the satisfying moment of two networks joining into one, and a rival mould you must keep apart or let fight. "Fuse the trained one into the new dish" is a verb.
*In our code [I]:* walkers get a strain id. Same-strain trails add up, and different-strain trail is read as repellent. A rival needs its own store. **Moderate.**

**8. Fruiting: starvation flips light from enemy to friend.** Starved, mature plasmodia stop avoiding light, move toward it, and form fruiting bodies that release spores [V, teaching and lab sources; older literature]. Light speeds this up, about 2 days against 5 at 25 °C [V, single source].
*For the player:* an end-of-dish payoff. The mould climbs into your lamp and fruits, and the spores seed the next dish. The lamp's meaning changes at the end of the dish. This is the probe brief's "starve-to-jump over salt".
*In our code [I]:* a phase flag. When the store falls below a threshold at the end of a dish, the push sign flips and walkers gather to the lamp, then turn into spore particles. **Moderate.**

Considered and ranked lower. *External slime memory* (Reid et al. 2012): the mould avoids ground it has already slimed when there is fresh ground available. That would push our fronts outward, but it overlaps with #1 and risks fighting the trail attraction. *Habituation to salt on its own* (Boisseau et al. 2016): a nice detail, best carried by #7.

---

## 1. The real organism, behaviour by behaviour

Real time-scales are hours to days [V, Marwan 2010, a *Science* commentary: self-optimisation "on a time scale of hours to a few days"]. Our 90 s dish compresses that by roughly 1,000× [I].

| Behaviour | What it looks like | Source | How robust |
|---|---|---|---|
| **Shuttle streaming** | Cytoplasm flows one way through the tubes, slows, and reverses, about every 100 s (90 s to 3 min). Tube walls squeeze in a wave whose phase rises steadily along the tube, so it is peristalsis, not one big pump. The wavelength matches the organism's size across two orders of magnitude, while the period barely changes with size. | Alim, Amselem, Peaudecerf, Brenner & Pringle 2013, *PNAS* 110:13306 ([Harvard group page](https://brennergroup.seas.harvard.edu/publications/random-network-peristalsis-physarum-polycephalum-organizes-fluid-flows), [PDF](https://www.bpm.ph.tum.de/wp-content/uploads/2023/07/pnas.1305049110.pdf)) [V]. The 90 s figure is from a 1977 paper and the 1 to 3 min range from a review, both via search summaries [V]. | **Very robust.** Known for decades and measured by many groups. |
| **Signal riding the flow** | A food stimulus makes a front of stronger contraction that travels at about the speed of the flow. The authors say this explains shortest-path finding. | Alim, Andrew, Pringle & Brenner 2017, *PNAS* 114:5136 ([page](https://brennergroup.seas.harvard.edu/publications/mechanism-signal-propagation-physarum-polycephalum)) [V]. The molecule was not identified [V]. | Good. One group, but a model plus experiment. |
| **Thickening and pruning** | It spreads widely, then thickens what carries flow and drops the rest. Wall shear feeds back on tube radius with a **1 to 3 min delay**. Veins next to large ones tend to vanish, and whole clusters can vanish together in "avalanches". Pruning raises how well the network mixes its contents. | Tero et al. 2010; Marbach, Ziethen, Bastin, Bäuerle & Alim 2023, *eLife* 12:e78100 ([eLife](https://elifesciences.org/articles/78100)) [V]; Marbach et al. 2016, *PRL* 117:178103 ([PDF](https://www.bpm.ph.tum.de/wp-content/uploads/2023/07/2016-PRL-Pruning-Increases-Taylor-Dispersion-Marbach.pdf)) [V] | **Robust** qualitatively. The delay figure comes from one study. |
| **Memory in tube widths** | Meeting food makes nearby tubes dilate and neighbours shrink, so the width pattern stores where the food was and later steers migration. Press coverage says a released chemical softens the tube walls. | Kramar & Alim 2021, *PNAS* 118:e2007815118 ([PDF](https://www.bpm.ph.tum.de/wp-content/uploads/2023/07/2021-PNAS-Memory-Hierarchy-Alim-Kramar.pdf)) [V] | Good. One group, microscopy plus model. |
| **Maze solving** | Spread through an agar maze with food at two exits, it pulls back to one thick tube along the shortest of four routes. | Nakagaki, Yamada & Tóth 2000, *Nature* 407:470 ([Nature](https://www.nature.com/articles/35035159)) [V] | **Very robust.** Widely repeated; there are school-kit versions of it. |
| **Tokyo rail** | Oats at the positions of Greater Tokyo cities, with light standing in for mountains and lakes. The mould's network matched the real rail system on cost, efficiency and fault tolerance. The paper's tube model reproduced it, and tuning that model trades resilience against cost. | Tero, Takagi, Saigusa, Ito, Bebber, Fricker, Yumiki, Kobayashi & Nakagaki 2010, *Science* 327:439 ([PubMed](https://pubmed.ncbi.nlm.nih.gov/20093467/), [SciSpace](https://scispace.com/papers/rules-for-biologically-inspired-adaptive-network-design-5fajyacscy)) [V] | Robust qualitatively, with about 1,000 citations. Becker (2011) notes that **real moulds' solutions "show big variations"** from run to run [V]: a spread of outcomes, not one answer. |
| **Light avoidance and minimum-risk paths** | It avoids white, UV and blue light. With part of the field lit, the tube took a longer path that kept its time in the light short, matching a computed minimum-risk path. | Nakagaki et al. 2007, *PRL* 99:068104 [V]; reviews [V] | Robust for avoidance. One University of Idaho thesis saw it **habituate** to light over an experiment [V], so the effect weakens with time. |
| **Speed against accuracy** | Light and hunger change how fast and how well it chooses between foods. It also makes "irrational" choices that shift when a decoy option is added. | Latty & Beekman 2011a, 2011b, *Proc B* 278 [V]. The direction of the light effect could not be confirmed. | One lab, replicated within it. |
| **Anticipating periodic events** | Three cold, dry pulses at a fixed interval slowed it each time. Afterwards it slowed by itself when the next pulse was due. Once the habit faded, a single pulse brought it back ("recall"). One summary says this worked at a 60 min interval but not 40 or 80 [V, single summary]; [M] I recall the paper reporting a range of about 30 to 90 min, so treat the interval as uncertain. | Saigusa, Tero, Nakagaki & Kuramoto 2008, *PRL* 100:018101 [V]; Nature News, Ball 2008 ([Nature](https://www.nature.com/articles/451385a)) [V] | **Moderate.** One group; not every organism showed it [M]. |
| **External slime memory** | It leaves non-living slime behind and, given a choice, avoids slimed ground. With the ground pre-slimed, it escaped a U-trap about a third as often and spent about 10× longer re-exploring. When nothing unexplored is left, it stops avoiding slime. | Reid, Latty, Dussutour & Beekman 2012, *PNAS* doi:10.1073/pnas.1215037109 [V] | Good. One study, widely discussed. |
| **Salt, quinine, caffeine and habituation** | Repellents slow it or turn it away. With repeated exposure it learns to cross: it habituates to quinine and caffeine separately, and the aversion returns after 2 days without them. The salt work used salt-coated bridges. | Boisseau, Vogel & Dussutour 2016, *Proc B* doi:10.1098/rspb.2016.0446 [V]; Vogel & Dussutour 2016, doi:10.1098/rspb.2016.2382 [V] | Moderate. One lab, and some debate over whether it is true learning. |
| **Fusion** | Same-strain plasmodia that meet merge into one. Fusion needs matching *fus* alleles at several loci, about four in one strain. Mismatched strains don't fuse, or fuse and then one destroys the other within about 6 h. Habituation transfers by fusion after 3 h but not 1 h. | Collins & Haskins genetics, Schrauwen thesis, Lane & Carlile 1979 [V via summaries]; Vogel & Dussutour 2016 [V] | Fusion genetics: **robust** (1960s to 70s classical genetics). Transfer of learning: one lab. |
| **Sclerotium (dormancy) and revival** | Dried slowly over several days, it forms a hard dormant body. With water, the actin chemistry restarts within 5 min, and it is growing again within about 24 h. It can be revived after up to about 3 years, with viability falling after 1. Habituation survives a month of dormancy. | Lab protocols (Ward's), an actin study (*J Euk Microbiol* 2002) [V]; Boussard et al. 2019 [V] | **Very robust.** It is how labs keep the organism. |
| **Fruiting and spores** | Starvation plus light makes it fruit. Light shortens fruiting from about 5 days to 2 at 25 °C. When ready to fruit it switches from avoiding light to moving toward it. Spores are 5 to 20 µm and last for years. | Teaching and lab sources [V]; older phototaxis literature [V via summary] | Robust in outline; the numbers come from single sources. |
| **Food balance** | One strain prefers 2:1 protein to carbohydrate and spreads across several foods to reach that mix. It spreads wider on dilute food. | Dussutour, Latty, Beekman & Simpson 2010, *PNAS* 107:4607 [V] | Good. One lab, a strong design. |
| **Speed** | A few cm per hour, up to about 5 cm/h. Speed scales with the thickness of the front, and fragments move faster when the contraction wave runs forward along them. It explores in self-avoiding runs and tumbles. | Reviews citing Kessler 1982 [V]; *J Exp Biol* 2015 allometry [V]; 2024 arXiv self-avoidance [V] | Robust as a range. |

---

## 2. Existing models and simulations

**Jones agent model (what we run).** Jones 2010, *Artificial Life* 16:127, 147 citations ([SciSpace](https://scispace.com/papers/characteristics-of-pattern-formation-and-evolution-in-3sr6e5wv1h)) [V]. Particles with three forward sensors on a diffusing trail show "tension-like effects, network minimisation, and repair to network damage" [V, abstract]. Related papers by Jones:
- Jones 2009 [V]: the first version.
- Jones 2011, *Natural Computing* [V]: how sensor scaling changes the networks.
- Jones 2012, amoeboid movement ([arXiv 1212.0023](https://arxiv.org/pdf/1212.0023)) [V]: the same particles produce **oscillations, phase shifts and shuttle streaming**. They are coupled only through the shared field, and the paper calls coupling between "membrane" contraction and streaming central.
- Jones 2015, *Parallel Processing Letters* [V]: a catalogue of ways to steer the model, including stimulus distance, angle and concentration, engulfment and consumption of food.
- Jones, Mayne & Adamatzky 2017 [V]: the model represents the *shape* of a point set, not just its connections.

Typical settings in citing work are sensor angle 22.5° to 45°, sensor offset 5 to 9 and rotation 45°; 30° is used to encourage branching [V]. Ours is 45/45/9.

Extensions:
- Lyubka 2026, *Applied Soft Computing* [V]: adds **repellent-driven behaviour** to Jones, with up to 65% fewer iterations on Steiner-tree problems. It is directly relevant to salt and light, and worth reading before redesigning either.
- Wu, Zhang & Deng 2012 [V]: adds birth and death of agents, which we already have in our economy.
- Becker 2011 [V]: **agent models reproduce the run-to-run variance of real moulds much better than the tube model.** This argues for keeping the agents and grafting on, not replacing them [I].

*Captures that ours lacks:* oscillation and streaming (Jones 2012). *Cost:* the same as ours. *Bolt-on:* yes.

**Tero flow-conductance ("current reinforcement").** Tero, Kobayashi & Nakagaki 2007, *J Theor Biol* 244:553 ([record](https://ir.lib.hiroshima-u.ac.jp/00020647?locale=en)) [V]; Tero et al. 2010 [V]. Tubes are graph edges with a conductance D. Food nodes inject and withdraw a fixed flux, pressures come from Kirchhoff's law (a linear solve), and the flux Q = D/L·Δp. Each step, dD/dt = f(|Q|) − D: busy tubes grow and idle ones fade [V, the form from reviews and implementations]. [M] In Tero 2010 the source and sink are re-chosen among food nodes each step, and f is sigmoidal with an exponent that tunes cost against fault tolerance. I could not confirm this here. Follow-ups:
- Watanabe & Tero 2011 [V]: rail traffic.
- Almeida & Dilão 2023 [V]: fixes some "inconsistencies" in the model and solves a maze and a map of mainland Portugal.

*Captures that ours lacks:* global flow, so a tube carrying food from *far* away thickens; principled pruning; tuned redundancy, so loops survive. *Cost:* a sparse linear solve per step. [I] On our 41k-cell grid that is too much every step, but fine on a coarse graph of tube junctions every second or so, or approximated by the BFS-parent flux count proposed in #1. *Bolt-on:* yes, as a slow layer under the agents. That hybrid is what borrow #1 is.

**The Alim group's physics models** (2013 to 2023, above). These are continuum fluid and elastic-wall models of real networks. *Captures:* the real peristalsis, signals riding flow, delayed vein fate, memory in widths. *Cost:* high, solving flow on the network every step [I]. *Bolt-on:* only as approximations. Their *rules* are portable: phase rising along the tube, a 1 to 3 min shear delay, softening carried by flow.

**Cellular automata and reaction-diffusion.**
- Adamatzky (*Physarum Machines*, many papers) [V]: an Oregonator (a two-variable Belousov–Zhabotinsky model), whose "target waves" and travelling blobs imitate plasmodium fronts, and a three-state "retained excitation" CA that makes amoeba-shaped patterns.
- Gunji, Shirakawa, Niizato & Haruna 2008, *J Theor Biol* 253:659 [V]: a minimal cell model that solves mazes and Steiner trees. It needs the cytoplasm to *harden* after material passes, or the cell falls apart.
- Cellular Potts (Semmler 2011) [V].

*Captures:* a real membrane and body edge (Gunji), and waves (Oregonator). *Cost:* cheap per cell, but it covers every cell. *Bolt-on:* an Oregonator layer could drive the pulse visual [I], but it is heavier than a sine on BFS distance and adds little that players would see.

**Anticipation models.** Saigusa et al. 2008 had a coupled-oscillator model [V]. Pershin, La Fontaine & Di Ventra 2009, *PRE* 80:021926 ([arXiv](https://arxiv.org/pdf/0810.4179)) [V] built an LC circuit plus a memristor that learns when pulses arrive near its resonance. A tiny model, easy to put in a game.

**GPU and art versions.**
- Sage Jenson's "physarum" (2018) [V]: 5 to 10 million particles in real time on a GTX 1070, everything on the GPU. It made the look famous in creative coding, and CPU ports top out around 100k particles [V].
- Bleuje's *interactive-physarum* ([GitHub](https://github.com/Bleuje/interactive-physarum)) [V]: an installation with gamepad control and a "**create wave (speed + sensing intensity)**" button. That button is a player verb that briefly agitates the whole mould, and worth copying [I].
- Amanda Ghassaei's gpu-io Physarum ([app](https://apps.amandaghassaei.com/gpu-io/examples/physarum/)) [V]: paint attractant to steer, like our oats.
- Michael Fogleman's renders [V]; the NetLogo SlimeMoldNetwork model [V].
- Elek et al.'s **Monte Carlo Physarum Machine** ([arXiv 2204.01256](https://arxiv.org/abs/2204.01256v1); Polyphorm, [arXiv 2009.02441](https://arxiv.org/pdf/2009.02441)) [V]: probabilistic sensing and 3D, used to rebuild the cosmic web from about 37,000 galaxies.

*Lesson [I]:* visually, art versions get their beauty from particle count and colour ramps, not new rules. We have about 1,800 walkers, one per cell, so our look is set by drawing, not by numbers.

**Models of streaming or nutrient transport, in short.** Jones 2012 (agents, cheap); the Alim models (physics, costly); Tero (flow, moderate). None of them is a game-ready "food packets ride the pulse" model. That is ours to invent, and #2 is the cheapest version.

---

## 3. Existing games and toys

| Game | What it does | Reception | What it leaves open |
|---|---|---|---|
| **Mushroom 11** (Untame, 2015) | You move a fungal blob by *erasing* cells, and it regrows on the far side. Physics puzzles and platforming. | OpenCritic 82, 67% recommend; IGN 9, PC Gamer 85, GameSpot 6 ([OpenCritic](https://opencritic.com/game/1955/mushroom-11), [Wikipedia](https://en.wikipedia.org/wiki/Mushroom_11)) [V]. Cubed3: the "random movements the mushroom tends to make really detract from the fun" [V]. | **The closest relative of our lamp**, and the warning is real: when the organism's randomness fights the hand, it frustrates. No networks and no feeding. |
| **PHYSARUM: Slime Mold Simulator** (Ka Hin Yuen, Steam 2021) | A Jones-style sandbox: tweak and mutate parameters, watch it grow. | About 15 to 19 reviews, about 89% positive ([Steam](https://store.steampowered.com/app/1667120/PHYSARUM_Slime_Mold_Simulator/)) [V] | **No goal.** That is exactly the owner's complaint about our probe. A real Physarum sim with a challenge is open ground. |
| **WETWARE** (Dan Emmerson, itch, in development) | "Grow an organic network to connect nodes", inspired by slime moulds. Per one summary, you drag the network out and retract it to get nutrients back, and retracted ground leaves residue you can't build on [V summary; page unreachable]. | Unknown | A hand-drawn network, not emergent: the player *is* the mould. We have the opposite, a mould you persuade. |
| **Slime Vs Mold** (itch, PICO-8) | Play as a slime mould fighting a rival mould over bathroom tiles; more slime on a tile, faster growth. | Unknown [V exists] | Proof that a rival mould is a natural game. Ours could be emergent rather than tile-based. |
| **Slime Gardener**, **Slime Network**, **Physarium**, **Plasmodium** (game jams) | Place and remove food for slimes; collect food while avoiding gnats; hex resource growth. | Jam entries [V exist; mechanics only from summaries] | Small and unfinished; nothing combines emergent tubes with an economy. |
| **Creeper World** series (Knuckle Cracker) | An enemy fluid, "creeper", floods the map. Your energy *packets* visibly travel along a relay network from the nearest node, use relays when that is shorter, and nodes share one pool. Creeper IXE adds falling-sand style fluids. | A long-running, well-loved series [V for the mechanics, from the dev blog and wiki; reception M] | **Visible packets on a network are our "food through tubes" made legible.** It shows that players enjoy watching supply flow. It is a hostile spreading fluid, not a creature you care for. |
| **Mini Metro** (Dinosaur Polo Club, 2015) | Draw lines between stations as the city grows; constant redesign. | GDC Choice "Best Debut" finalist; "tense but meditative" [V] | The difficulty curve to copy: demand grows, you never finish, and you keep re-routing [I]. Lines are drawn directly, not grown. |
| **Osmos** (Hemisphere, 2009) | Move by ejecting mass, which shrinks you; absorb smaller motes. | Metacritic roughly upper-70s/80s; Destructoid positive [V] | A verb whose cost is your own body: an analogue for "the mould spends itself to reach" [I]. |
| **SimAnt** (Maxis, Will Wright, 1991) | Your yellow ant lays pheromone trails that the colony follows; rain washes trails away. | A classic [M] | Steering a crowd by trail is our oats. Weather erasing trails is our storm. Already proven fun 35 years ago. |
| **Mycelarium** (Steam) | An incremental game: grow an underground fungal network. | About 511 reviews, 86% positive; one review says it slows and "loses cohesion" around 2 h and lags with many spore mushrooms [V summary] | Theme pulls players in, and depth runs out. Fungal networks have a market [I]. |
| **Mycelium Conquest** (Steam) | Fungus automation on an alien planet. | Small; 75 to 81% of 12 to 16 reviews; complaints of weak resource feedback [V summary] | Same lesson: feedback on resources must be legible. |

**The gap, as far as these searches can see [I].** I found no game that uses a real emergent Physarum simulation as its core mechanic *with a goal*. The sims have no goal (PHYSARUM), and the network games draw their lines by hand (Mini Metro, WETWARE). Mushroom 11 is the nearest in feel, and its main criticism is ours to avoid: an organism that ignores the hand. Absence from search is not proof of absence.

---

## Notes for whoever builds next

- **The biggest borrowable truth is the time-scale split [I].** Real moulds run fast streaming (about 100 s), slower vein adaptation (minutes, with a 1 to 3 min delay) and still slower memory (hours to a month). Our model has only the fast layer. Borrow #1 adds the middle one, and #3 and #6 add the slow one. Each layer is something the player can *see accumulate*.
- **Dead-ends that bite these ideas.** Light in the sensors cut tubes (`light_weight`), so put light in upkeep. Scent in the trail channel drew false tube (`fed_scent`). Both carry "re-test when tube stops being a trail threshold", which borrow #1 triggers.
- **Real outcomes vary a lot** (Becker 2011; Tero 2010 networks differ by run). That supports the owner's "outcome is a distribution" law, and suggests scoring dishes as a spread over seeds rather than one star line.

Sources not reachable in this session: pmc.ncbi.nlm.nih.gov, arxiv.org, eprints.lib.hokudai.ac.jp, pdodds.w3.uvm.edu, itch.io. Allowing them in the environment's network settings would let a later pass check the numbers above against full text. Consensus search hit its monthly limit. SciSpace and web search worked.
