# A needs-and-jobs ant: design for a second walk

Oct 6, 2026 · @Scott Gladstein

## Summary

This is a design for a second way for the lab ant to decide what to do. It is built beside the shipped ant and switched off, so nothing else is risked if it fails (Scott, 2026-10-06).

- **The problem.** Today a small brain decides *when* to act and about 27,000 lines of hand rules decide *where*. The walk takes the first matching pull, so hunger acts only when nothing else matched; six vetoes let a job hold a starving ant; and nothing pulls an idle ant inward, which is why ants spend about 5% of their time in the dug nest.
- **The design.** Two needs, hunger and escape, that outrank every job. Seven jobs, counting idle, taken by response thresholds that are genes. One steering sum over a fixed set of cue terms, weighted by the winning drive. Idle ants held inside by a kinesis on how stuffy, dark and crowded a place is.
- **Scents, taken seriously.** Measured on 12 real nest maps: a properly walled nest gas points the right way in tunnels, but rooms are flat (a 6-cell sensor reads under 1% on about nine cells in ten); any leak through soil points it at the roof; and today's terrain-blind planes point it away from the door. So gradients are used only where measured, rooms rely on level and touch, routes keep presence plus the home vector, and `NestWay` stands in for the nest-air slope until a solved field passes its check.
- **How it is judged.** By designed behaviours with traces (time in the nest, separate rooms, food apart from brood, foraging, brood care), a no-veto guard, paired seeds and a robustness score. Colony size is only a floor.
- **Build and stop.** Instruments first; then needs and idle, switched on at a frame after founding; then digging and escape; then nursing, laying and storing. Kill points after the first two slices.
- **For Scott:** the go-ahead for a separate thread, and five design questions (the last section).

## Where the ant is today

Today's ant is a small brain that decides *when* to act, wrapped in a large set of hand rules that decide *where* to go and *which job* to do. Counted in the code on main 3ba1e7bd (2026-10-06); the rule audit has the detail (rule-audit-2026-10-06.md).

| Part | Size | What it decides |
| --- | --- | --- |
| Brain (`brain.rs`) | 33 inputs, 8 hidden units, 17 outputs; about 57 authored weights of \~940 slots | the odds of stepping, eating, dropping food, dropping soil, digging, sharing, biting; how much trail to lay |
| Hidden units in use | 4 of 8 retired under the chooser walk (trail inputs fed as 0); 3 single-purpose (nest odometer, nest dig gate); 1 free | almost nothing combinational |
| Hand rules (`creature.rs` before its tests) | about 27,000 lines; 132 environment switches in `creature.rs` and `brood.rs`; 93 switch rows in `how-the-ant-works.md` §12 | where to walk, which pull wins, who is a nest worker, when to go out, where to drop and lay |

**The walk is a ladder.** `chooser_step` fills one pull slot with the first match: store trip, walk home to lay, nest-worker leash, soil out, back to the face, laden carry, then the hungry way out, the mound way out, the rest pull. Scouting, the only hunger-driven search, acts only when nothing above it matched.

**Hunger is what is left over, not a need that competes.** That is why a pellet holder can starve beside food (traced, findings 30, 31, 35). The audit found five more vetoes of the same kind: the door throttle, the lean no-dig, the heap cue at zero, `FaceTrip`'s `only`, and a caste fixed by id.

**Nothing live pulls an idle ant inward.** Every live pull aims at the door, out, or the ant's last home contact, except a digger's walk back to its face. So 4-6% of ant-time in the dug nest (traced, Deep trace baseline) is what the rules predict.

**Patches stack.** The heap cue blocked breakouts, so `DOOR_REOPEN` waived it at the door. Lean ants could not dig, so `MOUND_OUT=dig` let them in the mound. `HUNGRY_OUT`'s way missed the mound, so `MOUND_OUT=way` was built. Each was a correct fix of a traced problem; together they are a rule set nobody can predict.

**Why it grew this way.** The brain cannot sense where anything is: it has no direction inputs, and the trail readings are zeroed for it under the chooser. Evolution gets about 17 generations in a 300k test run, with mutation off while measuring. So every wanted behaviour had to be written by hand, one symptom at a time.

## Goals and non-goals

The goal is an ant whose behaviour comes from a few general mechanisms reading local signals, so that the designed colony behaviours appear without a rule per symptom. Survival is a base need inside each ant, never the score.

**Designed behaviours it must produce** (the lab goal, Scott 2026-10-03, and the standing ant goals):

1. Ants live in the nest: most ant-time underground in the dug nest, not on the mound or at the door.
2. Separate chambers: brood rooms and worker rooms apart, built by the ants' own digging.
3. Food organised apart from brood: stores the colony keeps and draws on.
4. Foraging from the player's food, with recruitment that follows supply.
5. Brood care: larvae fed where they lie, eggs laid where brood is kept.
6. Later: defence against rival colonies and predators.

**Principles.**

- **Local signals only.** An ant acts on what it touches, smells, carries or remembers. A global count is allowed only as a labelled stopgap with its local replacement named (Scott, 2026-10-06).
- **Needs are base.** Hunger, being trapped and rest rise with the deficit and cannot be switched off by what the ant holds or what job it is on (Scott, 2026-10-06).
- **One way to steer.** Every direction an ant takes comes from the same weighted sum over a fixed set of cue terms, so a new behaviour is new weights, not a new code path. The terms are of several kinds (slopes, routes, memory, gravity, touch), because most scents have no usable slope where the ant needs one (*Scents and fields in depth*).
- **Designed, then judged by trace.** Each mechanism names the behaviour it serves and the trace that shows it working; colony size is a check, not the target.
- **Mechanism is code, policy is genome.** Weights and thresholds are genes that start at designed values (owner ruling on the breeding plan, §2a).
- **Frame cost is a hard limit**, and the world stays deterministic per build and seed.

**Non-goals.**

- Not a neural network learned from scratch. In our generation counts it would not get there, and if it did it might find the colony that sits on the food and lays.
- Not a change to world physics: soil, footing, sliding, liquids and plants stay as they are.
- Not a change to the outdoor or held games, or to the shipped lab ant, until it has earned it (next section).
- Not exactness: a field that is nearly right and cheap beats one that is exact and slow.

