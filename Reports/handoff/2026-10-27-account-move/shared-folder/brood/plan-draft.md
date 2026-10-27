# Brood and breeding in the training world — plan (draft for review)

Brood and breeding lane, 2026-10-23. Parent: Teacher lane (owns the one ant line). Code: branch
`claude/project-thread-6einf2` (off the compute branch `claude/project-thread-8y5ak0` at 89453ec), `examples/inrun.rs`
switch `brood=lay`. Pilot data and scripts: `/mnt/project-files/brood/`.

## 1. The problem, and the owner's rulings

The owner: "I don't love that the nest is 100% full of food." In colsel20's rules-off gardens each nest is one roofed
bowl packed with stored food (light-head arm: median 389 stored cells, all 24 ants alive, banks ~87 full grants).

**Cause.** The training world (`examples/inrun.rs`, `train::challenges::inrun`) runs with the body's laying off
(`births_paused`, `reproduce_threshold = 1e30`). Ants appear only as selection births (`inrun::birth`), each handed
`kid_grants` × 200 J (3 × 200 = 600 J in colsel21) **from nothing**, and the headcount is held in a band (`lo=24..hi=36`).
So food has no use beyond the adults' upkeep. One food cell is 960 J; an adult costs ~1,040 J; 389 stored cells is
~350 ants that were never made. The lab game does not have this problem because its brood eats the surplus.

Owner rulings in this thread (2026-10-23):
- 16:30Z: design eggs and brood **as they should work for the full game**, using the old ant and real biology. **When
  an ant is ready to be born, its genome is chosen by the selection step** — to avoid a delay between selection and
  the ant appearing.
- 16:32Z (question): should laying behaviour/properties be in the genome? (Answered: yes, judged at colony level; §5.)
- 16:33Z: **reward food and brood being kept separate, ideally eventually in separate rooms.**
- 16:35Z: predicts **boom and bust** (lots of food → population overshoots → crash).
- Standing: one physics for every world; no hand-made castes; ants dig and forage all game; land via the Merge desk,
  off by default if it changes results; Teacher lane owns the one ant line (deliver mechanics, not a separate ant).

## 2. What already exists (the game's brood, `src/sim/brood.rs`)

Built 2026-10-02 onward from the old ant's work and ant biology; on by default in every world, but the training world
never lays. Per adult: an ant whose bank reaches the brood block's `lay_at` (1,100 J × a heritable multiplier) lays
one egg (`egg_cost` 120 J) onto the brood pile at a home cell within 4 steps (`pile_site`). Egg 250 frames → larva,
which must be fed up to the adult's price (`target`, ~1,040 J): by a nest-bound nestmate's bank on touch (`nurse`,
`NURSE=workers`), by a carrier's crop food on touch (`crop_feed`), and by eating food beside it. Unfed larvae pay
upkeep and starve first in a famine (graded shrinkage). Pupa 250 frames → hatch on or near its cell. The egg carries
its layer's mind (`mind::inherit`).

Regulators already in the game, all **off by default**, all applying to mind ants too (laying runs in the body's
`settle → try_bud`, `src/sim/creature/mind.rs:2271`, for every ant): breeding regime `graded` (on: bar raised near
another breeder), `PIXEL_PHYSICS_FOOD_BRAKE` (income vs burn), `PIXEL_PHYSICS_STORE_BRAKE` (store per hundred adults),
`PIXEL_PHYSICS_FEED_FIRST` (no egg near a starving larva), `PIXEL_PHYSICS_EGG_DOOR` (off/door/cut/deep),
`PIXEL_PHYSICS_BROOD_SPREAD`, `PIXEL_PHYSICS_BROOD_CARRY`. Laying traits already in the body genome:
`TRAIT_REPRODUCE_AT` (bar multiplier), `TRAIT_BIRTH_GRANT`, `TRAIT_DOOR_CLEAR`; the old brain's `Lay` output (mind ants
pass "lay" always).

## 3. The design

