# Colony against colony: what two trained colonies do to each other, and a first tournament

*Colony-vs-colony thread, 2026-10-24/25. Owner's ask (22:28Z): "What about colony vs colony combat, are we ready to
start exploring that?"; then (23:27Z) "Are ants being selected for the most kills or how are we training the
combat?" and (23:32Z) "Won't this just select colonies that live in peace. That seems less interesting." Harness
`examples/rivalbed.rs`; tournament `scratch/inrun/rivaltour.py`, judge `scratch/inrun/rivaljudge.py`; data
`/mnt/project-files/colony-war/`.*

## The answer in five lines

1. **Two colonies of today's trained ant can already fight.** The shipped pool holds the bite switched on at a blank
   entry (inrun's `enter_bite` put it there), and the mind's bite acts on any ant of another colony.
2. **When both can bite they keep a cold peace**: ~24 bites and ~17 kills in 100,000 frames between two colonies of
   ~80, no measurable cost to food, nest or headcount. They rarely meet, and the other side can answer.
3. **Biting does not pay today.** A colony that can bite beside one that cannot kills ~150 of it (the other colony
   dies out in 10 of 24 runs) and gains nothing itself: its ants, food and nest are those of the no-bite pair.
4. **Selection against a rival does breed fighters.** After a 16-round tournament scored against the rival, champions
   beat the founder in 24 of 24 matches and kill 2-7 ants for each they lose; colonies bred with no bite are wiped out
   when they meet one that bites. **But the champions also stopped carrying food home and dug a wide pit**, because
   the first score paid ants alive and cuts and nothing else. A second tournament with food home and the nest's real
   size in the score (§5) bred better fighters and a real nest, and **still lost carrying home**: inside a match
   nothing rewards an ant for carrying, so the fix belongs in the Teacher lane's selection, where births go to
   carriers (§6).
5. **A wipeout is one ant, and the bite is binary.** One bite takes one cell ant on ant and a bitten head is a death
   (7 bites in 10 kill); in a nest that cannot bite back, a single raider kills 20-60. A graded bite (4 bites a cell,
   mouths adding up) gives a fight its middle when both sides bite.

## 1. The bed

`examples/rivalbed.rs`: the in-run training bed (`colony_bed::scenario`, 256 x 128, 40 rows of soil) with **two**
colonies founded heaped at nests `gap` columns either side of the centre (default 80: x = 88 and 168), 30 founders
each from `std_pool` (one per member), three full tanks each. Each colony is fed as inrun feeds its one colony: a
capped drip 25 columns either side of its own nest (`food=split`: each colony's outer heap plus one shared heap at
the centre). Breeding is the game's life cycle with inrun's lay cap (`48:8`) **per colony** through a new harness
hook, `World::colony_lay_scale` (§1a). A hatchling keeps its layer's mind unless `mutate=` is given. No selection
inside a run.

**Why not inrun:** inrun books every ant of the species as one colony -- its ledger, its judging and its culls -- so a
neighbour would be scored, bred and culled as kin. A second colony there is a change to the Teacher lane's harness;
this report is the case for making it.

Counters per colony: adults alive at the end, `lived` (mean adults over the run), food home (inrun's `deliver`: a trip
cell put down within 12 columns of its own nest), `field_eat` (cells swallowed away from home), nest cuts (original
ground at or below the datum within 20 columns of its own nest), `nest_deep`/`crater` (cells open or holding
provisions, joined to its door, 3+ rows down / the top three rows, at the end), killed / starved, kills of the other
colony (`World::kills_log`, which names both colonies), bites given (`World::encounter_log`), and `kills.csv`: every
cross-colony killing with the killer's place against both nests. **Specificity:** the no-bite and solo arms read 0
bites and 0 kills on every run. **Sensitivity:** the one-sided arms read 1,600 kills over 12 runs and the victims'
`killed` agrees.

### 1a. A harness flaw found and fixed on the way

The brood block's `lay_at` is one number per species, so the first two batches capped the bed's **total** heads. The
colony that grew first raised the bar for both and **stopped its neighbour laying**: in one tournament pair the east
colony grew to 131 on digging while the west sat at 46 with no births for 60,000 frames. It made one-sided biting look
like conquest (biter +40 to +68 ants) and gave the first tournament a free win to whoever grew first.
`World::colony_lay_scale` (None everywhere else, so unread) now scales the bar per colony; `lay_cap_mode=shared` is
the old rule and is byte-identical to the build before it. Every number below is per-colony unless marked.

## 2. Twelve seeds x 100,000 frames, paired by seed

Medians over 12 seeds (11 where marked), per-colony cap. `none` = both banned from biting; `west`/`east` = only that
colony can bite; `solo` = that colony alone on the same bed with both nests' food dripping.

| Arm | bites a run | kills a run | west alive / lived / food home / nest cuts | east alive / lived / food home / nest cuts | died out (W/E) |
|---|---|---|---|---|---|
| solo (11) | 0 | 0 | 83 / 72.3 / 8,915 / 948 | 83 / 72.9 / 8,435 / 911 | 0 / 0 |
| none | 0 | 0 | 86 / 71.8 / 8,576 / 882 | 82 / 72.8 / 8,738 / 914 | 0 / 0 |
| both (the pool as shipped) | 24 | 17 | 84 / 72.8 / 8,884 / 901 | 81 / 72.5 / 8,571 / 893 | 0 / 0 |
| both, nests 40 apart (11) | 221 | 148 | 75 / 70.7 / 8,038 / 848 | 80 / 70.0 / 9,332 / 890 | 0 / 0 |
| only west bites | 190 | 126 | 84 / 71.7 / 8,846 / 908 | **58** / 68.2 / 8,266 / 854 | 0 / **5** |
| only east bites | 198 | 138 | **28** / 65.5 / 7,779 / 827 | 83 / 71.8 / 8,845 / 887 | **5** / 0 |

Paired against `none`:

- **A neighbour costs almost nothing** while neither can bite (alone against beside one: every measure split ~6/6).
- **Both able to bite**: killed +8-9 per colony (12 of 12), everything else ~6/6. `bite=keep` is byte-identical to
  `bite=0,0` (every pool member already holds the bite at 0), and a more willing bite (job constant +1 on both sides
  or one, shared-cap batch) changed nothing: fighting is limited by meeting, not by willingness.
- **Nests 40 apart**: a standing border war, ~74 killed per colony, absorbed by breeding -- headcount, food and nest
  within a few percent, nobody dies out.
- **Only one side bites**: the bitten colony loses ~150 ants (12 of 12), alive -31 and -55 (11 of 12), food home lower
  11 of 12 when it is the west one; the biter's alive, lived, food and nest are within noise of `none` (~6/6). **The
  biter takes nothing from the kill**: the mind's bite swallowed no flesh (`flesh 0` in every run), the bed's food is
  each colony's own, and there is no ground to take. So nothing in today's bed would select for aggression.

### Why one-sided biting wipes a colony out: one ant, not an army

Traced ant by ant (`kills.csv`, 12 seeds each). With both colonies able to bite, kills are spread thin: the most kills
by one ant in a run is 2 (max 6), 15 different killers, and 3% of kills happen within 15 columns of the victim's nest.
With only one side able to bite, the most kills by one ant is **23.5 (max 64)**, and **76-83% of kills happen inside
the victim's nest**. Seed 3, west bites: ant 105 strays 100 columns east, past the other nest to its outer food, kills
there at frame 10,160, walks back into the east nest and kills 25 ants in 700 frames with a full crop and a full bank.
Nobody can answer it, and every bite is a death. (Shared-cap batch; the per-colony batch has the same shape.)

## 3. The bite has no middle

`contest::bite_progress` is `(bite / armour)^2`, which is 1 ant on ant: one bite takes one cell, and a bitten head is
a death. 221 of 332 bites killed (67%). The mind's bite also skips the game's own fight rules (`sim::contest`'s
assessment and withdrawal); whether to bite is the mind's. That is the binary outcome `CLAUDE.md`'s first law warns
against: a fight is either nothing or a kill.

**A graded bite** (`PP_MIND_BITE_SCALE=0.25`, scratch, unset = 1): a mind bite takes a quarter of a cell, banked on the
victim (`OrganismState::gnawed`), so four bites -- from one mouth or several -- take a cell. Same seeds, per-colony cap:

| Arm | bites a run | kills a run | died out (W/E) |
|---|---|---|---|
| both, bite 1 | 24 | 17 | 0 / 0 |
| both, bite 0.25 (11) | 119 | **11** | 0 / 0 |
| only west, bite 1 | 190 | 126 | 0 / 5 |
| only west, bite 0.25 (11) | 1,424 | 118 | 0 / 3 |
| only east, bite 1 | 198 | 138 | 5 / 0 |
| only east, bite 0.25 (11) | 320 | 20 | 1 / 0 |

When both sides can bite the graded fight has its middle: five times the bites, fewer kills (~1 bite in 10 kills),
and in the shared-cap traces no ant killed more than two. Against a colony that cannot bite it mostly changes the
count of bites: a lone raider bites four times as often. **Not proposed for the default yet**: it is the same scale
for the spider's bite (spider on ant is also ratio 1), which the predator lane has tuned around one-bite kills.

## 4. The first tournament: selection against a rival breeds fighters, and two cheats

`scratch/inrun/rivaltour.py`: 12 colonies from `std_pool`; each round they are paired at random and each pair plays two
matches (sides swapped, different seeds) of 60,000 frames on the contested bed (`gap=40 food=split`); every hatchling
is a mutant of its layer (`mutate=0.013:1:4`, inrun's standard operator), so each colony is under natural selection
inside the match too. A colony's match score is **its share of the pair's ants lived plus half its share of the pair's
nest cuts** -- a score against the rival, so two colonies at peace tie and whichever takes the other's ground wins.
The winner's survivors found both of the pair's colonies next round. 16 rounds, three arms: `fight` (the bite as
shipped), `graded` (bite 0.25) and `peace` (bite banned, the control).

Over rounds, kills per colony a match rose and then settled: fight 33 (round 0) -> 49-57 (rounds 2-7) -> 32 (round
15); graded 19 -> 35-46. Judged afterwards (`rivaljudge.py`, two champions an arm, 6 fresh seeds x both sides, no
mutation, against `std_pool`):

| Champions | Won (of 24) | share of ants lived | kills / killed (median) | food home: champion vs founder | nest cuts: champion vs founder |
|---|---|---|---|---|---|
| fight | **24** | 0.55-0.56 | 39-46 / 16-23 | 498-718 vs 4,646-4,796 | 4,983-5,726 vs 492-511 |
| graded (judged with bite 0.25) | **24** | 0.59 | 68-86 / 12-40 | 246-560 vs 4,439-4,642 | 3,640-4,118 vs 467-496 |
| peace (no bite) | 3 | 0.15-0.27 | 0 / 232-827 | 67-96 vs 5,029-5,124 | 724-2,356 vs 488-574 |
| fight against peace | **24** | 0.56-0.87 | 168-216 / 0 | 556-775 vs 78-170 | 4,714-4,956 vs 721-2,110 |

So **yes, selection against a rival breeds better fighters**: from a pool whose two colonies trade kills one for one,
16 rounds made colonies that kill two to seven for each they lose, and a line that never fought is helpless against
one that does (peace pools died out in 6 of 24 matches against the founder, 11 of 24 against the fight champion).
**And it bred two cheats**, both read off the score: the champions **stopped carrying food home** (500-700 against
~4,700 a match; they eat at the heaps, `field_eat`), and **dug a wide pit**: 5,000+ cuts a match, which on the map is
a ~15 x 15 bowl open at the top (`nest_deep` 94, `crater` 39) where the founder leaves almost no open nest at all
(`nest_deep` 0-3: its ants tunnel through soil that closes behind them). The picture is
`/mnt/project-files/colony-war/tournament-champion-vs-founder.png`.

## 5. The second tournament: food home in the score does not hold it

Same bed, mutation and pairing; the score is now the colony's share of the pair's ants lived **plus half its share of
food home plus half its share of `nest_deep`** (the open or provisioned nest joined to its door, 3+ rows down). Two
arms, `fight` and `graded`, 16 rounds each. Medians over the round's 24 colony-matches:

| Round | fight: kills / food home / `nest_deep` / crater / lived | graded: kills / food home / `nest_deep` / crater / lived |
|---|---|---|
| 0 | 33 / 4,522 / 2 / 4 / 65 | 19 / 4,585 / 2 / 6 / 65 |
| 2 | 43 / 797 / 168 / 39 / 78 | 24 / 800 / 194 / 42 / 77 |
| 8 | 34 / 468 / 202 / 40 / 76 | 30 / 420 / 163 / 44 / 79 |
| 15 | **72** / 562 / 168 / 28 / 75 | **50** / 600 / 146 / 44 / 78 |

Judged as before (two final-round champions an arm, 6 fresh seeds x both sides, no mutation, against `std_pool`, scored
on this tournament's score):

| Champions | Won (of 24) | share of ants lived | kills / killed (median) | food home: champion vs founder | `nest_deep` | crater |
|---|---|---|---|---|---|---|
| fight | **24** | 0.56-0.62 | 72-144 / 30-35 | 364-446 vs ~4,500 | 32-168 vs 1-2 | 24-40 vs 2-4 |
| graded (bite 0.25) | **24** | 0.55-0.57 | 32-48 / 6-10 | 292-473 vs 4,284-4,428 | 231-246 vs 0-2 | 48-56 vs 2-4 |

The fighting got better again (the fight champions kill 2-4 for each lost, the graded ones ~5), the open nest is real
(a joined nest 3+ rows down where the founder keeps none), and **food home fell exactly as it did without the term**:
from ~4,500 a match to ~500 by round 3, in both arms. **It fell the same way in tournament 1's no-bite control**
(4,926 -> 290), so it is not the fighting and not the score. Three things together:

- **A share of the rival's food home cannot hold a habit both colonies lose together.** Two grazing colonies split the
  term 0.5 / 0.5 just as two carrying colonies do; only a colony that carries while its rival does not is paid, and
  that pays for one round at most.
- **Inside a match nothing rewards an ant for carrying home.** Every hatchling is a mutant of its layer, and the layers
  are whichever ants are fed well enough to lay; the ants carried to the next round are whichever are alive at the end.
  An ant that eats where the food lies is both. Food home per 10,000 frames falls inside each match, and faster each
  round (fight, round 1: 653 -> 172; round 3: 172 -> 15).
- **On this bed grazing is not even a loss for the colony's headcount**: the heaps are 25 columns from the door and
  refill, and ants lived rose from 65 to 75-79 as carrying stopped.

The Teacher lane's in-run selection does not have the second problem: there births go to the ants that bring food
home, which is what holds carrying in its colonies. That is the reason the next step belongs in its harness, not in
this one.

## 6. What this means

**For the owner's question -- does selection against a rival settle into peace?** No. From a founder whose two
colonies keep a cold peace, both tournaments bred colonies that kill more each round and win every match against the
founder; a line bred with no bite loses every match to one that bites and dies out in nearly half of them. Peace is where today's pool starts, not where
selection against a rival goes. What keeps it from being a war of conquest today is that **a kill gains the killer
nothing in the world**: no flesh eaten, no food or ground taken. The tournament pays for a kill only because the score
is relative. For colonies to fight in the lab for a reason, killing has to pay -- the dead eaten, or food and ground
that only one colony can hold.

**For the Teacher lane (proposal, not built):** a paired-colony case in colony selection. Two colonies of the batch on
one bed (the bed and counters are `examples/rivalbed.rs`; inrun would need a second colony's ledger, births and culls
kept apart). The contest scored against the rival -- share of ants lived, kills both ways -- and **food home and the
joined nest scored as colsel already scores them, against the founder, not as a share of the rival's**, with births
inside the run still going to the ants that bring food home. Run it with the graded bite: with a one-bite kill, a lone
raider in a nest that cannot answer is the cheapest win there is (§2), and selection will find it.

**Not changed by this work:** nothing outside `examples/rivalbed.rs` reads `World::colony_lay_scale` (None everywhere
else) or sets `PP_MIND_BITE_SCALE`, so the lab, inrun and colsel are byte-identical.

## 7. Other things found on the way

- **The stranger alarm does not reach minds**: `World::encounter_log` recorded 0 touches in every run, so the game's
  stranger-touch alarm (`PIXEL_PHYSICS_STRANGER_ALARM`) is not a channel a mind ant uses. A mind lays alarm where it
  bites, and by its own alarm emit, which runs at ~280,000 a colony per 100,000 frames whether or not a neighbour is
  there (one colony alone 279k, two at peace 581k, two at war 563-598k, shared-cap batch): the alarm a pool lays today
  says nothing about strangers.
- **The lone predator report's "every trained pool holds Attack banned" is out of date** for `std_pool`: all 30
  members hold the bite on at job constant 0, from inrun's `enter_bite`. So two trained colonies in the lab can fight
  today.
- **On this bed the founder pool keeps almost no open nest** (above): worth knowing for anyone who reads `nest_cuts`
  as nest size.