## Kept separate from the shipped ant

The new ant is a second walk the lab can switch to, built in its own files, off by default, so if it fails nothing else is lost (Scott, 2026-10-06). The shipped ant, the nest lanes and their baselines do not change and never wait on it.

**How it plugs in.** The tick already runs sense, brain, costs, `act`, the walk, trail laying, digestion and budding in that order (`creature_tick`). The new layer replaces only the middle: brain, `act` and the walk. Everything around it (body, falling, costs, digestion, eggs and brood, world physics) is shared.

| Piece | Where | Touches shared files? |
| --- | --- | --- |
| Needs, jobs, steering | new module `src/sim/needs/` | no |
| New scent fields and their update | new file beside `pheromone.rs` | one call in the world step, skipped when no ant walks this way |
| The switch | a new walk, `PIXEL_PHYSICS_WALK=needs` (or `World::walk`) | one branch in `creature_tick` where `act` and `chooser_step` are called |
| Lab founder | a scenario row or `LAB_ANT=needs` | `lab/scene.rs`, one option |
| Measuring | the existing `deeptrace` with extra columns | `examples/deeptrace.rs`, columns only |

**Off means bit-identical.** With the switch unset the game must match main frame for frame. That is checked at every landing with Deep trace's identity check (`deep-trace/tools/identity.py`), and the new code takes no random draws while off.

**Branch and landing.** Work happens on its own branch. Each finished slice lands on main switched off, so the branch never drifts far behind main (`branchcheck.sh` BxF stays under 300) and its arms are measured against the same shared baseline as everyone else's. Nest lanes never edit its files; it touches theirs only at the hooks above.

**Who builds it.** A separate thread, on Opus, started only on Scott's yes. No current lane is diverted.

**How to abandon it.** Revert the hook commits and delete the module. Because the shipped ant never changed, nothing measured elsewhere is invalidated. The kill criteria are in *Risks*.

**Known trap.** Adding a walk or a species enrols it in every rule that sweeps those sets (`CLAUDE.md`, *Adding a member to a set*). Every `match` on the walk and every test that iterates walks or species must be checked when the switch is added; `cargo test` (not `--lib`) reaches them.

## Architecture

The new walk keeps the engine's body and replaces the decision. One tick, in order:

