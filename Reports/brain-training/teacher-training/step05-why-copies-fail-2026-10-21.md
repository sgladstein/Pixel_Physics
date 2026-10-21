# Step 0.5: why copied lessons fail

*2026-10-21, by the teacher-approach session. Status: **in progress** -- items 1-3 of the teacher design's §4 are
done; the revive-one-unit arms are running. Local, offline on step 1's logs plus world runs on the 96 evaluation
worlds (seed-clustered s.e.). Scratch code: `contextprobe.py` and the copier's `arena_bit`, `all_slots`,
`revive_units` options (`scratch/teacher-probes`, never for main).*

## The answer so far

**Dig fails to copy because the brain's live wiring cannot hold it, not because a sense is missing. Walk home fails
for a different reason: the information to keep it out of foraging is there, and the copy does not use it.**

| Lesson | What the copy did in its own runs | The cause | What helps |
|---|---|---|---|
| **Dig at the foot** | Dug at the foot when there (0.17 -> 0.69), but dug the wall *more* (0.0135 -> 0.0163 a decision), so foot cuts fell 7.2 -> 4.1 a world and wall cuts rose 10.5 -> 14.2; total cuts unchanged (22.5 / 22.2), so each extra wall cut took a foot cut's place | **The verb stage cannot see "a foot cell is in reach".** Dig's job reads its best option score with weight -0.31 against a bias of -6.30, and the best dig score itself separates foot from wall at AUC 0.613. To copy the lesson the copier must build both at once; each one's gradient is near zero while the other is | **More wiring.** With every slot free the copy digs the foot 0.89 and the wall 0.0111 -- *below* the start -- in its own runs. But its 1,874 live links run it out of energy (0.08 by frame 8,000-12,000, against 0.26), so in the world it scores 4.40, not more. One revived unit is the test now running |
| **Walk home** | Spread to empty foragers and broke fetch (16.66 -> 11.46) | **Not a missing sense.** The context probe separates "empty digger walking home" from "empty forager walking" at AUC 1.000 from senses and memory alone -- but through arena artefacts (fetch is fed, so energy reads exactly 0.500; without energy, still 0.97 via run length, `at_nest` and depth). The copy did not use the separation | The arena bit (an offline counterfactual) lets the soil lesson work better (8.05 against 7.03) but breaks fetch worse, because the bit does not exist in the real fetch arena. More wiring alone: no better in the world (6.40) |

## 1. The dig copy's collapse, traced

