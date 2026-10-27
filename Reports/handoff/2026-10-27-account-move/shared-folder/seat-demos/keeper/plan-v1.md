# Keeper seat: implementation plan to a playable demo (v1, 2026-10-25)

*Step 2 of five (BOARD.md). Builds on the adversarial review
([`review-feasibility.md`](review-feasibility.md): feasible with changes) and on
a step-0 live look run on the trained ant before this plan was written
(§1). v1 goes to an independent reviewer next, then is revised.*

## 0. What the owner will play, in one paragraph

`cargo run --release --bin lab -- demo=keeper` opens a wide formicarium box
(1024 columns). It holds a founded colony of trained ants and a small larder by
the nest that the keeper keeps topped up. Far out on the surface, about 200
columns from the nest, the player paints a **feeding station**. Left alone,
today's ants never find it: trained colonies do not wander past about 50 columns
(the lab playtest of 2026-10-24). That is the colony's real, known weakness, and
the keeper's job is to fix it.

The player has three hand verbs that teach:
- **carry an ant** to the station;
- **treat an ant**, a sugar bead that puts food in its crop and marks the spot as good;
- the existing **Food** brush, for the station itself.

A carried or treated ant **remembers the place and commutes**. When it feeds a
nestmate mouth to mouth, the nestmate gets a **second-hand, weaker copy** of the
place, and some of those ants start walking out too, though they were never
there. A "knows the station" share rises, then fades, because a memory
loses half its strength every 4,000 frames unless a fresh trip renews it. If the
player stops tending, the colony drifts back to foraging near home.

Every taught ant is tinted. A thin line joins each ant that learned second-hand
to the ant that fed it. One key switches the scripted listening gain to 0 (the
honest arm), so the owner can see what today's ants do with the same memories.

The lab's other hand tools stay available: Dig, Alarm, Water, Lamp. The HUD
counts what each one did, for the parts of the game that are honest today.

## 1. Step 0 already run: what today's trained ants do with the keeper's hand

**Instrument:** `examples/keeperlook.rs`, new on this branch.
- Box: `nest_goal`, built with `set_new_ant(true)` (asserted; 26 of 26 colony ants carry a mind).
- Window: 12,000 frames of warm-up, then the hand action, then 9,000 to 18,000 frames.
- Pairing: each arm is run against `arm=none` on the same seed.
- Raw output: `look/*.txt`.

| Verb | What happened | Number |
|---|---|---|
| **Hand-laid trail (B)** | **The trail is gone before an ant can walk it.** It was laid as the lab brush lays it, ramped nest to food. | B at the food end: 9,216 at laying, **42 after 100 ticks**, 10 after 1,500. The colony's *own* B in the same corridor reads 3,000 to 9,500, because ants refresh it. Corridor ant-ticks and far-food eaten are within seed noise of the control: trail vs none, far food left 14 vs 0 (seed 1) and 4 vs 12 (seed 2) |
| **Alarm puff** | **A flash, not a fence.** | Alarm at the puff point: 61,445 at drop, 1,957 at 100 ticks, 59 at 300. On seed 2, three puffs produced output **byte-identical to no puff**: no ant came near before it faded. On seed 1, ants near the puff rose (14,023 vs 10,252 ant-ticks), the opposite of repel, but that is one seed |
| **Hand-dug gallery (Dig, lined)** | **It survives and is slowly adopted, not torn down.** | Sketch cells still open at +18k: 261/265, 227/271, 249/267. Cells cut within 3 cells of the sketch vs the same band in the control: 36 vs 32, 32 vs 5, 40 vs 15. Ant-ticks in the sketched room: 345, 3,855 and 0, against 0 in every control. GIF: `look/v2-sketch-s2.gif` |
| **The box itself** | **The test colony starves out unless the larder is topped up.** | Ants at +18k (frame 30,000): 2, 10 and 3 of a 26-41 peak, in every arm. `nest_goal` is designed to be topped up by `examples/nestgoal.rs` |
| Meals | Adult-to-adult feeds are frequent | `minds.counts.fed_adult`: about 70 per 1,000 frames in a colony of about 30 |

