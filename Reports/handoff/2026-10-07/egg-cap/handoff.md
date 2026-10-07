# What limits egg laying on the new stack, and the food cap (Laying, 2026-10-07, paused 22:50)

**Status: trace part-done, paused at Scott's request (22:41) so usage lasts to the handoff.**
- No proposal has been written or sent. Nothing is built.
- One test ran to completion: the stack at heap 90, seeds 1-4, 200k frames. Everything below that says "measured"
  comes from it.

The coordinator's ask (22:18):
1. Trace what actually limits laying on the stack, and why `LAY_BAR=body` doesn't bite.
2. Then write a food-based birth cap, citing biology.
3. Send it to Nest race to review before building.
4. Build on Nest race's store-crumb fix (`NEST_STORE` part `edible`) once it lands.

## The run

**Build.** `claude/nest-race-way-foot` at b59228529. That is 74a2f08a plus Nest race's `edible` part, which is off
and was not used here. On top of it, a read-only laying census was added to deeptrace's `colony.csv`:
`tools/census.patch`, against `examples/deeptrace.rs`.

**Env.**
`NEEDS_FIRST=on,backfill CARRY_HOME=on DOOR_COLUMN=on LAY_BAR=body NEST_STORE=on,pick=20,jaws,sky,meal,smell=10 WAY_FOOT=on MUTATION=off`

**deeptrace args.**
`scenario=nest_goal founder=evolved ants=0 dig=1 foodgap=90 mapevery=5000 nestevery=5000`, with
RAYON_NUM_THREADS=1. The script is `tools/run.sh`.

**Positive control.** Seed 1's counters at 20-50k match Nest race's base (t90on) exactly: 836 eggs laid against
587,857 ticks held for want of a site. So the census did not change the run.

**What the census records.** For every live ant, every 1k frames:
- its egg bar and energy;
- `creature::home_ring` at the egg pile's reach of 4;
- what fills the 9x9 square around its head: brood, food, other ants, and empty dug-home cells;
- whether the pile walk would find a site if it could pass through brood.

Raw runs are in the Laying container only (`scratchpad/laycap/runs/base-s1..4`, about 0.5 GB each) and will be lost
with it. The tables here are complete.

**Files in this folder:**
- `laycensus.txt`, `eggwhere.txt`, `larvae.txt`: every window, every seed;
- `pictures/nest-over-time.png`: seeds 1-4 at 25, 50, 100, 150 and 200k;
- `tools/`: the three readers, the run script and the census patch.

## Measured (4 of 4 seeds, every window from 20k to 200k unless said)

### 1. The body bar is cleared by about a quarter of the colony, so it is not what limits laying