### 3a. Life cycle: the game's own, unchanged
`brood=lay` turns `births_paused` off in the training world. Nothing about laying, feeding, pupating or hatching is
training-specific: the lab and the training world run the same brood code (one physics).

### 3b. The genome is chosen at hatching (built, `f5f31e2`)
Each judgement, selection's parents (the same pick as today: quota z-rank or lexicase, kid caps, fresh_kid, retire)
are queued best first, round by round (every parent once, then those given two, then three). The frame an adult
hatches (`creature_stats.births` changes), the harness takes the next queued parent still alive and under its cap,
mutates its genome exactly as a quota child is mutated, and writes it into the hatchling with `mind::give`. Fallbacks:
best living adult by rate, then best of the hall, then the founder line. `sel=off`: the hatchling keeps its layer's
mind (as in the lab). Selection's own births (quota, refill, hall refill) are skipped: **food decides how many are
born, selection decides who.** The quota's culls wait for hatches as they waited for births. Logged as `hatch` rows in
`births.csv`; `brood.csv` every 1,000 frames; `occupants*.txt` maps food, egg, larva, pupa and ants.

Known gap: body traits are still the layer's (body genes do not mutate in the bed yet; Body genes lane). When they do,
they should be written at hatch from the same parent.

### 3c. Regulation (the open problem — see pilot §4)
Laying with no brake overshoots to 150–200 ants and holds there by starving ~650 adults a round. The game's food and
store brakes swing 40↔200 (boom and bust, as the owner predicted). A training-time size brake `lay_cap=T:D` (the
laying bar doubles for every D adults+brood above T) is the steadiest but still lets a colony stop breeding on fat
banks and then die in lockstep (seed 3: 72 → 27 at 100k; two of three seeds extinct in some arms).

Proposed direction, to be tested as arms (all off by default, all game rules unless marked):
1. **Brood-per-nurse brake (new game rule).** Biology: a queen's laying follows the food nurses pass up from the
   larvae (Tschinkel 1988), and colonies hold brood roughly in proportion to workers. Rule: the laying bar rises with
   (larvae + eggs) per adult above a ratio R, so the colony never commits to more brood than its nurses can feed.
   Graded, keyed on the colony's own state, not on a headcount.