1. **Sense.** Only what the ant can know: its own energy, crop and jaws; what it touches (soil, brood, food, nestmates, open air); short-range scents summed from sources in reach; trail presence where a step would land; nest air's level here and, where one exists, its slope; gravity, dark and walls; and its memory (home vector, patience, how long it has been still).
2. **Needs.** Hunger and escape, each an urge from 0 to 1 that rises smoothly with its deficit. Water and heat come later.
3. **Jobs.** Forage, haul soil, dig, nurse, store and lay, with idle as the default. Each job has a stimulus the ant senses and a threshold that is the ant's own gene. The ant holds one job at a time, with hysteresis so it does not flicker.
4. **Arbitrate.** A need whose urge passes the job's hold takes over, whatever the job or the load. Whatever the jaws hold is eaten, dropped or packed first if it stands between the ant and the need. This is the audit's no-veto rule built into the structure, instead of checked rule by rule.
5. **Steer.** The winning drive (a need, the job, or idle) sets weights on a fixed set of steering terms. Every usable heading is scored by the weighted sum, and the pick is the existing `choose_weighted`, whose randomness is intended. A kinesis term decides whether to step at all.
6. **Act.** The drive picks the act from what is in reach: eat, drop, dig, pick up, feed, lay or fight. The acts are the engine's existing mechanics; only the decision to use them moves.
7. **Trace.** Every decision records the drive, the job, the strongest terms on each heading and the heading taken, so "why did it do that" is answered per ant (owner's rule, 2026-09-20).

&#91;embedded content: one tick of the new walk · senses, needs and jobs, arbitration, steering\]

**What stays exactly as it is:** the body, falling and footing, energy costs, the crop and digestion, sharing, eggs and brood, the dig cut and pellet physics, the trail planes, and every world rule. **What is new:** needs, jobs, arbitration, the steering terms that do not exist yet (gravity, walls, kinesis, nest air), and the trace columns.

**Why this shape.** It is the response-threshold model of division of labour (Bonabeau, Theraulaz & Deneubourg 1996, in the project's references) with needs placed above jobs, feeding a steering stage much like the one the shipped walk already has. The real difference from today is arbitration. Today the first matching pull wins and silences everything below it. Here every term the winning drive cares about is added, and needs outrank jobs by construction.

## Needs

A need is an urge inside the ant's own body that rises with a deficit and outranks every job. Two to start: hunger and escape.

**Hunger.** Its urge is a smooth ramp of the ant's own energy: 0 above `hunger_onset` (a gene, starting at 0.6 of the birth grant), 1 at `hunger_full` (a gene, starting at 0.25), smooth in between. There is no cliff like today's `LEAN_LINE` at 0.5. What hunger does, in order:

- **Eats what it holds first.** Food in the crop, or a store load in the jaws, is eaten before the ant walks anywhere. Soil in the jaws is dropped where it stands, or packed into the wall when there is no open cell (Nest building's wall-pack rule, built for the shipped ant).
- **Goes toward food it can sense:** food at its head, crumbs, a pile within reach (summed from sources, the way `larva_scent` is).
- **Goes out, when inside:** up, along the walls, and back along the door vector; in tunnels, down the nest-air slope (`NestWay` until nest air exists).
- **Gets onto routes** by trail presence, with direction from the home vector: away from home when empty.
- **Begs** from a nestmate it touches, with today's sharing mechanics.
- **Scouts** when nothing else is sensed: today's scout term, scaled by hunger.

**Escape.** An ant is trapped when a need wants it out and it cannot get there: it is under cover, its progress toward out has stalled (its patience has run down), and this has gone on for a while. All of that is the ant's own state. It never needs to know that a door is shut, only that it has been pushing at soil. Escape rises with the stall, faster when hungry, and lets the ant dig at any energy, upward and toward the door vector. It is `MOUND_OUT=dig` as a need, everywhere, including the shaft below the founding ground where seed 1's ants died (finding 36). The heap cue's zero and `FaceTrip`'s `only` cannot refuse it, because they belong to jobs.

**Rest is not a need.** Idle is what an ant does when no need or job passes its threshold. Real colonies keep many workers idle as a reserve: "most social insect colonies contain large numbers of highly inactive workers" (Charbonneau, Sasaki & Dornhaus 2017, PLoS ONE 12:e0184074, [doi 10.1371/journal.pone.0184074](https://doi.org/10.1371/journal.pone.0184074), via PubMed). Idle ants are held inside by the level kinesis of *Scents and fields*, not pulled by a rule.

**How a need takes over.** Each job carries a hold, which rises while the job makes progress and fades when it does not. A need wins when its urge passes the hold. The job is then suspended, not forgotten: a digger that went out to eat walks back to its face by memory afterwards. The no-veto guard in *Measurement* checks that no job, load or place can keep an ant at 30% energy away from food.

**Later needs:** water, once the lab has dry spells; heat.

## Jobs and response thresholds

Each job has a stimulus the ant senses, a threshold that is its own gene, and an end. A job is taken with probability s² / (s² + θ²) per decision, where s is the stimulus and θ the threshold. This is the response-threshold rule (Bonabeau, Theraulaz & Deneubourg 1996, in the project's references): ants with low thresholds take a job first, and the share of ants on it follows how strong its stimulus is.

| Job | Stimulus (local) | Ends when | Steers by | Today |
| --- | --- | --- | --- | --- |
| Forage | the forage drive from meetings with returning foragers (today's `met`, local and graded), plus the ant's own hunger | crop full, when it turns to carrying home; or patience runs out | out: routes with the away term, the door reader, scouting. Home: home vector and route presence, laying trail B only while closing on home | forage drive, scout, `trailaway`, home pull, laden carry |
| Haul soil | a pellet in the jaws, or loose soil at its head inside | the pellet is down at a heap | out by up, walls, door vector and nest-air slope (`NestWay` until then); the drop site by building scent | soil out, `SOIL_WAY`, `WAY_GAPS`, the spoil ring |
| Dig | soil at its head, plus crowding at its head (bodies per open cell within 2) | crowding at its face falls below threshold, or patience runs out | back to its face by memory (a face vector), down, away from crowds | the dig gate on the nest census, back to the face, `FaceTrip` |
| Nurse | larva hunger scent, with food in the crop | larvae in reach fed, or crop empty | up the larva scent | crop nursing (on); `NURSE` and nurse seek (off) |
| Store | food in the crop beyond the ant's own need, inside the nest | food put down beside food | toward food lying inside, summed in reach, so piles grow where food is | storeroom `pile`; `NEST_STORE` (off) |
| Lay | the ant's bank above the lay bar | egg laid | toward brood, by touch and larva scent; lays less where larva scent is strong | walk home to lay, lay at the pile |
| Idle | nothing passes its threshold | a need or a job takes over | kinesis: slow and stop where nest air is high, it is dark and nestmates rest; inward along tunnels by the nest-air slope | `NEST_REST` (off), the nest-worker leash |

**Hysteresis.** Once on a job, the ant stays while its stimulus is above half its threshold (the half is a gene) and for at least a short dwell, so a stimulus near the line does not make it flicker.

**No fixed caste.** Today one ant in four is a nest worker for life, chosen by its id. Here thresholds differ between ants (genes) and shift with age: young ants start with low nurse and dig thresholds, old ants with a low forage threshold. Tracked colonies show workers moving from one group to the next as they age (Mersch, Crespi & Keller 2013, Science 340:1090, [doi 10.1126/science.1234316](https://doi.org/10.1126/science.1234316), via PubMed). The age schedule is fixed at first, not a gene.

**No colony counts.** The dig stimulus reads crowding at the head, which is local; nothing here reads a census. The forage stimulus is already local (`met`), in line with colonies that regulate foraging "through a network of local interactions" (Gordon 2013, Nature 498:91, [doi 10.1038/nature12137](https://doi.org/10.1038/nature12137), via PubMed). The only global knowledge left is the `NestWay` stand-in, named wherever it is used.

## The steering law

Every heading the ant can take gets a score, and the walk picks one with `choose_weighted`, as the shipped walk does:

**score(heading) = persistence × route hold + Σ weight(drive, term) × term(heading)**

Separately, a kinesis decides whether to step at all. The weights come from the winning drive: hunger weights the "out" terms, nursing weights the larva scent, idle weights almost nothing but persistence and kinesis. Which terms exist is fixed in code; how much each counts for each drive is a gene.

| Term | What it reads | Kind | Status |
| --- | --- | --- | --- |
| Persistence | the turn: going on 1, turning round 0 | body | shipped |
| Route hold | trail presence where the step lands, counted above the weakest option, so even fog holds nothing | route | shipped, with `CARRY_HOME=turn`'s floor |
| Away from home on a route | presence × cos(heading, away from home) | route and memory | shipped (`trailaway`) |
| Home vector | cos(heading, last nest contact) | memory | shipped (`forage_anchor`) |
| Door vector | cos(heading, the last open-air cell it stood on) | memory | new: the inside version of the home vector; 0 for an ant born inside that has never been out |
| Up or down | the heading's vertical part | gravity | new |
| Wall following | keep the wall on one side when the way the drive wants is blocked | touch | new |
| Field slope | the direction a short-range scent gives: larva hunger, alarm, building scent; nest air in tunnels | gradient | `larva_scent` shipped; the rest new |
| Crowd at the step | bodies in the cells the step lands in | touch | new as a term; `Crowding` exists as a sense |
| Scouting | level cos(heading, away from home) × hunger × (1 − presence) × patience | memory | shipped |
| Kinesis (step or stay) | nest-air level, dark, resting nestmates touched | level | new; `under_cover` and `Crowding` stand in |

Wall following has a basis in real ants: a weak individual tendency to follow walls, amplified by trails (Dussutour, Deneubourg & Fourcassié 2005, Proc Biol Sci 272:705, [doi 10.1098/rspb.2004.2990](https://doi.org/10.1098/rspb.2004.2990), via PubMed).

**Starting weights.** All are genes, set by design and then checked by trace:

| Drive | Main terms |
| --- | --- |
| Hunger, inside | up 1, door vector 1, wall following 1, food in reach 2, nest-air slope down 1 (`NestWay` until then) |
| Hunger, outside | route hold, away on a route 1, scouting 2, food in reach 2 |
| Escape | up 1, door vector 1; digging allowed into the heading's soil cell |
| Forage, going out | route hold, away on a route 1, scouting 2, the door reader 6 |
| Forage, carrying home | route hold, home vector 1 scaled by patience |
| Haul soil | up 1, door vector 1, wall following 1, building scent near the heap 1 |
| Dig | face vector 1, down 0.5, crowd at the step −1 |
| Nurse | larva scent 1 |
| Lay | larva scent 1, brood by touch |
| Idle | persistence only, kinesis on; inward along tunnels by the nest-air slope 0.5 |

The gains already shipped keep their values (`HOME_GAIN` 1, `TRAIL_GAIN` 3, `AWAY_GAIN` 1, scouting 2, `FOOD_TRAIL_GAIN` 6). New terms start at 1, the size of the home pull, and are switched on one at a time.

**Where no cue exists, a term is 0, never a guess.** No presence, no scent in reach, a slope under its read threshold, or a sensor sample landing in soil all give 0. With every cue silent, persistence and the pick's noise make a correlated random walk, which is the scout.

**Why this is harder than it sounds** (Scott's point, 2026-10-06). A weighted sum has known failure modes, and each needs its own defence:

- **Terms that cancel trap the ant.** Up against a door vector pointing down, or a larva scent across a wall, can leave no heading that gains. The defence is the shipped one, patience: when a drive stops making progress its weights fade, and wall following and noise rise until progress resumes. The laden ant's patience and the scout's give-up already work this way.
- **A slope exists only where a field has one.** Rooms have none (*Scents and fields*). So the room behaviours use level and touch, and every gradient term says where its slope was measured.
- **Ants that read a field they also write change it.** Idle ants stopping where nest air is high are themselves its sources, so they can pile up wherever the first few stopped, the door included. The first check of the new walk is where idle ants cluster, by picture and by depth, before anything is built on it.
- **Saturation silences a term.** Presence over the mound is 0.98-0.99 on every heading. Counted above the weakest option it holds nothing there, which is what lets the home vector work.
- **Weights can become the new rules.** Ten drives by eleven terms is the same hand-tuning moved into a table. Each drive uses at most five terms, starting weights come from gains already measured, and the robustness score in *Measurement* flags any weight a result hangs on.

**Every term carries a physics card before it ships:** who writes the cue, whether its slope was measured where it is used, its read threshold against the storage step, where the sensor samples, and what happens when the ant writes what it reads. A term without a card does not ship.

## Scents and fields in depth

"Go up or down this scent" is only as good as the scent, and most of our scents have no usable slope where the ant would need one. This section gives the evidence in three parts: what the trail loop already taught us, a new measurement of whether a nest gas would have a gradient in today's nests, and each cue the new ant would use, with what it can and cannot steer.

**The upshot:** a gradient term is one of several kinds of steering term, used only where a gradient has been measured. Inside a room the ant is held by a *level* (how stuffy, dark and crowded it is here), not steered by a slope.

### What the trail loop taught us

From the repo's own measurements, surveyed 2026-10-06. The full list with a source on every claim is `needs-ant/trail-lessons-2026-10-06.md`.

1. **Who writes a field sets its slope.** Only laden ants walking home lay the food trail, so its fresh end is the nest end: it read +0.04 to +0.23 toward home along the whole route. An ant climbing it walks home. The test bed's hand-laid ramp rose toward the food, so a "climb the scent" reader would have passed the bed and failed in the game.
2. **A trail has no direction.** Presence reads the same both ways along a route. Direction came from the ant's own sense of home: an away-from-home term took ants across the lattice scene 24 of 24, at a median 297 decisions, against 21 of 24 at 1,189.
3. **An ant's own mark blinds its nose.** The mark lands at its head, so "here" is always its freshest deposit: facing home read positive on only 1.2-8.2% of laden ticks. Making the plane four times more readable moved round trips 47 to 48. What worked was a path-integration input: completed returns 6.8% to 28.1%, better on 8 of 8 seeds.
4. **The blend, not evaporation, sets a trail's life.** On a one-cell line the 3x3 blend takes 16.7% a pass against decay's 2.9%. An unreinforced trail lived 144 frames against a 2,200-frame round trip: "a live map of where ants are, not a memory" (`pheromone.rs`).
5. **Storage runs out at the faint end.** On 8 bits the reading was exactly 0 at 0.7 and 0.9 of the way out. Sixteen bits took trail life from 144 to 1,476 frames.
6. **Saturation erases information.** Over the spoil mound trail A reads 0.98-0.99 on every heading, so the trail hold multiplies every heading about four times and the pull home loses: 84-89% of carries lost the pull home at least once. At the door, presence read 0.99-1.00 whatever the news.
7. **Where the nose samples matters.** The 6-cell sensor landed in sky or rock on 6 of 8 headings. The honesty gate and row projection took closed laps from 144 to 376.
8. **Ants reading a field they also write change the field.** Wiring "scent rising under me" into stepping tethered empty ants to the nest (reached food 199 to 156). A two-point reader changed the trail the ants then laid (+0.06 to -0.016). The repo's rule: size any change to how ants read a field against the field they will then make.
9. **Lifetime is wrong both ways, and depends on the food patch.** Too short and no road forms. Too long and dead side paths stay lit: food 104,340 to 82,820, lower on 8 of 12 seeds. Stale trails recruit to food that is gone, which a 700-frame news window fixed.
10. **Instruments lied three times:** a polarity metric that counted one shoulder of a blob, a `filmstrip` that never steps the planes, and an overlay that drew a 250-to-50 ramp as flat.
11. **The planes ignore terrain.** Scent spreads into rock and sky and leaks between tunnels. Nobody had measured how much that matters. The next part does.

### Would a nest gas have a gradient? Measured on today's nests

I solved the steady state of a gas breathed out by ants (1 each) and brood (0.25 each), spreading through open cells and escaping to open sky. It ran on 12 real maps from the shared baseline (main 3ba1e7bd5, seeds 1-4 at 100k, 200k and 295k), with sources averaged over 10k frames of ant positions, under three wall rules: sealed, leaking 2%, and ignored (what `pheromone.rs` does today). Then for every dug-nest cell I asked what an ant's 6-cell sensor would read, and where following the slope leads. Scripts, data and a positive control are in `needs-ant/gradient-check/`.

| Field | Sensor reads 1% or more | Down the slope reaches open sky | Down the slope shortens the walk out | Slopes into soil | Whole room varies |
| --- | --- | --- | --- | --- | --- |
| Nest air, sealed walls (7 maps with a way out) | 9% of cells \[3-16%\] | 99.6% | 96% | 0% | 6% \[3-9%\] |
| Nest air, walls leak 2% | 27% \[22-41%\] | 8% \[0-16%\] | 61% | 22% | 12% \[8-26%\] |
| Nest air, walls ignored (today's planes) | 100% | 0% \[0-3%\] | 19% | 17% | 58% \[47-64%\] |
| Larva scent, fades within 10 cells | 100% | n/a | n/a | 0% | climbing it ends at the brood for 85-100% of cells |

Medians over the 11 maps that had a way out, range in brackets.

&#91;image: Inside the nest, seed 1 at 200k. Top: each field on the nest's own scale. Bottom: green where a 6-cell sensor reads at least 1%, grey where it reads flat, red where the field slopes into soil.\]

&#91;image: Seed 3 at 295k. The only way out passes a corner gap, so a face-to-face gas calls this nest sealed although ants walk out of it in 21-66 steps.\]

What it shows:

- **Sealed walls give the right shape and almost no slope in rooms.** Down leads out and agrees with the shortest walk out; up leads deep (26-48 rows under the ground). But a room varies only 3-9% top to bottom, so the sensor reads under 1% on about nine cells in ten. The slope lives in the shaft and the tunnels. This is physics, not our numbers: Cox & Blanchard predicted the same for real nests, "a plateau of high concentration in the back half of the nest; an intermediate region of increasingly steep gradient towards the entrance; and a steep linear gradient in the entrance tunnel" (2000, J Theor Biol 204:223, [doi 10.1006/jtbi.2000.2010](https://doi.org/10.1006/jtbi.2000.2010), via PubMed).
- **Any leak through soil points the slope at the roof.** At 2% leakage, 22% of nest cells slope into soil and only 8% of descents reach open sky.
- **Today's planes make it worse, not better.** Ignoring walls gives a readable blob centred on the brood. It is readable everywhere, but no descent reaches the sky, only 19% of steps shorten the walk out, and at the door the field slopes away from the door. An ant "following nest air out" would be walked to the room's side walls.
- **The field must connect cells the way the walk does.** On 4 of 11 maps the nest's only exit is a corner gap between two soil cells. Face-to-face exchange calls the nest sealed while ants walk out. On one map (seed 4 at 200k) the door was shut and no slope to an exit existed at all, so the escape need cannot lean on one.
- **It must be solved, not stepped.** At today's plane rate the leaky version needs 492,000 frames to settle within 10%, and the sealed one did not settle in 1,500,000. The stepper is right (a 5-wide, 40-deep shaft settles in 222,000 against 218,000 predicted); the room is a reservoir behind a long, thin exit. Simple relaxation took 1,710-5,375 sweeps per refresh even warm-started, over about 1,000-1,400 open cells, because 33-68 cells open or close every 1,000 frames, mostly in the mound. A direct solver is the realistic route; its cost is unmeasured in Rust.
- **A short-range scent from a source works.** A larva scent that fades within 10 cells is readable on every nest cell, and climbing it ends at the brood for 85-100% of them.

### Four ways to make a field

| Way | Today | Good for | Fails at |
| --- | --- | --- | --- |
| Laid and spread on a plane | trails A and B | marks that fade unless traffic renews them: routes | direction, long range, rooms; leaks through walls; blinds the layer |
| A gas solved over open cells | none (new) | in versus out along tunnels; how deep you are, from its level | slope inside rooms; corner gaps; shut doors; solver cost |
| Summed from sources in reach, when asked | `larva_scent` (13x13 box, inverse square), the alarm's falloff | short-range cues: hungry larvae, alarm, building scent | anything past its reach; ignores walls, which is tolerable at 6 cells |
| A map built by search | `NestWay`, the dug-home and soil-way maps (breadth-first from the door) | a slope everywhere the walk reaches; no settling, no leak | it is knowledge no ant has, so it is a labelled stand-in |

### Each cue the new ant would use

| Cue | Made by | A slope where the ant needs it? | What it can steer | Status |
| --- | --- | --- | --- | --- |
| Food trail (B) | laden trip-load carriers walking home; plane, lab fade 0.03, blend 0.05 | across its width only; along it, it slopes toward the nest | staying on a route; the door read (east against west) | shipped |
| Home trail (A) | ants walking out; plane, no fade but the blend | no: saturated over the mound | little in the nest; homing comes from the home vector | shipped |
| Nest air, its level | bodies and brood | not needed: the level itself says how far in | resting and staying: slow and stop where it is high (kinesis) | new; `under_cover` is a crude stand-in |
| Nest air, its slope | the same | tunnels and shaft yes, rooms no; breaks at corner gaps and shut doors | in and out along tunnels | new; `NestWay` is the stand-in |
| Larva hunger | hungry larvae | yes, within 6 cells | nursing, and (later) laying less where it is strong | shipped (`larva_scent`) |
| Building scent on soil | soil an ant has placed | yes, short range | where to drop soil and dig | new (Khuong et al. 2016) |
| Alarm | wounded or fighting ants | yes, short range, an event | defence | shipped |
| Nestmates by touch | bodies at the head | contact, no slope | resting clusters; crowd avoidance at a face | shipped (radius 2) |
| Home vector | the ant's own memory of its last nest contact | always defined; noisy, reset at the nest | direction on routes and back to the door | shipped (`forage_anchor`) |
| Gravity | the world | always | up toward the surface, down to dig deeper | read by footing only |
| Walls | touch | contact | following a wall to an exit from a room | not used |
| Dark and cover | the world | a level | staying under ground | shipped (`under_cover`) |

### What this means for the design

1. **A gradient term only where a slope was measured:** larva scent near brood, alarm, nest air in tunnels. Each new field gets a gradient check like the one above, on real maps, before any ant reads it.
2. **Inside rooms the ant is held by a level, not a slope.** An idle ant slows and stops where nest air is high, it is dark, and it touches resting nestmates (a kinesis). Ants are known to aggregate in the dark and by attraction to each other, and brood-tenders aggregate even under light (Depickère, Fresneau & Deneubourg 2004, J Insect Physiol 50:629, [doi 10.1016/j.jinsphys.2004.04.009](https://doi.org/10.1016/j.jinsphys.2004.04.009); 2008, 54:1349, [doi 10.1016/j.jinsphys.2008.07.013](https://doi.org/10.1016/j.jinsphys.2008.07.013), via PubMed). This is the mechanism for "ants live in the nest", and it needs no gradient at all.
3. **Routes stay presence plus direction from the home vector**, as the shipped walk already does. Real ants also get direction from path integration (Müller & Wehner 1988, PNAS 85:5287, [doi 10.1073/pnas.85.14.5287](https://doi.org/10.1073/pnas.85.14.5287)) and from the angles where trails branch (Jackson, Holcombe & Ratnieks 2004, Nature 432:907, [doi 10.1038/nature03105](https://doi.org/10.1038/nature03105)), both via PubMed.
4. **Leaving a room uses up, the walls and the home vector**, then the nest-air slope once in the shaft. Until nest air exists, `NestWay`'s slope stands in, and every rule that uses it says so.
5. **Escape never assumes an exit exists.** Shut in means dig: up, and toward the last remembered way out.
6. **Nest air is built with a direct solver**, over the walk's own connectivity (corner gaps included where the walk passes them), with sealed walls, refreshed on a slow schedule. Its frame cost is measured before any ant reads it; if it is too dear, the stand-in stays.

## Acts

The acts are the engine's own mechanics and do not change: eating, the dig cut and the pellet it makes, picking up and putting down, sharing, laying, biting. What changes is who decides. Today the brain gives the odds of each act and the ladder and hand rules gate them. In the new walk the winning drive decides, from what is in reach:

| Act | When |
| --- | --- |
| Eat | hunger above its onset, with food in the crop, the jaws or at the head |
| Drop | a need wins while the jaws hold something (soil where it stands, or packed into the wall; food eaten if hungry, else put down); haul at a heap, drawn by building scent; store beside food inside |
| Pick up | dig: the pellet its own cut makes; haul: loose soil at its head inside; forage: food at its head |
| Dig | the dig job at its face while crowding there is above threshold; escape at any energy, into the soil cell of the chosen heading |
| Share | nurse: to a hungry larva it touches; any ant: to a begging nestmate when its own crop is above its hunger onset, weighing the adult's hunger against the larvae's (audit loop L7) |
| Lay | the lay job at the brood, less often where larva hunger scent is strong (audit loop L1) |
| Lay trail | food trail B only while carrying a trip load and closing on home, by the carrier's own patience (audit loop L3a); trail A as today |
| Bite or fight | later, with defence; today's alarm and bite stay as they are |

The audit's rule holds for every row: an act's gate may delay a need only if it names a floor above death.

## Mapping today's rules

Every pull and every walk switch either has a place in the new walk or is dropped with a reason.

| Today | In the new walk | Kind |
| --- | --- | --- |
| Store trip (rung 1) | the store job; yields to hunger, which eats the store load | job |
| Walk home to lay (rung 2) | the lay job, toward brood by touch and larva scent | job |
| Nest-worker leash (rung 3) and the fixed caste (`is_nest_bound`) | thresholds that differ by gene and age; idle kinesis keeps inside workers inside | dropped |
| Soil out (rung 4): `soil_way_pull`, `spoil_haul_target`, `SOIL_WAY`, `WAY_GAPS` | the haul job: up, walls, door vector, `NestWay` slope | job |
| Back to the face (rung 5): `dig_return_target`, `FACE_TRIP` | the dig job's face vector, with no `only` refusal | job |
| Hungry walk home (rung 6, off) | hunger: food in reach | need |
| Laden carry (rung 7) | the forage job's carry home: home vector and route hold | job |
| Hungry out (rung 8), the door throttle (`outward_want`), `LEAN_FORAGE` out and no-dig, `LEAN_LINE` | hunger, graded, with no zone and no cliff; the colony's want reaches an ant only through its forage stimulus | need |
| Mound out (rung 9, `MOUND_OUT=way` and `dig`) | hunger's out terms, and escape's dig everywhere | need |
| Rest pull (rung 10, `NEST_REST`) | idle kinesis | default |
| Scout, away term, door reader, give-up, `noreturn` | kept as terms, with their gains and patience logic | term |
| Heap cue at floor 0 (`spoil_cue_factor`), `DOOR_REOPEN` | building scent draws drops; nothing refuses an escape dig | term |
| The nest census in `Crowding` (`NestRoom::occupancy`) | crowding at the head | sense |
| Door scent read anywhere in a 33x33 box | the ant's own sensors | sense |
| `NestWay`, the dug-home and soil-way maps | kept as labelled stand-ins for the nest-air slope | stand-in |
| `HOME_SEARCH`, `CARRY_HOME` (off) | the carry home's route hold counted above the weakest option, tested afresh | term |
| `CROP_DOWN`, `EGG_DOOR`, `BROOD_CARRY`, `STORE_CHAMBER`, `NURSE`, `NEST_STORE`, `FOOD_BRAKE`, `DIG_MODES` (all off) | asked again as threshold questions in the nurse, store, lay and dig jobs, not ported as rules | job |
| `HAUL_BITE` | hunger eats from a load in the jaws | need |
| `WATER_FOOTING`, `GRASS_REGROW`, footing, sliding, pellets | unchanged | world |
| The brain's outputs | not read under the new walk; the brain's genes ride along unused | dropped for this walk |

The ten rungs and the switches above become two needs, seven jobs counting idle, and eleven terms.

## Genes

**What is heritable**, per ant, in a table of its own (about 45 genes):

| Group | Genes | Starting values |
| --- | --- | --- |
| Need curves | hunger onset and full; how fast escape builds | 0.6 and 0.25 of the grant; escape's rate from the traced `MOUND_OUT=dig` spells |
| Job thresholds | one per job, and the hysteresis half | set so that, at today's stimuli, the share of ants on each job matches today's traced shares |
| Term weights | per drive, only the terms it uses (about 30) | the table in *The steering law* |
| Kinesis | its strength and the level where it acts | from the first idle-cluster check |
| Shared walk genes | persistence, the patience leak, the six shipped walk gains | today's values (`WALK_GAIN_SLOTS`) |

**Kept apart from the shipped genome.** The shipped ant has 27 trait slots, all in use, and every birth draws from them. The new genes live in their own table, read only under the new walk, so the shipped ant's random stream, and every off-arm run, stay bit-identical.

**Mutation off while measuring** (`mutation_off_for_measuring()`), as for every lane. It is turned on only for an evolution round after the walk has passed its acceptance; the weights and thresholds are then what evolution tunes. Mechanism is code and policy is genome (owner ruling on the breeding plan, §2a).

**Age.** Thresholds shift with age on a fixed schedule at first (young: nurse and dig low; old: forage low). Making the schedule a gene comes later.

## Measurement and acceptance

Each designed behaviour has its own metric and its own trace. Colony size is a floor, never the target (Scott, 2026-10-06).

| Designed behaviour | Metric | Trace |
| --- | --- | --- |
| Ants live in the nest | share of ant-time in the dug nest (the scorecard headline); share of ants spending half their time or more inside | where idle ants stop, by depth and picture |
| Separate chambers | rooms of 30 cells or more joined by tunnels, counted and pictured | each cut, with crowding at its face, per room |
| Food apart from brood | food cells inside that do not touch brood; store held over time | each store drop and what lay beside it |
| Foraging follows supply | food taken; departures that follow returns; departures that stop when a pile runs out | per trip: drive, route terms, door read |
| Brood care | larvae starved per egg; eggs per food brought home | each lay, with the larva scent at the pile |
| Needs work | hungry ants with no positive weight toward food or out; shut-in spells ending starved; deaths by place (`starvewhere.py`) | the decision before every death: drive, job, load, place |

**The no-veto guard** (the audit's §5, built first). For every job, every load and every place (door zone, mound, the shaft below the founding ground, a sealed pocket), an ant at 30% of its grant must, within a bounded number of ticks, have eaten or dropped its load and have positive weight toward food or out, or be digging if sealed. Each row is proven by putting a veto back and watching it go red.

**Comparison.** Paired seeds against the shipped walk on the same build: 1-4 seeds while iterating, 12 seeds to 300k for any claim that the new walk is better. Switching on at a frame (*Build order*) makes the two arms the same world up to the switch. The shipped arm reuses the shared baseline; no duplicate runs.

**Robustness score.** Four seeds, plus each drive's two most important weights at x0.8 and x1.25, to 300k. A result is fragile if any perturbed run collapses or flips the verdict.

**Frame cost.** Measured with `examples/ascii`'s worst frame and the lab's phase timings, paired, at fixed threads. The budget is Scott's call (open question 4); the reference is today's chooser at about 9% of the ant scene's frame.

**Fields.** Every new field passes the gradient check on real maps before any ant reads it, and every term carries its physics card.

## Build order

Each slice lands on main switched off, is checked bit-identical when off, and has a gate Scott sees before the next one starts.

0. **Instruments.** The gradient check as an example binary over Deep trace maps; trace columns for drive, job and terms; the no-veto guard harness. Gate: the positive controls pass (the shaft's known settling time; a veto put back goes red).
1. **Needs and idle, switched on at a frame.** The colony founds under the shipped walk, and at a chosen frame (say 50k) every ant switches to the new walk, so both arms are the same world up to the switch. This slice has hunger, escape, idle kinesis, and foraging with today's route terms. Hauling and digging stay the shipped rules, wrapped as jobs, so needs already outrank them. Gate: the no-veto guard is green; idle ants rest inside (time in the dug nest above the shipped walk's on 4 seeds); starvation no worse.
2. **Haul, dig and escape as jobs.** Crowding at the head drives the dig, escape digs anywhere, and dropped soil carries building scent. Gate: shut-in spells ending starved at or under the shipped walk's; door seals; any second room, by count and picture.
3. **Nurse, lay and store.** Larva-scent nursing, laying keyed to larva hunger, storing beside food. Gate: larvae starved per egg; store held over time.
4. **Nest air (optional).** A direct solver over the walk's own connectivity, refreshed slowly, replacing the `NestWay` stand-in where it passes. Gate: the gradient check on live runs, frame cost, and no loss on the metrics above.
5. **Genes on.** An evolution round with mutation, only after slices 1-3 pass.

Kill points come after slices 1 and 2 (*Risks*).

## Risks, failure modes and kill criteria

| Risk | What it would look like | Defence |
| --- | --- | --- |
| The sum traps ants where terms cancel | still spells and revisits in one spot | patience fades the drive's weights; wall following and noise rise |
| Weights become the new rule list | a table nobody can predict, tuned per symptom | at most five terms per drive; starting values from measured gains; the robustness score |
| Idle ants pile up where the first few stopped | one dense heap at the door or the brood column | the first check is where idle ants stop, by picture; kinesis capped by crowding |
| A field has no slope where a term needs one | ants dither in rooms | gradients only where measured; rooms use level and touch |
| Stand-ins become permanent | `NestWay` forever | named where used; nest air is slice 4 |
| It repeats Nest building's hunger-first work | two teams fixing the same vetoes | different scope: Nest building fixes the shipped ant now; this walk is judged on designed behaviours and rule count |
| It breaks off-arm runs | the identity check fails | new genes in their own table, no random draws while off, the identity check at every landing |
| Adding a walk enrols it in every rule over walks and species | a test that should fail passes | check every `match` on the walk and every iterating test at the hook; run `cargo test`, not `--lib` |
| It takes a long time | a thread busy for weeks | slices with kill points; nothing else waits on it |

**Kill criteria.** Scott decides at each kill point; these are proposals.

- **After slice 1** (4 seeds to 300k): idle ants do not rest inside (time in the dug nest under 10%, against about 5% for the shipped walk, where the goal is most), or starvation is worse on 3 of 4 seeds after one round of fixes.
- **After slice 2:** shut-in deaths are not at or under the shipped walk's, or no room beyond the first ever forms.
- **At any point:** the walk has needed more than about ten special cases outside the needs, jobs and terms structure to pass, which would mean it has become the ladder again; or its frame cost is over budget.

If it is killed, the hook commits are reverted and the module deleted. Nothing measured elsewhere changes.

## Open questions for Scott

1. **Switch on at a frame for slice 1?** The colony founds under the shipped walk and switches at, say, 50k, so needs and resting are tested before digging is ported. Recommended: yes.
2. **`NestWay` as a labelled stand-in** for "which way is out" inside the nest, until a nest-air field passes its check? Recommended: yes.
3. **Resting by kinesis.** Idle ants slow and stop where it is dark, stuffy and crowded with resting nestmates. Is that the resting you want, or should resting ants also seek particular rooms, away from brood for example?
4. **Frame budget** for the new walk and its fields.
5. **Rule budget as a kill criterion** (no more than about ten special cases outside the structure): the right kind of test?
6. **Go-ahead.** A new Opus thread builds it, starting with slice 0, only on your yes. No lane is diverted.

## References

Checked via PubMed on 2026-10-06 unless marked.

- Bonabeau, E., Theraulaz, G. & Deneubourg, J.-L. (1996). Quantitative study of the fixed threshold model for the regulation of division of labour in insect societies. Proc R Soc B 263:1565. In the project's references; not found in PubMed here.
- Charbonneau, D., Sasaki, T. & Dornhaus, A. (2017). Inactive workers as a reserve labour force. PLoS ONE 12:e0184074. [doi 10.1371/journal.pone.0184074](https://doi.org/10.1371/journal.pone.0184074)
- Cox, M. D. & Blanchard, G. B. (2000). Gaseous templates in ant nests. J Theor Biol 204:223-238. [doi 10.1006/jtbi.2000.2010](https://doi.org/10.1006/jtbi.2000.2010)
- Depickère, S., Fresneau, D. & Deneubourg, J.-L. (2004). The influence of red light on the aggregation of two castes of the ant, *Lasius niger*. J Insect Physiol 50:629-635. [doi 10.1016/j.jinsphys.2004.04.009](https://doi.org/10.1016/j.jinsphys.2004.04.009)
- Depickère, S., Fresneau, D. & Deneubourg, J.-L. (2008). Effect of social and environmental factors on ant aggregation: a general response? J Insect Physiol 54:1349-1355. [doi 10.1016/j.jinsphys.2008.07.013](https://doi.org/10.1016/j.jinsphys.2008.07.013)
- Dussutour, A., Deneubourg, J.-L. & Fourcassié, V. (2005). Amplification of individual preferences in a social context: the case of wall-following in ants. Proc Biol Sci 272:705-714. [doi 10.1098/rspb.2004.2990](https://doi.org/10.1098/rspb.2004.2990)
- Gordon, D. M. (2013). The rewards of restraint in the collective regulation of foraging by harvester ant colonies. Nature 498:91-93. [doi 10.1038/nature12137](https://doi.org/10.1038/nature12137)
- Jackson, D. E., Holcombe, M. & Ratnieks, F. L. W. (2004). Trail geometry gives polarity to ant foraging networks. Nature 432:907-909. [doi 10.1038/nature03105](https://doi.org/10.1038/nature03105)
- Khuong, A. et al. (2016). Building pheromone on soil, whose lifetime shapes nest architecture. PNAS 113:1303. [doi 10.1073/pnas.1509829113](https://doi.org/10.1073/pnas.1509829113)
- Mersch, D. P., Crespi, A. & Keller, L. (2013). Tracking individuals shows spatial fidelity is a key regulator of ant social organization. Science 340:1090-1093. [doi 10.1126/science.1234316](https://doi.org/10.1126/science.1234316)
- Müller, M. & Wehner, R. (1988). Path integration in desert ants, *Cataglyphis fortis*. PNAS 85:5287-5290. [doi 10.1073/pnas.85.14.5287](https://doi.org/10.1073/pnas.85.14.5287)
- Robinson, E. J. H. et al. (2005). A 'no entry' signal in ant foraging. Nature 438:442. [doi 10.1038/438442a](https://doi.org/10.1038/438442a)

**Project and repo sources**

- The rule audit: `rule-audit/rule-audit-2026-10-06.md`
- What the trail loop taught us, with a source per claim: `needs-ant/trail-lessons-2026-10-06.md`
- The gradient check, its scripts, data and positive control: `needs-ant/gradient-check/README.md`
- The shared baseline: `deep-trace/baseline/3ba1e7bd5/summary.md`
- `Reports/how-the-ant-works.md`; `src/sim/pheromone.rs` (constants and their docs); `src/sim/brood.rs` (`larva_scent`); `src/sim/creature.rs` (`chooser_step`, `NestWay`, `under_cover`)