**How many ants clear the bar.**
- The bar is 1,034 J (the evolved founder's `lay_at`).
- From 50k to 200k, 19-27% of adults are at or above it at any moment: 117-175 ants.
- At 20-50k it is 12-23%: 41-101 ants.
- Their median energy is 1,420-1,690 J.

**Every egg is already paid from the layer's own body.**
- The layer's median energy after the egg is 1,440-1,670 J.
- Layers left under 200 J: 0.0-0.2% of eggs.
- So `LAY_BAR=body` works as designed: it stopped hungry ants laying from food in reach. It does not bite because
  rich ants are plentiful.

**Why the bar measures the wrong thing for "food caps births."**
- The egg costs 120 J. A pupa costs 1,140-1,290 J of larval food on top of that.
- So the egg is about a tenth of a new ant, and the bar checks only that tenth.

**Where the other nine tenths comes from (larval food, `feeds.csv`):**
- 52-79% from nestmates' bodies (share and bank);
- 19-47% from food lying beside the larva;
- 0-4% from crops.

### 2. What holds eggs: rich ants are not at home, and the door they reach is packed with ants

**Rich ants do not stand at home.**
- 90-96% of affordable ant-time has no home cell within the pile's reach of 4.
- Affordable ants stand on the mound top (31-73%), in the mound's tunnels (4-50%, rising over the run) and on the
  surface (15-24%).
- Below the old ground line, 0-1% of ants are ever above the bar. **The nest's own ants are never rich.**

**The few within reach of home are blocked by ants, not brood.**
- They are 4-9% of affordable ant-time.
- The 9x9 around them holds 38-44 other ants and 0.0-0.2 empty home cells.
- 89-99% of the time there is not one empty home cell in it.
- Brood in that square averages 0.1-0.9 cells. A walk allowed through brood would find a site 0-3% of the time.

**So the counter that Nest race read as "egg space" is mostly rich ants away from home.** The remainder is ants
crowding the door. Brood filling the space plays almost no part.

**How rarely a held ant gets to lay.** Eggs laid are 900-1,140 per 50k. That is 0.05-0.09% of held ticks, the same
order as Nest race measured.

### 3. Where eggs go and where the brood ends up (the brood column)

**Eggs land in the door shaft's top.**
- 69-90% of eggs land within 2 columns of the door and within 10 rows of the old ground line.
- At 20-50k, 49-65% of layers stood on the mound top.
- Later, layers stand in the shaft's top rows (31-56%) and the mound's tunnels (19-48%).

**The brood ends up in one column.**
- Larvae lie at the bottom of the door shaft: 58-89% are within 2 columns of the door, deeper than 10 rows.
- The pictures show a column of brood straight down from the door to the room floor.
- The rest of the room is empty, with stored crumbs along its walls.
- The adults live on the mound.
- (Inferred, not traced: eggs fall down the shaft, since brood is a powder.)

**Few larvae have a nurse near them.**
- 11-17% of larvae have an adult over 200 J within 2 cells at 20-50k. That falls to 5-6% by 150-200k.
- 2-9% have a crop carrier within 2 cells.

**The brood queue grows and slows.**
- Standing larvae rise from 121-146 to 364-410.
- Larva to pupa takes a median 2,300-8,400 frames; the p90 is 11k-65k.
- Larvae starved per 50k rise from 0-13 at 20-50k to 149-234 at 150-200k. **This is the base arm, not brood carry.**

## Inferred, not traced

**The game already had a food cap, and this stack disconnects it.**
- `brood::nurse` gives a hungry larva a quarter of the surplus of the richest adult touching it. Its doc says: "a
  nest with brood waiting finishes them before it lays more: the regulation is a side effect, not a rule."
- That works only when layers stand beside larvae.
- Here layers lay at the shaft top, and the larvae lie 10 or more rows below. Rich ants stay on the mound.
- So the larvae rarely drain the ants that lay. The colony keeps laying while its larvae wait longer and longer.

**Brood carry (Nest race, eggs 1.5-2x, larvae starved 4.5-7.2k) is probably the same disconnection, made worse.**
- Carriers move brood deep into one heap on the room floor, away from the door crowd. That frees the shaft top,
  so eggs go up.
- Those larvae are farther still from any rich adult, so they starve.
- **Untraced:**
  - why carrying raises eggs, given that brood is not what blocks the door;
  - whether the carried larvae starve for lack of donors or for lack of food.

## Next steps, in order

**1. Do layers ever feed larvae? (No new run.)** Join `feeds.csv` donor ids (kinds share and bank) with broodlog
`laid` parent ids.
- If layers almost never donate, the disconnection above is measured rather than inferred.

**2. Trace the brood-carry arm.** It is built in the Laying container, with the census: `/home/claude/wt-carry`
= `claude/nest-race-two-crowds` 516a223c + `census.patch`.
- Run it with the env above plus `BROOD_CARRY=on BROOD_DEEP=on`, seeds 1-4, heap 90, to 200k.
- Then run `larvae.py` and `laycensus.py` on it.
- Questions:
  - Are the starved larvae the ones carried away from donors?
  - Is the colony short of food, or only of food reaching the brood?
  - What frees the egg sites: the ants at the door, or the brood near it?
- If the container is gone, rebuild: check out 516a223c, `git apply tools/census.patch`, then
  `cargo build --release --example deeptrace`.

**3. Write the food-cap proposal and send it to Nest race to review before anything is built.** Directions found so
far (a draft, not reviewed):

**a. Feed before you lay.**
- The rule: an ant that could lay first gives to hungry larvae it can smell (`brood::larva_scent`, reach 6), and
  lays only on a tick when it smells none.
- This turns the nurse side effect into a rule, with a local cue.
- The egg rate then follows how fast larvae are fed, which is how fast food reaches the brood.
- **The catch:** on this stack layers and larvae are 10 or more rows apart. The cue must reach across that gap, or
  the egg site must move to the brood.
- **Biology:**
  - Young larvae reduce workers' egg laying in bumble bees (Orlova, Starkey & Amsalem 2020).
  - Fire-ant nurses feed each larva at a rate set by its hunger (Cassill & Tschinkel 1995, already cited in
    `brood.rs`).

**b. Cull young brood when food is short.**
- The rule: hungry larvae, or adults, eat eggs.
- It gives a graded outcome and a verb.
- **Biology:**
  - Honeybees cannibalise young larvae under pollen shortage, cutting demand to match supply (Schmickl &
    Crailsheim 2001).
  - Ant queens eat brood and reinvest it in eggs (Bizzell & Pull 2024).
  - Formica larvae eat eggs (Pulliainen et al. 2019).

**Not proposed:**
- **An egg priced at the whole child.** It would make larval feeding pointless, and crop feeding of larvae is
  Scott's decision.
- **A colony-wide brake.** `FOOD_BRAKE` is global, not a local cue.

**Context for the proposal:**
- Larvae are the colony's protein sink, and colonies collect food differently with larvae present (Dussutour &
  Simpson 2009).