2. **Training-only size cap** `lay_cap` for frame cost (the bed's cost grows with ants: 2x at ~60 ants, 5x at ~165).
   Marked as a training lever, not physics.
3. Food brake and store brake as comparison arms only (they boom-bust here).
4. The lockstep die-off is a foraging failure, not a laying one: with births drawing banks down to 3–30 grants
   (from ~70) hunger is now real in training, so colony selection can finally see it. Do not add a rule for it now.

### 3d. Brood placement and food/brood separation (owner 16:33Z)
Pilot: brood lands as columns on the chamber floor and in the shaft (a non-rolling powder stacks); `EGG_DOOR=deep`
changes nothing while the colony's bowl is all it has dug; `BROOD_SPREAD` turns one column into several short ones.
1. **Measure first**: per brood cell, distance to the nearest stored food cell; per nest, the fraction of brood with
   no food within 3 cells; later, whether brood and food lie in different rooms (`roomshape.py` segmentation). Printed
   with every garden picture.
2. **Pay the ant that chooses**: ants decide where food goes (Drop). A food put-down within k cells of brood pays
   less; a put-down in the nest but away from brood pays in full (training score, `inrun.rs`).
3. **Colony case** for the Teacher lane's colony lexicase: brood/food separation from the end map.
4. **Maybe a game rule**: `pile_site` prefers empty home cells with no food beside them (graded tie-break, off by
   default until measured).

### 3e. Laying genes (owner 16:32Z)
Keep laying in the genome (it already is: bar multiplier, birth grant, door clearance). With the genome chosen at
hatch, the per-ant score never rewards laying or nursing; only colony selection can judge them. Order: switch with
laying genes fixed → mutate them with body genes → check whether colony selection moves them.

## 4. Pilot evidence (2026-10-23, hcs founder, round 0, colsel21 inrun args, 3 seeds × 200k frames, local 4 cores)

| arm | adults after 20k (min/max over seeds) | stored food (mean) | adults starved/round | larvae starved | food home | dug | frame cost |
|---|---|---|---|---|---|---|---|
| today (no laying) | 24/24 | 15–56 | 0–24 | – | 896–973 | 235–273 | 1x (32–37 s) |
| laying, no brake | 94–204 | 2–5 | 602–682 | 214–1,096 | 1,685–1,890 | 283–354 | ~5x |
| + size cap 48:8 | 26–90 | 5–7 | 95–232 | 0–12 | 1,061–1,789 | 267–270 | ~2x |
| + store brake | 36–175 | 6–7 | 264–569 | 0–13 | 1,102–2,526 | 286–358 | ~3x |
| + food brake | 40–207 | 3–8 | 522–1,341 | 100–642 | 2,059–3,418 | 303–416 | ~4x |
| cap + egg door deep | 0–78 | 2–8 | 102–155 | 1–68 | 260–1,037 | 253–275 | |
| cap + brood spread | 21–114 | 1–6 | 74–169 | 0–17 | 562–958 | 266–410 | |
| cap 36:16 (softer) | 1–129 | 2–9 | 43–225 | 0–39 | 636–2,489 | 301–501 | |
| cap + feed first | identical to cap on 2 of 3 seeds (never fired?) | | | | | | |

Pictures: `pilot-curves.png` (adults and stored food over the round), `pilot-nests-end.png`,
`pilot-nests-placement.png`. Three seeds is not a sweep: these are direction, not verdicts.

## 5. Measurement plan (before anything is called better)
- Paired arms by seed, 16+ seeds per arm (GitHub Actions or a VM slot from the Teacher lane), full colsel rounds with
  inheritance, not round 0 only. Judge on the quantity, never a pass/fail.
- Larder: `nest_food`, churn (put-downs at home), tank_mean (the gut should fall toward a few grants).
- Population: adults over time (variance and minimum, not the end), adult starvations, larvae starved per egg,
  extinctions (paired McNemar).
- Brood: where it lies (separation measure, rooms), how many stand unfed.
- Work: food home, kept nest (`nestcount.py`), crater, rooms (`roomshape.py`), spoil — digging and foraging must hold
  all game.
- Cost: wall time per 200k frames against adults.
- Positive controls: `brood=off` reproduces today's numbers byte for byte; a sweep where food is cut must shrink the
  colony; the separation measure must score a hand-built mixed pile low and a two-room nest high.

## 6. Steps
1. (done) `brood=lay` + genome at hatch + `lay_cap`, logs, occupant maps. Pilot.
2. Brood-per-nurse brake arm; separation measure; 16-seed batch of the regulation arms.
3. Separation pay + colony case, handed to the Teacher lane as switches.
4. Multi-round colsel with brood=lay on the current founder (hcs + dh7 dig head, PP_DIGLIGHT=1, PP_HOMEVEC=40:30,
   weaned body rules) against today's colsel21.
5. Land: harness switches onto the Teacher lane's compute branch; any game rule (brake, egg-site preference) to main
   through the Merge desk, off by default, with its measurement.

## 7. Risks and open questions
- Selection pressure changes: births now come when food comes, so selection's quota no longer sets turnover; with
  many hatches the queue cycles the same top parents (diversity risk), and with few hatches selection stalls.
- Newborns hatch with ~0.5 grant (the brood's price minus the body) instead of 3 grants: more newborn starvation.
- The quota's hard culls scale with colony size (929 culls in a 165-ant colony vs 131 at 24).
- Frame cost scales with ants; the VM is one machine.
- `FEED_FIRST` looked inert; check it fires.
- Brood stacked as columns may block the shaft; ants walk through brood (`PushPast`), but nothing measures the jam.
