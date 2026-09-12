# The evolution lab, round twenty-nine: the bed comes back, and the colony learns to die

*The coordinator's record of one round, 2026-09-11 evening to 2026-09-12
(`session_01Y7C61ozfhgPj8Lo3Z6Swue`), written at the round's close and moved
out of [`lanes/evolution-lab-coordinator.md`](lanes/evolution-lab-coordinator.md)
so the note stays under its 12 KB cap. Status: **record, not a work order.**
Every number here was taken by the lane it is credited to and is on `main` or
on the branch named; what binds from the round stays in the note.*

**Read this if you want to know what the round overturned.** Fifteen lanes
ran under one coordinator across a container restart, a snapshot restore and
a five-hour usage cap, six of them relaunched as cloud sessions halfway; a
second coordinator (round thirty, Opus) started beside this one at the
owner's request and the two shared the trunk under a written protocol. The
owner's rulings at the open: *"You can ship everything on. I will tell you to
change it if I don't like it"*; his colonies reach 500–1,000+ ants where the
harness's peaked at 495; he sees a huge mound that never regreens; and cloud
sessions for lanes have *"no downside"*. Eighteen pull requests landed from
this round, the last of them an hour before the close, and none was left open
(see *Open at close*).

## State at open, and the choice

On `main` since round twenty-eight (#326): the flitter that hops, the bodies,
the pip's clock re-armed, heads at 12. The round opened on the round-28
record's first build — **the flitter must float like a bee, not hop like a
frog** — and grew into three programmes as the owner's playtest reports
arrived: the flitter's flight; the **late game** (the owner's own phrase for
the colony boom that eats the bed and dies all at once); and **colony
fission** (nest odour, budding, place-drift), which he asked for as
*realistic*, with the party as authored values rather than a caste.