- Queen egg-laying rate follows protein intake (Abril & Gómez 2014).

**4. Rebuild on the store-crumb fix (`edible`) once Nest race lands it.** These runs had it off.

## References (retrieved from PubMed)

- Orlova M, Starkey J, Amsalem E (2020). Synergistic and additive effects of the queen and the brood on worker
  reproduction in a primitively eusocial bee. *J Exp Biol* 223. [doi:10.1242/jeb.217547](https://doi.org/10.1242/jeb.217547)
- Schmickl T, Crailsheim K (2001). Cannibalism and early capping: strategy of honeybee colonies in times of
  experimental pollen shortages. *J Comp Physiol A* 187:541-7. [doi:10.1007/s003590100226](https://doi.org/10.1007/s003590100226)
- Bizzell F, Pull CD (2024). Ant queens cannibalise infected brood to contain disease spread and recycle nutrients.
  *Curr Biol* 34:R848-9. [doi:10.1016/j.cub.2024.07.062](https://doi.org/10.1016/j.cub.2024.07.062)
- Pulliainen U, Helanterä H, Sundström L, Schultner E (2019). The possible role of ant larvae in the defence against
  social parasites. *Proc R Soc B* 286:20182867. [doi:10.1098/rspb.2018.2867](https://doi.org/10.1098/rspb.2018.2867)
- Dussutour A, Simpson SJ (2009). Communal nutrition in ants. *Curr Biol* 19:740-4. [doi:10.1016/j.cub.2009.03.015](https://doi.org/10.1016/j.cub.2009.03.015)
- Abril S, Gómez C (2014). Large and permanent colonies have higher queen oviposition rates in the invasive Argentine
  ant. *J Insect Physiol* 62:21-5. [doi:10.1016/j.jinsphys.2014.01.004](https://doi.org/10.1016/j.jinsphys.2014.01.004)
- Cassill & Tschinkel 1995 (*Anim Behav* 50:801-813), as cited in `src/sim/brood.rs` (`nurse_seek`).

The Consensus search quota is used up until 1 November. PubMed works but rate-limits bursts.