**What this means for the plan:**
1. **Today's honest keeper is an architect and a larder, not a teacher.**
2. **The hand trail fails for a mechanical reason nobody had found:** the brush's
   deposit evaporates in about 100 ticks, at the engine's B decay. It is not
   that trained ants ignore B. This is a finding for every seat and for the
   lab's Scent tool. It is filed for the board, not fixed here.
3. **The demo's teaching has to ride a memory the ant carries, not a mark on the ground.**
4. **Feeds are common enough that a meal copy will spread a place quickly.**
   The copy must be weaker than first-hand, or the whole colony "knows" within
   one or two thousand frames and the share stops being graded.
5. **The demo box must keep its colony alive:** a topped-up larder by the nest
   (the keeper's job, scripted).

## 2. Scope

**In:**
- the `demo=keeper` launch, using the queen seat's launcher (`shared/launcher.md`);
- `assets/lab_scenarios/demo_keeper.ron`;
- the recall head (`shared/recall-head.md`, keeper-owned: scout depends on it);
- the meal copy;
- three keeper verbs: Carry, Treat, and Station (a marked Food brush);
- the overlays and HUD;
- a scripted paired harness that films the GIFs for the Artifact page;
- unit tests;
- the frame-cost check;
- a report in `Reports/`.

**Out (post-demo, named so they are not forgotten):**
- trust as a smell, and the gnome as a visitor (`shared/scent-ladder.md`);
- plants as words;
- nursery imprinting;
- a heritable listening gain, with selection on it;
- a persistent hand trail (a longer-lived player deposit is a design question
  for the owner, not a demo hack);
- the gnome's body on screen (review §C: cosmetic, after the demo is judged).

**Never in the demo:**
- any change to the shipped lab with no `demo=`: it must stay byte-identical;
- any change to training or to `std_pool`.

## 3. Engine pieces

### 3.1 The recall head (shared, keeper-owned): `src/sim/creature/recall.rs`

The precedent is `DigHead` (`mind.rs:5833-5856`): a term added to an option's
raw score before the sharpness squash, kept out of the genome.

- `World::recall: Option<Recall>`, with `Recall { gain: f32, copy: f32, counts: RecallCounts }`.
  **`None` means off and byte-identical**: one `Option` read per decision.
- **The term.** In `mind::decide_planned`, after raw scores are computed and before the squash:
  - for each **Walk** option of an ant that is **not laden** (`trip_load == false`, no crop load), add `gain * v[f::FOOD_WAY]`;
  - the feature is already computed for every option (`options.rs:1128-1137`) whenever the last-food memory is on (default on), so the term adds no feature cost;
  - laden ants are left to go home, so a taught ant does not walk back out with a full crop.
- **`recall::plant(world, ant, cell)`** (public; scout calls it on tandem arrival): sets `last_food = (cell, frame)`, fresh, and marks the ant `taught_by = Hand` for the readout.
- **The meal copy.** In `mind::feed`, adult branch, after the share (`mind.rs:3086-3093`): if `recall.copy > 0` and the giver's memory is fresher than the receiver's, the receiver takes the giver's place, but **aged**. The written frame is pushed back so its freshness is `copy` times the giver's: `fresh = T/(T+age)` inverted. The copy remembers the giver id for the readout line.
  - With `copy = 0.5`, a second-hand memory starts at half strength and a third-hand one at a quarter. The epidemic is self-limiting by distance from the teacher, as well as by time.
- **Read-outs:**
  - `RecallCounts { decisions_moved, plants, copies, copies_by_generation[4] }`;
  - `recall::knows(world, ant, place, r) -> Option<f32>`, the freshness if `last_food` is within `r` of `place`.
- **"Decisions moved" counter:** for an ant with a nonzero term, recompute the argmax with the term at 0 and count the difference. This is the counter that says the fake fired (review must-fix 5). Its sensitivity control: at gain 0 it must read exactly 0.
- **Tests:**
  - (a) off means a byte-identical world hash over 3,000 frames of `nest_goal`;
  - (b) at a gain, a planted ant's Walk pick turns toward the spot, and the test goes red with the term removed;
  - (c) laden ants are untouched;
  - (d) the copy halves freshness, and gain 0 means `decisions_moved == 0`.
- **Land it first, as its own small PR, so scout can stack on it.** The scout plan §7.5 calls `plant` and the knows readout.

**Why not hand-seed `FOOD_WAY` into the pool:** no free scorer unit (review
finding 3). **Why not a `chooser_step` pull:** minds never reach it (finding 2).

### 3.2 Carry: `recall::carry(world, ant, to) -> Result<(), &'static str>`

- Lift the body: read its chain cells, clear them, and rewrite the same cells translated to `to`. Accept only if every target cell is air and the lowest has footing. Otherwise search a small radius for the nearest place that fits.
- **No change to energy or crop.** The ant then finds the food itself, and its *own* `pick_up` writes `last_food`. So **carry teaches through the engine's own memory write**: the only fake is the gain that reads it.
- The lab verb: press on an ant to lift it, then release where you want it put down. The ant is drawn under the cursor while held, and the world is not paused. Precedent for press/release state: `lamp_grab`.

### 3.3 Treat

- Click an ant to give it one mouthful of provisions into its crop, a real meal from the game's crop path (`crop_take` with the provisions material's worth), and `plant` its memory at the click cell.
- It is a real reward, because an ant must consume it (keeper.md §4 item 16, the rule against reward hacking).
- A small gold pip rises off the ant.