The dig-only copy (step 1's `only-dig`, 3.35 full loads) and its start (s161 quieted, 6.42), each on the same 48
training-layout worlds with every decision logged (`copylog soil ... 9101 48`):

| | Start | Dig copy | Dig copy, all slots free |
|---|---|---|---|
| Full loads a world | 6.27 | 3.44 | 4.75 |
| Foot / wall / rim cuts a world | 7.2 / 10.5 / 4.8 | 4.2 / 14.2 / 3.8 | 5.4 / 8.5 / 1.7 |
| P(dig) with a foot cell in reach | 0.17 | 0.69 | 0.89 |
| P(dig) with only wall cells in reach | 0.0135 | 0.0163 | **0.0111** |
| Takes the deepest step when empty below ground | 0.22 | 0.26 | -- |
| Carry time, foot pellet (median frames) | 260 | 262 | -- |
| Put-down to next cut (median frames) | 245 | 245 | -- |
| Mean energy by 4,000 frames | 0.47 / 0.40 / 0.31 / 0.26 | same | 0.44 / 0.27 / **0.08** |

**What it rules out**: descent (the copy goes down as well as its start), carrying (identical), energy (identical for
the step-1 copy). **What it shows**: a trip down ends at the first cut, so the rate of wall cuts on the way down decides
how many trips reach the foot. Step 1's held-out log already said so -- on "don't dig here" states the copy's P(Dig)
went 0.0104 -> 0.0193 -- but it was read as "dig copies half". It copied the wrong way where it mattered.

**The withdrawn claim**: step 1's "the copy digs at the foot 80-91% of the time" was measured on *s161's* logged states.
In its own runs it reaches the foot far less often.

## 2. The joint-fit diagnostic

Each lesson copied three ways (design §4 item 2), scored on held-out logged decisions and then in the world:

| Arm | Walk home: lesson / kept skills (held out) | Walk home in the world: full loads, fetch | Dig: P(Dig) foot / wall (held out) | Dig in the world: full loads, fetch |
|---|---|---|---|---|
| (a) as in step 1 | 0.64 / 0.94 | 7.03, 11.46 | 0.79 / 0.0193 | 3.35, 18.07 |
| (b) every slot free, energy ignored (1,874 live links) | 0.87 / 0.95 | 6.40, 11.81 | 0.96 / 0.0079 | 4.40, 17.60 |
| (c) (a) + an arena bit (on `c::CROWD`) | 0.75 / 0.95 | 8.05, 5.97 | 0.82 / 0.0193 | 3.11, 13.44 |

(Start: 6.42 full loads, fetch 16.66; dig's start on held-out "don't dig" states 0.0104. Arm (c)'s bit borrows
`c::CROWD`, which reads 0 in every solo arena but has live links in s161 -- setting it moved s161's own fetch
agreement to 0.75 before training; and it does not exist in the real fetch arena, so (c)'s fetch number is not
deployable, only diagnostic.)

**Reading, by the design's own key**: for dig, only (b) fits both, so the wall is **capacity or wiring**; (c) does
nothing, because dig's spread is inside the soil arena. For walk home, (b) and (c) both fit both on held-out
decisions, and neither helps in the world: **agreement on logged decisions is not performance**, which the design's
own guideline T6 says and which this table shows twice.

## 3. The context probe (walk home)

`contextprobe.py`: 6,000 soil decisions the teacher labelled "walk home" against 6,000 fetch decisions of an empty
forager choosing Walk, held out by world.

| Inputs | AUC linear | AUC 16-unit |
|---|---|---|
| senses + memory | 1.000 | 1.000 |
| ... energy senses held at 0 | 0.974 | 0.977 |
| ... + mean Walk-option features (energy held at 0) | 0.982 | 0.983 |
| Positive control (energy above its median) | 1.000 | -- |
| Negative control (arena labels permuted by world) | 0.516 / 0.507 | -- |

Separating inputs: energy (fetch is fed: 0.500 every frame), then scent A, depth, age (the run clock: arenas differ in
length), `at_nest` (0.77 in fetch, 0.04 on the soil surface). **These are arena artefacts**: a brain that keys on them
learns "which arena am I in", which no real colony offers. The finding that matters for the brain lane: the kept-skill
arenas can be told apart from the soil arena by things that are not the world.

## What this changes

- **For the brain lane's copy-then-select test**: the copy handed over (go-down into s171, 11.64 full loads on my 96
  worlds) does **not** clear the trainer's fetch floor on the brain lane's harness (worse side of 48 mirrored pairs:
  17.62 against s171's 19.34 and a floor of 17.61) -- see step1-results; a copy anchored on more fetch states is being
  built. Go-down's channel was clean, so this is a fetch-anchor problem, not a copying one.
- **For dig**: test the cheapest capacity (one revived scorer unit, ~90 links) before anything else. If it holds the
  lesson without running the ant dry, dig copies; if not, dig is a lesson for selection with link protection.
- **For walk home**: neither a sense nor capacity fixes it in the world. Leave the empty return trip to selection
  (step 1's option (c)), or teach it only in a world where diggers and foragers share an arena, so the look-alike states
  carry both labels.
- **For the arenas**: fetch being fed makes energy an arena tag. Worth the brain lane's knowing before any kept-skill
  floor is trusted as "the same brain doing two jobs".

## 4. The narrowest capacity: one revived scorer unit

s161 uses 8 of its 16 scorer units. Arm (u1) lets the copier use one of the dead ones -- its 35 context and memory
inputs, its 45 option-feature inputs, its verb row and its output (`revive_units`). **First build, a null result to
keep:** the revived unit came out identical to arm (a) to every digit, because a unit with every weight at zero has an
exactly zero gradient on both sides (zero output weight to its inputs; zero activation to its output). Its inputs are
now seeded (sigma 0.3) with the output left at 0, so the start's policy is unchanged.

| Dig copy | Full loads (start 6.42) | Fetch (start 16.66) | Held out: P(Dig) at the foot / at a wall (start in brackets) |
|---|---|---|---|
| (a) step 1 | 3.35 | 18.07 | 0.79 / 0.0193 (0.24 / 0.0104) |
| (b) every slot free | 4.40 (runs dry) | 17.60 | 0.96 / 0.0079 |
| **(u1) one revived unit** | **5.80** (-0.61 +- 0.45) | 17.26 (+0.60 +- 0.92) | 0.88 / 0.0137 (0.17 / 0.0129) |
| (u2) two revived units | 5.91 (-0.51 +- 0.41) | 17.84 (+1.18 +- 0.88) | 0.84 / 0.0182 (0.25 / 0.0153) |

(The held-out split differs between arms -- the seeding draws from the stream that picks held-out worlds -- so the
start's own rates are given beside each.)

**One unit removes most of the collapse** (3.35 -> 5.80) with fetch held: the dig lesson no longer takes over the wall.
It does not yet *gain* alone, which is what the ceiling predicted -- dig and go-down are superadditive (dig alone +4.74
acting, go-down +3.41, both +10.11). The test that matters is the two together with one revived unit; it is running.

| Walk home copy | Full loads | Fetch |
|---|---|---|
| (a) step 1 | 7.03 | 11.46 |
| (u1) one revived unit | 7.82 (+1.41 +- 0.44) | 10.47 (-6.19 +- 1.13) |

A unit helps the soil lesson and does nothing for the spread into fetch -- consistent with the context probe: that
spread is not a capacity problem.

**Dig + go-down with one revived unit** -- the pair the ceiling says is superadditive:

| Copy | Full loads | Fetch (start 16.66) |
|---|---|---|
| go-down alone (step 1) | 10.57 | 17.11 |
| dig + go-down (step 1) | 9.32 | 11.50 |
| **dig + go-down, one revived unit** | **10.67** (+4.25 +- 0.43) | **17.74** (+1.09 +- 0.86) |

The revived unit removes the interference (fetch held, soil not below go-down alone) -- but dig still adds nothing on
top of go-down, where acting it adds +6.7. The wiring problem is solved; the dig lesson's own gain is still missing.
In held-out terms the copy now digs at the foot 0.86 and at the wall 0.0049 (start 0.17 / 0.0060), so it *does* the
lesson on s161's states; what to look at next is whether it reaches the foot in its own runs (§1's measure).

## 5. Routing (from the research lane's audit)

The research lane read s161's wiring and found **unit 14 is the brain's own empty-only unit**: `c::SPOIL` reaches it at
+31.7, so whenever the ant carries it sits saturated and flat (slope ~0.001), and it has no link from `f::HOME` (slot
1252 is 0). Confirmed on the start genome used here. So a walk-home lesson routed through unit 14 would not reach
loaded ants. Arms: (r1) slot 1252 alone on the allow-list; (r14) all 45 of unit 14's feature links. Neither can stop
the spread into *empty* foragers, which unit 14 serves too -- and they did not:

| Walk home copy | Full loads | Fetch |
|---|---|---|
| (a) step 1 | 7.03 | 11.46 |
| (r1) + slot 1252 | 7.31 | 11.43 |
| (r14) + all of unit 14's feature links | 7.67 | 10.26 |

## 6. The kept-skill floor: why the go-down copy fails fetch on some worlds

On the trainer's own fetch harness (96 worlds, worse side of each mirrored pair) s171 scores 19.34 and the go-down
copy 17.62; heavier anchors (10x, 20x, plus the copy's own fetch runs) did not help (15.76, 17.51). **The loss is a few
total failures, not a general slide**: s171 scores under 5 on 2 of 96 worlds, the copies on 5-12; away from those
worlds the 20x copy is level with s171 (+0.13 a world). Traced on one such world (`fetch-copy.log`, world 44, score
0.18): 661 of 675 decisions are walks, nearly all inside the nest (`at_nest` 1, `c::DEPTH` +0.27, the pre-dug chamber),
with brief trips to the surface. **Go-down has spread to the empty forager in the nest**: "empty, below ground" is the
lesson's trigger, and the forager keeps stepping deeper instead of climbing out. Under the worse-side fold the trainer
uses, a few such worlds decide the floor. Being tested: those look-alike decisions (an empty ant below ground, in
fetch) as their own heavily weighted anchor group (`keep_lookalike`).