Design first, then builds, each with a measured precondition: the flight
design ([`evolution-lab-flight-design-2026-09-11.md`](evolution-lab-flight-design-2026-09-11.md),
#329), the late-game design with its session-length census
([`evolution-lab-late-game-design-2026-09-12.md`](evolution-lab-late-game-design-2026-09-12.md),
#340) and the fission design
([`evolution-lab-fission-design-2026-09-12.md`](evolution-lab-fission-design-2026-09-12.md),
#341). Two of the three designs' baselines were overturned before the round
closed — see *What the round overturned*.

## Environment, learned this round — read before spawning anything

- **Cloud lanes are the default now, and the mechanics are not the obvious
  ones.** `create_session` with `source_url`, `source_revision`,
  `outcome_branch` and tags; the child opens its own PR. A lane reaches the
  coordinator, and the coordinator a lane, **only by poke**:
  `create_trigger(persistent_session_id=<id>, no schedule, the whole message
  in the prompt)` then `fire_trigger` bare (the response's `session_id` must
  read `cse_<id>`). `SendMessage` to a session id fails; `update_trigger`
  cannot edit another session's trigger, so it is a fresh trigger per message,
  and every one is deleted at the close. A poke does not strip a session's
  tools. **Copy trigger ids from the tool output; a retyped one dropped four
  characters and fired nothing.**
- **In-process lanes die with the coordinator's container; cloud lanes do
  not.** A restart at 05:13 UTC killed six in-process lanes; each worktree
  was salvaged as a WIP commit on its branch and relaunched as a cloud
  session. A snapshot restore twenty minutes later reset the coordinator's
  own checkout to the round-28 close and deleted `/tmp` — everything that
  matters is pushed, and the coordinator's state file is rewritten from
  memory first and appended at every event after.
- **Cloud lanes share the account's five-hour usage cap.** Six lanes at once
  burnt it in about 35 minutes; a stopped lane shows `post_turn_summary`
  *"session limit · resets …"* and resumes on a poke after the reset. Check
  `list_sessions mine:true` at every check-in.
- **A lane that ends its turn "waiting on …" is idle**, shown as
  `REVIEW_READY` with a *waiting* detail. Poke it to poll in the foreground.
  Two lanes did this; one had a finished branch and no PR.
- **Lanes arm GitHub auto-merge on their own PRs.** One did, on a PR queued
  third in a five-PR merge order behind another coordinator's holds. Disable
  auto-merge on every lane PR at subscription time and say so in the brief.
- **Five PRs in one file serialise, and `git merge-tree` tells you before CI
  does.** Seed cargo, the round trip, the pile, lifespan and the flitter all
  land in `src/sim/creature.rs`, the bug register's generated index and
  `wiki/ants.md`'s freshness note, so every landing invalidated every other
  open PR. The order was fixed, each lane re-merged once on a *"#N LANDED,
  main is <sha>"* poke, and `git merge-tree --write-tree origin/main <branch>`
  on the coordinator's side found each conflict an hour before CI would have.
  The contested `SUMMARY` lines are resolved one way only: `main`'s fields
  first and in place, the lane's appended, format string and argument list in
  the same order. A generated index is never hand-merged: take `main`'s and
  re-run `scripts/bugindex.py`.
- **A bug letter claimed on an unmerged branch is invisible to
  `bugindex.py --check`.** Four collisions in one day, none careless: the
  zoom-out lane took §Z11 first; two lanes filed §Z12; the pile lane was
  fenced to §Z12/§Z13; the baseline lane filed §Z14 while the flitter lane
  was moving to it, so the caged-flitter section was written §Z12, moved to
  §Z14 and landed as **§Z15** — wrong three times on one section — and the
  who-kills lane took §Z16 leaving Z15 free on purpose. The author allocates
  the letter at filing time and the check sees only `main`. Say the letter in
  the brief; sweep `git branch -r` for `^### Z[0-9]+\.` before filing; the
  durable fix is allocation at merge time, or a `bugindex.py --claim` that
  does the sweep itself — a tooling brief for round thirty.
- **A cancelled push run beside a pending pull_request run is the concurrency
  group, not a failure** — the note already says so, and it still read as red
  twice today.
- **The coordinator's own clock drifts.** Twenty minutes of narrative time
  were labelled as forty; `send_later`'s `fire_at` and a notification's
  *queued at* are the clock, not a running estimate.

**The cloud container restarts every forty to fifty minutes.** Lane M
counted four in a day; a six-run 500,000-frame sweep never once finished,
and this coordinator's own session was resumed twice in an hour by the same
mechanism. Any measurement longer than the gap has to checkpoint or be
split into runs that fit inside it; a lane that reports a long sweep
"still running" has probably lost it. The state file, appended at every
event, is what survives.

## The flitter — the float, the bed, the landing, the flight (#329, #332, #334, #339, #356 — merged `7bfb4e54`)

The round opened here. #332 gave an airborne animal a brain tick and a
surface for `Turn`; #334 measured the bed (ninefold the flowers, and the
flitter still cannot live on one — distance to the nearest flowering clump is
the lever); #339 closed §Z10 by turning the float's gate into a distance,
`(Bias, Fly)` −4.0 → −9.6, after 30 deaths in mid-air. All three are in
[`lanes/evolution-lab-flitter.md`](lanes/evolution-lab-flitter.md).

Four cards then said the same thing — *"both look very much like hopping"*,
*"stuck next to a plant, then one long hop"*, *"seems stuck in the plant"* —
three of them stills and one a follow-camera GIF, and the owner was right on
all four. Lane N (#356, merged `7bfb4e54`) found and fixed four real flight
faults: the brain's `Fly` output was squashed below 1 so lift could never
cancel gravity (**that is the hopping**: `HOVER_GAIN` 2.5); a Schmitt hold so
a flying animal does not drop the moment its output dips; a power-down
landing after twelve stalled frames; and a **proportional lift charge**, which
is the economic win (with the cruise off, visits 274 → 506 and births 12 →
25). The `(FoodAdjacent, Impulse, −2.0)` row cancelled `(Bias, Impulse, 2.0)`
and deadlocked a fed flitter for ~4,160 frames — **that was the sitting in
the plant**; it ships at −1.75. Over 120,000 frames and three seeds from one
binary (`PIXEL_PHYSICS_FLIGHT29=0` reproduces `main`): flower visits
142/39/85 → 421/106/142, births 1/0/0 → 25/3/6, share of deaths in the air
8/5/21% → 13/8/15% against the pre-#339 97/61/37%. **Alive at 120,000 is
still 0 on every seed in both arms**: the standing gap is the ground economy,
not the flight.

**And none of that is why the owner saw a stuck animal.** The flitter is
**caged**: `translated_if_free` moves a body only if every target cell is
empty, and `body_is_supported` counts `Plant`, so inside a canopy nothing is
empty to step into and the leaves beneath are perfectly good ground — it
launches and cannot translate, chattering between airborne and grounded on
one cell. The followed animal had zero empty neighbours on 532 of 601 frames
and sat at one cell for 1,200; between 42% and 12% of the colony cannot step
anywhere on `main`. `labgif follow=` takes the lowest live id, which is
disproportionately a settled, caged animal, **so every flitter card this
round was aimed at the failure.** Filed as §Z14 and deliberately not fixed:
letting a body into a plant cell is `relocate_chain` overwriting tissue, the
dead end already paid on this species (960 plant cells → 566). Starting
points for the design lane that takes it: `leaf.ron` already has density 0.25
and `climbable: true` against wood's 0.9, and no creature uses `climbable`.
Read the track (`labgif track=1`), not the picture.

## Late game, Brief 1 — the seed is cargo (#342, merged `a4359300`)

The colony's staple is the seed bank on the floor, and every seed an ant bit
was destroyed. Lane J made a bitten bare seed **cargo**: the bite takes the
provision the seed carries for exactly that purpose (`seed_provision_fraction`
0.25), the seed rides in the crop and is set down where the meal ends (the
round's own rule, #335: *the seed drops where the food is eaten*). A leaf on a
living plant is priced under the edibility bar (`food_energy` 480 → 40 — a
−0.6 gut still eats it), litter stays a whole meal, and **grassblade was never
food** (both fields unauthored, i.e. 0) — the design's "480 → 40" was a cut
for one file and a rise for the other. `SeedBite::{Digested, SurvivedInFlesh,
SurvivedBare}` replaces a bool because a species with no fruit defaults its
windfall to the string `"seed"`, so the material test was wrong for grass.

The owner's card question — *"is it about to get destroyed or coming
back?"* — has an answer: **it keeps falling ~100,000 more frames, bottoms at
about a third, and comes back.** Plants standing on seed 3: 198, 134, 96, 68,
**62** (the bottom, 260k), 144, 114 at 500k, against `main`'s 202, 110, 103,
14, **2, 2, 2**. All three seeds come back with the build; one of three does
not on `main`. Bank at 500k 402 / 1,090 / 1,574 against 0 / 368 / 39. **It
saves the bed and shrinks the colony**: on seed 1 `main` reaches 3,182 ants
(13,975 births, 13,897 starvations) and leaves 58 plants over a bank of 39;
the build's colony stays in single figures. `plants_from_pip` 65 / 41 / 156,
where every earlier measurement read 0–2. A **KILLED** death channel appears
with seed cargo (56 / 37 / 153 at 120k against 0 / 1 / 0; 159 at 500k) and is
not nestmate predation by the food rule. It is not the baseline shift below,
and #358 found what it is: **the bed writing plant cells over the ants**
(see *Who kills whom*).

Two things checked rather than assumed after its merges: #345 re-renders the
played bed byte-identically; #347 leaves the whole 500k census **identical in
every column**, and the null was checked — cohesion fires (13,550 blends in
20k frames) but the played bed has **one** nest, the blend writes scent slots
only, and nothing in the feeding, digging or planting economy reads scent. A
scenario with two colonies is needed to measure cohesion or budding. And the
census move (#351) would have silently dropped the carried-seed rule: the
moved `is_waiting_seed` counted an organism owning one seed cell, and a seed
riding in a crop owns **no cell** — every passenger would have counted as a
plant, and no test would have gone red.

## Late game, Brief 2 — ants die of age (#354, merged `24738cbb`)

`CreatureDef::life_half_life` runs the plants' own hazard
(`plant::old_age_chance_over`, interval-aware, its own RNG slot) on any
species that authors it; `ant.ron` and `longant.ron` ship 40,000, every other
animal is untouched by construction and byte-identical by measurement; a dial
on the ANTS page; `OldAge` is the eighth death cause. The hazard's arithmetic
was corrected on the way: survival at 2.5 lifespans is **1.31%**, not the 0.4%
the design report §1.2 said (0.4% is survival at 2.83).

**Its headline was overturned twice and the honest version is the record.**
On the tree of the morning (`f3acaf76`) the immortal colony swung 530-fold
over the second half of a session and fell 3,099 → 16 in one 20k step — the
owner's *"they die all at once"*, measured — while the 40k arm held a 3.2-fold
band for 280,000 frames. On the trunk after cohesion (`c7ee0f40`) neither
claim survives: **every colony at every setting, 0 included, is extinct by
500,000 frames**; the lifespan does not bound the peak (760 → 1,265 on seed
1); what it changes is what the colony leaves — seed 1 ends with **114 plants
over a bank of 600** against 14 plants and an empty bank, the only arm to
clear the bank half of the programme's bar, and 1,715 of 8,960 deaths are age
rather than hunger. The mechanism, its controls and three unit tests are
about the hazard and are unaffected; the outcome it was meant to buy is not
demonstrated on the current trunk. It landed at `24738cbb` on head
`33cad2ed` after two re-merges (#353 and #357 landed ahead of it). The
three-seed sweep on the merged tree — six 500,000-frame runs at one thread,
one to two hours — **could not be run**: the cloud container restarts every
forty to fifty minutes (four times in the lane's day) and killed it twice,
at 60,000–100,000 frames and at 80,000–140,000. The shipped table is
therefore labelled as one trunk behind the code in the README and the lane
note, each with the command to re-derive it, and the re-derivation belongs
on a machine that stays up or in runs short enough to finish in forty
minutes (about 200,000 frames, which reaches every seed's peak and not its
tail). What the partial runs did say: on `08ea61f7` seed 3's immortal colony
was already at zero by 100,000 frames where the earlier table had 89 and a
peak of 190 at 140,000 — seed cargo moved this bed again, toward smaller and
shorter-lived colonies.

## Late game, the baseline that moved — and what did not move it (#357, merged `08ea61f7`)

Lane M re-ran the *identical* control arm on two trunks. Ants on the played
bed, seed 1, immortal, at each 20,000-frame stop:

```
main f3acaf76   73 205 339 354 752 1816 2013 1023 943 2079 3182 3099 16 25 6 46 458 1283 848 289 108
main c7ee0f40   73 236 334 254 274  420  676  705 726  760  126  208 129 94 125 2 0 0 0 0 0
```

Identical to 100,000 frames, then a different ecology: peak 3,182 → 760,
extinct by 420,000. Only half the runs moved — seed 2 at both settings and
seed 3 immortal are byte-identical across the merge — so whatever it was is
inert on a small colony and decisive on a large one, and **no control shorter
than the mechanism's onset can see it.**

Lane O isolated it on **one commit**: on `c7ee0f40` with `scent_drift` forced
to 0 the run reproduces the `f3acaf76` series digit for digit on all 21
stops, and the shipped arm reproduces the `c7ee0f40` series. It is #347's
drift at 0.15, and nothing in round thirty's #344–#346. **Then every proposed
mechanism failed, including O's own.** Not combat: the shipped arm kills none
of its own to 500,000 frames and the drift-0 arm kills six; deaths equal
starvations in both. Not strangers: 0.00% of ordered pairs read as non-kin
where the arms part, 0.18% at worst, scent spread 0.17 against a kin radius of
1.0. Not suppressed sharing (this coordinator's reading and then O's, both
withdrawn): at 180,000 the drifting colony shares **more** per living ant,
179 against 91. And not the extra random draws — a control at drift 0.0001
consumes exactly the draws 0.15 does, is too small to flip anything, and
reproduces the drift-0 arm exactly on 17 stops. Nothing reads a scent slot
continuously; the only consumer is a threshold at radius 1.0, so a
perturbation either flips one kin decision or nothing, and the channel is
occasional flips amplified by a chaotic bed — inside the *single* drift-0 arm,
adjacent 20,000-frame stops read 3,099 and 16. **§Z14: a single-seed
500,000-frame trajectory on this bed is not a baseline**, and cannot be
compared across any change that perturbs behaviour at all. Order statistics
over seeds, or nothing.

**On the current trunk the dial is inert.** On `main a4359300`, seeds 1–3,
500,000 frames, drift on and off are identical on 26 of 26 rows. The colony
peaks at **12 / 12 / 212** ants against the 3,182 that seed 1 reached before
seed cargo, and is extinct by 420,000 / 100,000 / 220,000 — the late-game
boom the design was written around is gone from this bed — and deaths are now
**mostly killings**: seed 1 killed 82 against 40 starved, seed 3 killed 159,
which is lane J's unexplained figure and reads the same at drift 0. J's
KILLED channel is therefore excluded from scent drift by direct control.
O's recommendation, carried here: change nothing on the drift value, there is
no live cost to weigh; the fresh census replaces the late-game design's §0
(censused at `be2808de`). *Who kills whom* was the round's most important
open question for six hours, and the next section is its answer.

## Who kills whom — nobody; the colony is overgrown (#358, merged `%%SHA358%%`)

The question had a wrong premise, and lane O found it in the code before
running anything. `creature::reconcile_chain` books `DeathCause::Killed`
whenever a creature's deciding cell goes away, **whatever took it** — its own
comment has always said *"a bite, a fire, a blast, the brush — so the cause is
`Killed` and no finer"*. `World::tally_kill`, the counter every page reads as
"killings", fires only when an animal's bite took the cell and both parties
still resolve. Two lanes this round read the first number as the second.

O logged both: a per-kill record with both parties, the frame and the
victim's energy, and for every `Killed` death what was standing in the vital
cell at the moment it was booked. Played bed, 500,000 frames, one thread,
shipped configuration:

| seed | `KILLED` booked | by an attacker | empty | pip | grassblade | other plant | ground / water |
|---|---|---|---|---|---|---|---|
| 1 | 216 | **2** | 129 | 40 | 29 | 2 | 14 |
| 2 | 98 | **0** | 33 | 37 | 17 | 4 | 7 |
| 3 | 70 | **0** | 33 | 25 | 8 | 2 | 2 |

**Two of 384.** Both are ants of the same colony at about 300 J in the third
tenth of one run — two events, not a channel. About two fifths of the
colony's "killings" are a **plant cell standing where the ant's head was** —
a pip, a grass blade, wood, a seed, a leaf. The colony that seed cargo shrank
to a dozen is not fighting and is not being hunted; it is being **overwritten
by the bed it saved**. The other half of the deaths leave the vital cell
**empty**, vacated by something that left nothing behind, and this instrument
cannot say what: that is the larger half and the next measurement. Nothing
here is a mechanism change; the log and the table (`World::kills_log`,
`World::vital_losses`) are per event and off the sweep, and `latecensus`
prints both under *who kills whom*.

**And it is not growth.** This coordinator's first reading — a plant growing
into the ant — was wrong, and O read the code before the round could act on
it: `plant::growable` is the gate every growing tip passes and it refuses an
occupied creature cell twice over (a shoot takes a cell only when it is
empty, a root only when it is a soft powder), so the pip was not grown there.
The cell was **converted in place** by a path that never consults `growable`,
and there are three: `plant::seed_survives_bite` writes a pip *over* the
bitten cell rather than clearing it, immediately before the creature path
reconciles the victim; `plant::germinate` converts a standing pip in place,
which would make the grass blades a consequence of the pip case rather than a
second route; and whatever sets a carried seed down. For the empty half the
bite path is nearly excluded — it reads the victim before its clear and
attributes the kill, and 384 deaths produced two — so the cheapest next
instrument is one bit on `note_vital_loss`, *was an attack in progress on
this cell this frame*, which splits the empty column in a single run. O also
notes that **§Z15 and §Z16 are one collision seen from two sides** — the
living animal a plant cell holds up and blocks, and the dead animal whose
head cell became a pip — and may be one repair rather than two.

The repair is therefore two things, and only one is cheap. **Which of the
three in-place writes puts a plant cell into a creature's cell, and does it
own the cell it writes** — a measurement first, then a rule, on every bed,
read beside §Z15. And `DeathCause::Killed` should be split, or at least
renamed in every readout, so a lost cell is never again reported as a killing
— a labelling change, and the thing that stops a third lane re-deriving this.
Both are filed as §Z16 with the table above. The round-thirty coordinator
has them with the correction and **takes the rename in round thirty**; the
in-place write and the empty half it holds for round thirty-one. Its own
room-per-ant lane reached the same verdict from the other end — a twelve-seed
sweep said the dig gate's question is not what makes the mound, and it ships
that gate off — so two lanes at opposite ends of the day agree that what
shrinks this colony is neither digging nor fighting.

## Fission B1 — one odour per nest (#347, merged `9979e6fa`) and its precondition (#350, merged `d3d4aec5`)

A `NestSite` list in `world.rs`, registered once per patch; an ant at the nest
takes a little of the mound's odour and leaves a little of its own
(`nest_blend` 0.10, `nest_uptake` 0.02); the nest's own odour wanders
(`nest_scent_drift` 0.065 per 1,000 frames) and `carry_nest_wander` moves the
residents with it; `scent_drift` 0.15 ships on; the dials sit on the ANTS
page. Two findings overturned the design: the free-nest arithmetic was wrong,
and `TRAIT_TOLERANCE` drifts unblended, so at drift 1.0 a colony kills 16–30
of its own where at 0.15 it kills none. The owner's card verdict asked two
things at once — *does he then get a different home, or no colony, or a new
colony?* and *where is the nest defined, since the ants moved from where they
were placed and live in the plants* — and the second became a measured
precondition: **on seeds 1 and 2 the colony leaves the painted patch inside
10,000 frames and never returns** (all blends in the first window, then 7–8
for 110,000; centroid ~130 cells out); on seed 3 it stays (1,077 blends in the
last window). The owner was right on two beds in three, budding's `Leave`
gated on `AtNest` cannot fire where nobody is at the nest, and the owner's
*"too long away turns enemy"* is **not built**: an ant's odour moves at birth,
home undoes it, away it neither blends nor drifts. The costed repair (gate
`carry_nest_wander` and blending on proximity; re-derive σ) is his design
call.

## The round trip — the door stood under a puddle (#343, merged `ccc421ee`)

§T2's frozen deliveries had a mechanism: `nest.ron` authors no
`water_capacity`, so a one-cell film on the patch is a wall an ant will not
step into, and `AtNest`, `nest_visits` and `deliveries` freeze on one frame
while `pickups` climb. Lane K's `nestdoor` census gave every hypothesis a
counter that could move only under it — buried (no), dug away (no), drowned
(**89–91 water cells on the patch against 17–19 on the same width beside it**:
a nest that floods because it is impermeable floods *alone*), nobody paths
home (also, separately), unloads en route (stale since 2026-09-02). Two
principled fixes were built and are worse (`water_capacity` on a `Solid`
aliases `Cell::aux` and makes the door a water sink; `nest` as a `Powder`
loses the gnome a wall), so `paint_nest_patch` leaves every third column as
the ground it was: `nest_visits` 731 → 2,730 and deepest generation 7 → 26 on
seed 1. The film guard reads **volume**, after its first version counted 36
cells at fill 8–12 of 1,000 as "36 under water" — the door had drunk 99% of
what fell on it and the bed was 1.3% full. §T2 stays open: the last 1% of a
film is still a `Liquid` and still a wall, and past ~170 ants the residual is
home-finding (laden mean distance 54 → 143).

## The long-ant pile — three things, one fixed (#353, merged `39b31c7e`)

The owner: *"long ants getting stuck … in a big group/pile of long ants."* It
is three things. **Long bodies waiting on each other**: `boxed_by_traffic`
withholds a laden animal's flip on the premise that a jam clears when the
other animal steps, and §13g named the case where it does not — the other
animal is boxed too — so the deferral repeated for ever (the longest-boxed
animal was laden on 24 of 25 sampled stops, head and tail in the identical
cells, for streaks of 68 stops). Lane I bounds it: after
`traffic_defer_max` consecutive deferrals (4 of the animal's own ticks,
`longant.ron` only, two-cell bodies unreachable by construction) the flip
fires anyway. Over nine seeds no column is consistent in sign; wedged long
bodies as a share of readings improve on 7 of 9 (median −0.72 pp, seed 9
5.05% → 1.33%), `alive` rises on 7 of 9 (median +60), deliveries are a wash.
**Bred one-cell morphs that cannot flip** (§Z12): `pile_short_by_loss` 0 in
17 of 18 runs, authored and held cell counts both a mean of 1.0, max
generation 6–31 — inheritance, not injury, and the visible clump is theirs
(3+-cell clumps never exceed 7 in either arm). **Resting that reads as stuck
at play zoom** (§Z13): the owner's three marked spots were full-length ants
with open headings, zero refused steps and rising energy, and the shipped
two-cell ant's idle streaks are *longer* (p90 13–20 stops against 11–16) — a
look problem, not a mechanic. The first card's numbers did not describe its
own picture (whole-run maxima of 108/41 over a window whose largest clump was
four animals), which is what the owner's *"your numbers show a huge
difference, but I see…"* meant.

## The chronicle carries a census (#351, merged `c7ee0f40`)

The owner asked whether logging his real playtests would help. Key 9 saves
the chronicle; a `CENSUS` row lands every 10,000 frames
(`PIXEL_PHYSICS_CHRONICLE_CENSUS_EVERY`) at 1.7 ms a call; files at
`assets/chronicles/chronicle-<date>-<bed>-s<seed>.txt`; `examples/latecensus`'s
body moved into `src/lab/census.rs` so the game and the harness read one
census. Two lanes then ported columns into the moved module after the merge
would have dropped them in silence.

## The earlier lanes, in one line each

The played bed's thinning bisected across `d1535c39..eafde084` (#330): no
single landing owns it, and the colony's eating climbs with every crossing.
The seed drops where the food is eaten (#335): the rule had been violated
twice, digestion releases the pip and a dropped fruit keeps its seed. The
zoom-in and zoom-out designs, the soil design and the magnify styles (#344,
#345, #346, #352) and the readout corner (#355) are round thirty's.

## What the round overturned

- **The played bed's baseline moved under everyone.** Shipping nest scent
  drift at 0.15 changed the late half of every large-colony session; the
  late-game design's §0 census and every played-bed population figure past
  ~100,000 frames taken before #347 are measurements of a tree nobody has.
  The change is inert on a quiet bed, so a lane that re-checks on one will
  correctly find nothing moved.
- **The colony does not turn on itself, and nothing hunts it: the bed
  overwrites it.** `Killed` was never a killing counter; of 384 such deaths
  on three seeds, two are an animal's bite and about a hundred and fifty
  leave a plant cell where the ant's head was — not grown there (`growable`
  refuses an occupied cell) but written in place by the bitten-seed,
  germination or seed-drop path. The other half leave an empty cell and are
  not yet explained. This coordinator's own "overgrown" reading lasted an
  hour before the code overturned it.
- **The stuck flitter was never the flight.** Four cards, four verdicts, all
  aimed at a caged animal by a camera that picks the lowest id.
- **The stuck pile is mostly not stuck.** Bred one-cell bodies and resting
  ants; the one genuine jam is bounded.
- **The nest door was under water.** Not spoil, not digging, not the walk —
  a material with no water capacity.
- **A seed riding home saves the bed and starves the colony**, and the
  design's leaf cut was a rise for grass.
- **Survival at 2.5 lifespans is 1.31%**, not 0.4%.
- **The nest is not where the colony lives** on two beds in three, which
  bounds budding, cohesion and the owner's "time away" model alike.
- **Method:** a determinism or baseline control shorter than a mechanism's
  onset proves nothing (identical to 100,000 frames, dominant after); a card's
  `meta` must be measured in the window it shows; `labgif` defaults to
  `rain=steady` and `latecensus` sets no rain, so a card paired against a
  census must pass `rain=off`; a 3x magnified view is blind to thin-feature
  loss; an exactly identical result across a simulation change is the shape
  to distrust, and the null is checked by asking what the condition could
  discriminate.

## The owner's verdicts (2026-09-12)

Flitter cards: hopping, stuck, cannot tell — four times, and correct each
time. Seed cargo: *"A looks slightly better but not sure what happens at
80–100k frames"* — answered above. Long-ant pile GIF: *"way too slow"*, and
three markers of *"no movement"* that were resting full-length ants. Cohesion:
two mechanisms on one card, could not tell them apart, likes both, and the
two questions that became #350. The render board's zoom cards belong to round
thirty.

## Open at close

Every round-29 pull request is on `main`: #329, #330, #332, #334, #335,
#339, #340, #341, #342, #343, #347, #350, #351, #353, #354, #356, #357 and
#358, the last at `%%SHA358%%`. What is open is work, not paper:

- **Three owner design calls**, put to him and not decided here: the nest
  scent drift value (shipped 0.15, inert on today's trunk, decisive on a
  large colony); the *"time away turns an ant into an enemy"* model, which
  needs a home the colony actually lives at first (#350); and whether a plant
  cell may ever be written over an occupied creature cell (§Z16 and §Z15,
  one collision from two sides).
- **§Z16's empty half** — half the colony's deaths leave a vacated cell that
  no instrument yet attributes — and the `DeathCause::Killed` rename, both
  round thirty's, with the finding already in its coordinator's hands.
- **§Z15, the caged flitter**: a design lane starting from `climbable`, not a
  flight fix. The flitter's ground economy (resting, the walk, the eye,
  `start_energy`) is the reason it still dies out at 120,000 frames with the
  flight working.
- **The pile's remainder**: the bred one-cell bodies (what are they?) and
  resting-versus-stuck as a readout, from #353.
- **Lifespan**: the 20k/80k arms were never re-run on the trunk, and the
  40,000-frame constant is labelled one trunk behind its sweep; the
  re-derivation needs a machine that stays up for two hours or runs cut to
  200,000 frames.
- **§T2's home-finding residual**, and a two-colony scenario without which
  cohesion and budding cannot be measured at all (#347 left the census
  identical in every column on a one-nest bed).
- **Tooling**: a `bugindex.py --claim` that sweeps `git branch -r`, so the
  fourth bug-letter collision of the day is the last.
- **Cards awaiting the owner**: the lifespan session strip (`07d3ab`, blind)
  and the flitter flight A/B (`89ba6b`, blind); read both through
  `blind_was`.