### 3.4 Station

- The Food brush, unchanged, plus `demo::keeper::stations`: each Food stroke more than 40 columns from the nest registers a station (its centroid).
- Station readouts: visits by taught ants, visits by untaught ants, and deliveries home that came from the station.
- The nest larder: the scenario keeps a heap beside the nest topped up every N frames (the keeper's chore, scripted and labelled "LARDER KEPT"). This stops the colony starving out, as it did in §1.

## 4. The lab side: `src/lab/demo/keeper.rs` (queen's launcher)

- **`demo=keeper`:**
  - loads `demo_keeper.ron`;
  - sets `world.recall = Some(Recall { gain: G, copy: 0.5 })`, with G chosen in step 3 below;
  - arms the larder top-up.
- **Keys (seat keys from `shared/steered-ant.md`):**
  - `Y` Carry, `U` Treat;
  - `K` toggles the gain between G and 0, labelled "LISTENING: SCRIPTED GAIN G" or "LISTENING: OFF (TODAY'S ANTS)";
  - `J` cycles the overlay: off, knows-the-station, traffic.
  - `H` shows the help card.
  - Food, Dig, Alarm and Water stay on the bar as they are.
- **Overlays:** a full replace on a fixed ramp, never a blend (CLAUDE.md debug-readout rule). They are drawn only when on, so the dirty-rect skip is untouched when off.
  - **Knows:** a ring on each ant whose memory points at a station, coloured by hand-taught or by copy generation 1, 2 or 3. A thin fading line from each copied ant to its giver, for 30 seconds.
  - **Traffic:** a fading u16 heat ribbon of ant steps over the last 2,000 frames.
- **HUD line:**
  - stations: knows N of M ants, a 60-second sparkline of the share;
  - deliveries from the station, cumulative;
  - decisions moved (last 1,000 frames);
  - carried and treated;
  - meal copies by generation;
  - plus the existing counters for Dig (cells cut near your sketch), Alarm and the larder.
- **Ant page:** one plain sentence, for example "Remembers the east station (taught by your hand, 2 trips since)" or "Heard of the east station from Bramble 412 (second-hand)". It uses the existing page machinery.

## 5. Build order, each step with its proof

| # | Step | Proof (posted with the count beside the picture) |
|---|---|---|
| 1 | Recall head, plant, meal copy, counters, tests (§3.1); PR to the merge desk at once | The four tests green; test (b) red with the term removed (show it) |
| 2 | `keeperlook` grows `arm=carry`: on `demo_keeper` geometry, carry 3 ants to a station 200 columns out, at gain ∈ {0, 0.5, 1, 2, 4}. 3 seeds | Table by gain: station trips per carried ant over 12k frames, `decisions_moved`, share of a carried ant's unladen walks that head out. **Pick G as the smallest gain giving 2 or more return trips per carried ant on every seed, while laden and hungry ants still go home.** Gain 0 must show about 0 return trips: the honest arm |
| 3 | Same harness with the copy on at G: the knows share over time, copies by generation | A curve that rises and then decays (graded, not 0 or 100%); the share at peak reported, aiming for 20-60% |
| 4 | `demo_keeper.ron`, the larder top-up, the station registry | Untaught colony over 30k frames: alive (ants at least 15 at 30k on 3 seeds), **0 station deliveries** (the station really is out of reach untaught) |
| 5 | The lab verbs Carry and Treat, the gain key, overlays, HUD, page sentence | `uishot`/`labgif` stills of the HUD and overlay; a scripted click run (`PIXEL_PHYSICS_LAB_CLICK`) carrying 2 ants, then 6k frames: GIF |
| 6 | The paired film for the owner: same seed, a scripted 10-minute session (carry 2, treat 1), gain G vs gain 0, side by side, with the counts in the captions | The Artifact page: the two GIFs, the counts table, the step-0 look GIFs (sketch vs none), and one sentence on what is scripted |
| 7 | Frame cost: `examples/ascii.rs` and `labperf` with the demo off vs on | The worst frame quoted; off must be unchanged |
| 8 | Report `Reports/keeper-seat-demo-2026-10-25.md` (step 0, plan, results, what is scripted), its line in `Reports/README.md`, the lab Scent-trail lifetime finding in `open-bugs-handoff.md` (bug letter from `bugindex.py --branches`), and the `instruments.md` line for `keeperlook` | `docscheck.sh` green |

**Parallel with others:**
- step 1 lands first for scout;
- step 5 waits on the queen's launcher SHA. If it is late, a `PP_KEEPER_DEMO=1` env gate in `bin/lab.rs` is the stopgap, swapped for `demo=keeper` when the launcher lands.

## 6. Risks and their early counters

| Risk | Counter |
|---|---|
| The term never moves a pick (the job head rarely picks Walk for unladen ants, or the squash flattens it) | `decisions_moved`; step 2's gain sweep. If no gain gives a commute, the fallback is a forced pick toward the place for unladen taught ants, like scout's forced pick, labelled and *weaker* honesty. Say so to the owner rather than hide it |
| Pikmin: taught ants pace the line and starve or abandon digging | Taught ants' share of unladen walks heading out (target 60-80%, review); digs per taught ant vs untaught; starvation among taught ants |
| `last_food` is overwritten by food found nearer home (the larder), so the lesson "jumps" | The larder sits *at home* (`at_home` pick-ups never write memory, `mind.rs:2828`); check the per-taught-ant distance from memory to station over time |
| The copy saturates (everyone knows) or never spreads | Step 3's curve; tune `copy` in {0.25, 0.5, 0.75} |
| The station is reached untaught anyway (as at 150 columns in §1) | Step 4's 0-delivery check; move the station further, or behind a Wall stretch |
| The demo measures the old ant | Every harness asserts `new_ant` and the mind count (already in `keeperlook`) |
| Frame cost | Step 7. The term costs one multiply-add per Walk option for taught ants only. The overlays draw only when on |
| A wide box is slow on the owner's machine | Fall back to 768 wide if `labperf` says so |

## 7. Collaboration

- **scout:**
  - consumes `recall::plant` and `recall::knows` (agreed in its plan §7.5);
  - "carry" (mine) and "lead by touch" (theirs) teach the same lesson through the same memory and copy, so **one demo with two hands is possible**: `demo=keeper` could later enable scout's tandem key;
  - offered in both status files; decided after both demos are judged.
- **queen:** the launcher; seat keys Y, U, J, K, H.
- **guest:** the scent ladder for the trust idea, post-demo.
- **colony mind:** the copy rule may be what it wants for "habits".
- **Teacher lane:** nothing here touches training. Tell them about the B-trail lifetime finding and the recall head (a candidate trained sense later: a heritable gain rider). Also: `Lab::new` defaults to the old ant.

## 8. What is scripted, stated for the owner

- The listening gain G: hand-set. Today's pool has zero weight on this memory. It is shown on screen, with a key to switch it to 0.
- The copy fraction: a design choice, not evolved.
- The larder top-up: the keeper's chore, done for you.

Everything else is the engine's own:
- the memory and its fade;
- the ant's own pick-up writing it;
- feeding, walking and starving.
