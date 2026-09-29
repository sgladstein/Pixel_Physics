# Ants that breed like ants: eggs, brood, a colony's own breeder, and colonies that found colonies — an implementable plan, 2026-09-29

**Status:** plan, proposed 2026-09-29 for the owner's reading. No step of it is
in the code yet. The one code change it asks for before anything else is B0a,
a crash fix. Written against `main` at `2274e347`. Tag: `engine` — the
mechanism is shared, and §7 says what each of the three games turns on.

**How this was put together:** the code was read directly for `try_bud`,
`place_creature`, the breeding regimes, old age, founding and the
seed-dormancy substrate. Five parallel reads covered the reports: reproduction
economics, castes and biology, genetics, the registers (`dead-ends.md`,
`open-bugs-handoff.md`, `PLAN.md`, README), and the in-flight lanes. The
real-ant biology in §3 was checked against papers retrieved from PubMed, cited
with DOIs in §11. One claim was run rather than read: the crash in §1d.

---

## 0. The answer, stated once

**What this is for.** Today an ant that has eaten enough splits off a
full-grown copy of itself on the spot, wherever it stands. Every ant breeds.
There is no egg, no brood and no queen, and a colony never starts another
colony.

This plan replaces that, in six shippable steps, with the ant life cycle:

- New ants come from **eggs laid in the nest**. The eggs lie in a pile in a
  chamber and hatch after a delay.
- **Larvae must be fed** by the colony to grow, so food brought home has
  somewhere to go.
- The ant that is breeding **makes breeding harder for the ants around it**.
  Each nest therefore finds one or a few breeders, and the rest feed them and
  their brood.
- **Young ants stay home, pale**, before they go out. How a larva was fed
  decides what it grows into.
- A colony that is big and well fed **raises winged breeders that fly off and
  found colonies of their own**. Most of those foundings fail.

**Where it sits.** The colony now forages, homes, stores food in a granary, and
digs a door, a shaft and a storeroom. Breeding is the last part of its life
still done as an instant copy, and it is what all that food is for. Four
rejected switches (`dead-ends.md` L1185, L1194, L1195, L1204) each name *a
brood to feed* or *a birth paid from the store* as the condition that would
change their verdict. The nest's own "what is it for" question
(`nest-biology-2026-09-19.md` §10–11) was answered with *eggs*.

**The constraint everything is built around.** The owner ruled on 2026-09-09
that a queen is *"three authored values over mechanisms that exist … never a
type the engine knows"*. The same rulings made fertility graded and required
castes to behave differently, not only breed differently. The breeding-clock
study then measured queen-only breeding collapsing the lab's evolutionary
clock **thirteen-fold** (median generation 1 against 13.5). The cause is
structural: the box has no way for a colony to found a colony.

So **nothing below names a queen**:

- A breeder is any animal that has bred — the existing `children > 0`.
- She is made a breeder by what the colony does around her, not by a flag.
- The step that lets breeder-centred breeding pay its way (B6, colonies
  founding colonies) is **in** the plan rather than deferred.

**The steps, and what each puts on screen:**

| step | what you see | what it builds | size |
|---|---|---|---|
| **B0** before anything | nothing, except the held world no longer crashes | the crash fix, the owed `GRADED_MAX_SUPPRESSION` sweep, a re-taken breeding clock, the switch documented | 1 session |
| **B1** the egg | pale grains piling on the chamber floor beside the ants that laid them, hatching into ants | a birth lays one egg cell instead of a whole adult. Budding's economy is otherwise unchanged (`nest-biology` D11.2) | 1–2 sessions |
| **B2** graded fertility on | breeding gathers onto one or a few animals per nest | the existing `graded` regime, swept, with the breeder's eggs as a second signal source | small |
| **B3** brood that must be fed | a brood pile in three colours (egg, larva, cocoon) with nurses crowding it. In famine the pile thins before the workers die. Dig into a chamber and ants carry the brood away | larvae with a bank that nurses fill through the existing `Share` verb. Laying becomes cheap and growing an ant is what costs | 2–3 sessions |
| **B4** what the young become | callow ants pale and at home, turning into foragers one by one. Richly fed larvae become different ants | `made` from how a larva was fed, the developmental block wired, a stochastic callow release | 1–2 sessions |
| **B5** the founding rule | a colony starts as one well-provisioned breeder in its chamber plus a cohort, not 52 identical strangers | a per-species founding rule in `ant.ron`, and a nest-bound founder | 1 session |
| **B6** colonies found colonies | winged breeders leave the mound after rain. New mounds appear across the world, descended from the old | `(Made, Fly)` alates and a founding verb shared with the fission design's budding party | 2–3 sessions |

**Deliberately out of it (B7):**

- **Mating and males.** The world stays asexual: a standing dead end, because
  asexual budding is what keeps lineages diverging.
- **Breeders that outlive their workers.** This has to be priced before it can
  be inherited.
- **A bigger queen body.** Longer bodies do not survive in this engine.

§5 B7 says what each would take.

**Found on the way, and confirmed by a run:** a refused birth by any animal in
organism slot 4,096 or above crashes the game (§1d). It is one line from B0's
fix.

---

## 1. What an ant's breeding is today, from the code

### 1a. The birth path

- **When.** `creature::creature_tick` step 9, after digestion, calls `try_bud`
  on every tick an ant survives.
- **The bar.** `reproduce_at_of` is `reproduce_threshold` (1,100) × the
  heritable `TRAIT_REPRODUCE_AT` multiplier. It is floored at `birth_cost_of
  + 1`.
  - **The price** is `birth_grant` + `body_energy` × body cells: 80 J (the
    grant, 0.4 × `start_energy` 200 from `TRAIT_BIRTH_GRANT` −0.2) plus a
    960 J body stamp. That is **1,040 J**, all paid by the parent.
  - The bar counts the food in the eight cells round the head as well as the
    bank (`reachable_provision`, seeds at their guaranteed-bite price since
    §Z36). A birth the bank cannot cover eats that food and leaves the parent
    at 1 J or more.
- **Where the child goes.** `place_creature(…, Origin::Bud { … })` lays the
  child on the first of the parent head's eight neighbours where the **whole
  body** fits. Its facing is flipped (`facing_west = dx >= 0`) so it does not
  lie across the parent, and only three of the eight neighbours can ever take
  a multi-cell body.
  - A failure charges nothing, calls `World::note_birth_denied`, and is
    retried next tick.
- **What the child inherits.**
  - **The brain genome**, via `brain::mutate` at `mutation_rate` 0.0033758 per
    live slot, keyed on the child's handle (`RNG_SLOT_BIRTH`).
  - **Fourteen trait slots**, jittered by `trait_variance` (0.15 on slots
    0–9).
  - **The body's `FateGenome`**, mutated at `fate_mutation_chance` on the
    parent's handle.
  - **`made`**, the parent's `Provision` output. `ant.ron` wires none, so
    every child gets 0.
  - **`lineage` and `colony`**, copied, and `generation + 1`.
- **What the parent gets.** `children += 1` and `record_breeder`, which feed
  the breeding regimes.
- **The ledger.** The parent is charged the whole price. The stamp is booked
  `StoredInMeat`, and the grant moves live-to-live.
- **Death.** Starvation at a bank of zero or below. Old age on a hazard that
  rises linearly with age, with a median of `life_half_life` 40,000 frames
  (`plant::old_age_chance_over`), read against `born_frame`.

### 1b. What already exists for a colony's breeding, switched off or unwired

| piece | where | state |
|---|---|---|
| **Breeding regimes** — `PIXEL_PHYSICS_BREEDING=individual\|queen\|graded`, `PIXEL_PHYSICS_BREEDING_RADIUS` (24) | `creature.rs` `breeding_regime`, `suppress_bar`, `nearest_breeder`, `colony_has_other_breeder`, `GRADED_MAX_SUPPRESSION` 6.0; `World::colony_breeders` index | in the code, default `individual`. **`GRADED_MAX_SUPPRESSION` has never been swept**, and the owner's standing gate says graded ships only after it is. No living document names the switch; §5 B0c adds it to the reference |
| Breeder counters | `World::deepest_breeder_generation`, `CreatureStats::breeder_scan_visits`; `labforage`'s `brdr`/`gen`/`bgen` columns | live. **`gen` and `bgen` diverging is the tell** that an arm manufactures genetic dead ends |
| Breed only at the nest — `PIXEL_PHYSICS_BUD_SITE=nest` / `World::bud_at_nest` | `try_bud`, `buds_held_for_nest` | off. With an adult-length child it took births **1,592 → 0**: a clear line for a whole body exists at the nest 0% / 7% of the time (`ant-scenes-2026-09-23.md` §9–10) |
| **The caste channel** — `Provision` output, `Made` input, the developmental block (`brain::TRAIT_SLOTS`, 64), `expressed_traits`, `World::plasticity` 1.0 | `brain.rs`, `creature.rs` | on, and **nothing fires it**: `ant.ron` authors no `Provision` weight and no developmental weight |
| Nest-bound animals — `OrganismState::nest_bound_until`, `is_nest_bound`, `home_pull`; `Storeroom` parts `nestbound=<frames>[/<k>]`, `caste=<k>`, `workerhome` | `creature.rs` | `caste=4` is the default: one ant in four, **by id**, is nest-bound for life. It lives, eats, digs **and breeds** in the founding cut |
| Flight — `BrainOutput::Fly`, `launch`, `step_flight` | `creature.rs` | the flitter flies. The ant authors no `fly_cost_in_moves`, so no ant can leave the ground |
| Colony identity — `claim_colony`, `World::colony_parents` (the `ANT 1b` naming), scent signatures | `world.rs`, `creature.rs` | on. **`scent_spread` is now 2.0 in `ant.ron`**, so two foundings smell different. That is half of the breeding clock's re-open condition (colony competition) |
| **The seed-dormancy substrate** — a dormant organism allocated at seed set, owning one `CellType::Seed` cell of a `Powder`. `World::set` keeps its cell list current under both drivers; `relocated_seed` re-finds it after it falls; `World::carried_seed_organisms` keeps a carried seed's id alive | `plant.rs`, `world.rs`, `organism.rs` | the substrate `creature-direction.md` §3b and `nest-biology` D11.4 name for the egg |
| Founding — `found_colony_with`: a door (5 columns), a 6×2 shaft, an entrance chamber, a side storeroom; `founder_reserve_spread` 0.5; `COLONY_ANTS` 52 on `Y` | `creature.rs` | on. The shaft is sized "as a queen's first burrow" (`nest-colony-size` §3) |

### 1c. Where births happen now, and what limits them

- **At the door's floor food.** 56 of 58 births came from a parent at home
  (`ant-scenes-2026-09-23.md` §22n). That is an accident of
  `reachable_provision`, not a design, but it means a nest-centred model moves
  births a short way rather than a long one.
- **Bigger colonies breed less per head, and nobody knows why.** Born per
  founder was 0.30 / 0.11 / 0.06 / 0.01 at 20 / 40 / 80 / 200 founders, with
  starvation at 13% / 28% / 42% / 52% (`nest-colony-size-2026-09-28.md` §2).
- **Today's colony-bed baseline** (`main` 2274e347, 24 seeds, from the
  foraging lane's handoff branch):

  | bed | born | starved |
  |---|---:|---:|
  | 90 cells | 188 | 57 |
  | 140 cells | 63 | 127 |
  | 80 founders | 74 | 1,109 |

- **Lab box.** The §Z36 fix cut births 608.5 → 407.5. The granary cut them
  443 → 226, and that second drop is untraced.
- **The breeding clock's numbers are stale in absolute terms.** 13.5 / 8.5 /
  1.0 generations per 120,000 frames were taken before the #366 cull fix and
  before §Z36. The queen collapse is structural and should survive a re-take;
  the individual-against-graded trade should be re-taken before the owner
  rules on it (B0d).

### 1d. Two defects found while reading

**A refused birth in organism slot 4,096 or above crashes the game.** Confirmed
by a run: `World::note_birth_denied(4095)` passes, and `note_birth_denied(4096)`
panics with *"index out of bounds: the len is 64 but the index is 64"* at
`world.rs:7579`.

- `World::denied_seen` is `[u64; 64]`, which is 4,096 bits: one per slot of
  the old 12-bit organism index.
- `ORGANISM_INDEX_BITS` has since been widened to 20, giving 1,048,575 slots
  (`world.rs` L38–62).
- So any creature allocated past slot 4,095 that is refused a birth for want
  of room indexes past the array.
- **The held world is where this bites.** Its grown start makes 4,093
  organisms (the same `world.rs` doc, citing §Z21), so ants founded with `C`
  after it land in slots 4,094 and up, and the first one refused a birth
  crashes the game.

This was not observed in a played session. The arithmetic is confirmed; the
path to it is read from the allocator. The fix is B0a.

**Three comments still say the ceiling is 4,095**:

- `push_organism` ("`None` when the 4,095 slots are all live")
- `World::free_organism`'s doc ("caps concurrent organisms at 4,095 — one long
  session of a laying queen exhausts it")
- `note_birth_denied` ("`ORGANISM_INDEX_MASK` is 12 bits")

**Three of the five readers that surveyed the reports for this plan quoted the
stale ceiling back as a live constraint on eggs**, which is exactly the failure
a stale comment causes. The ceiling does not bind eggs; the census does (B1,
the second trap).

---

## 2. What is already settled — rulings and measurements this plan must not re-derive

### 2a. Owner rulings

| ruling | date | recorded in | what it forces here |
|---|---|---|---|
| *"A queen is three authored values over mechanisms that exist — a per-species founding rule, a founder who rests because she is full, sterile workers through the caste channel — never a type the engine knows."* | 2026-09-09 | `evolution-lab-breeding-clock-2026-09-10.md` §0; `evolution-lab-direction-2026-09-09.md` L62; the lab coordinator's not-to-be-relitigated list | no `Queen` type, flag or species anywhere. A breeder is `children > 0`. B5's founding rule lives in `ant.ron`; sterility comes through `made` (B4) |
| *"Castes must actually behave differently, not only breed differently"* — `(Made, verb)` weights are the mechanism | 2026-09-09 | breeding-clock §0 | B4's and B6's castes are `(Made, verb)` weights, including `(Made, Fly)` for alates |
| *"Fertility is graded, so a queenless colony's workers resume breeding."* | 2026-09-09 | same | B2 uses `graded`, never a sterile bit. **Principle check:** zeroing the sterility weight must give budding workers back |
| *"Rest is the absence of a reason, not the presence of a full stomach."* | 2026-09-09 | same | B5's breeder is not coded to rest; she is given no reason to walk, and that is measured |
| *"I lean graded suppression…"*, and graded ships only after `GRADED_MAX_SUPPRESSION` is swept | 2026-09-10 | breeding-clock §6.1; coordinator note | B0e is the gate on B2 |
| Trophallaxis is *"a brain output the genome evolves, never a rule"* | 2026-09-09 | coordinator note | brood is fed through `Share`/`KinNeed`, not through a feeding rule |
| *"The mechanism is code, the policy is genome"*; *"add senses and economies, never behaviours"*; *"a sense must not pre-categorise what it senses"* | standing | `creature-genome-flexibility`; `evolution-lab-late-game-design` | no brood-specific sense. `KinNeed` reads each kin against **its own** target (B3) |
| *"Anything should be able to evolve. Don't lock"*, tempered by *"priced before it is inherited"* | 2026-09-05 | `creature-locked-fields-2026-09-05.md`; README lifespan status | new brood timings start as species constants and become genes only once priced (B7) |
| *"An omnivore should be viable."* | 2026-08-23 | card `…963f8d` | a larva eats through the same diet filter as the mouth |
| *"Yes — let them starve."* | 2026-08-30 | E14 | brood can starve, visibly |
| *"We should also consider implementing eggs?"*, answered by `nest-biology` D11.1–D11.5: eggs first, keep budding's shape, re-derive the economy in the same breath, port the seed | 2026-09-19 | `nest-biology-2026-09-19.md` §11 | B1 is D11, and B3 is its dearer half |
| An ant should breed only at the nest, and in the end *where* it breeds should be something a lineage evolves (the report's paraphrase) | 2026-09-23 | `ant-scenes-2026-09-23.md` §9 | B1 makes nest-only laying geometrically possible. B1b's `Lay` output makes *where* heritable |
| *"Some ants should stay home… two different types of ants or castes"* → `caste=4`; *"Full granary on my default."* | 2026-09-28/29 | `Reports/lanes/nest-mouth.md`; `nest-granary-2026-09-28.md` | B4 replaces the by-id caste only when feeding-made castes supply at least as many home ants |
| *"In general, I prefer options on by default unless there is a good reason not to"*; *"You can ship everything on"* | 2026-09-27 / 09-12 | `CLAUDE.md` | every step ships on unless a harm is measured |
| *"It does not have to perfectly match how real ants, but we should take inspiration when we can"* | — | `ant-sim-research-review-2026-09-19.md` §2.13 | biology guides; legibility decides |
| *"Give me the tools, data, access to the parameters… That is the game."* | 2026-08-30 | coordinator note | every new constant gets a dial on the lab's parameter page |
| *"Not too many ants in your tests"*, *"snapshots at multiple times"*, *"just use a simpler test environment for now"*, *"we eventually do want our nests to work for larger colonies"*, *"track food in the room over time, not at a single instance"* | 2026-09-26/28 | nest lane | §6's protocol: the colony bed at 20 founders, plus 80 founders and `digbox` at 200 ants, read as time series |
| *"I cannot review the queue, post questions/images in this chat."* | 2026-09-29 | `claude/ant-nest-mouth-4f6s79`, lane note (unlanded) | post GIFs in the session chat, or on the queue where the owner reads it |
| A target of **60–70 evolutionary generations per session** | 2026-09 | `evolution-lab-direction-2026-09-09.md` L69 | every step reports `gen` and `bgen` in the lab. §7's per-game defaults exist because of this |

### 2b. Measurements that bound the design

- **Queen-only breeding: median generation 1.0 against 13.5; breeder chain
  median 0** (6 seeds × 120,000 frames, played bed).
  - "The evolutionary clock IS the queen replacement rate", so a
    better-provisioned queen makes it worse.
  - **Re-open condition:** "the moment the box has dispersal and colony
    competition" (breeding-clock §5).
- **Graded suppression: 8.5 generations, and a quarter of the deaths** (36
  against 146), with the same standing colony. That is a trade, not a free
  win.
- **The stamp is deferred, not removed.** Born-at-one-cell and fission routes
  gave 0 births over 12 seeds in 2026-08: *"the cell you do not buy at birth
  you must buy at growth"* (`dead-ends.md` L1770). Its bank ceiling is gone
  since Gate 0, but the arithmetic is not: **someone still pays ~960 J per
  adult**.
- **A floor is not a margin.** A bar at its floor with a cheap birth left 3 of
  12 seeds with one ant; a real margin (bar 200 against a grant of 80) gave a
  median of 228 (`dead-ends.md` L1798). **Read `live`, never `births`.**
- **`Energy` read against the breeding bar**: alive at 4,500 frames, 10
  against 46 — worse (README, Hunger). So the breeder's need is a **separate
  term** (breeding-clock §6.4; the `kin_deficit` doc).
- **Lifespan 40,000** is the smallest setting at which no colony dies inside a
  session; 20,000 kills 4 of 12. It is **not** a population brake
  (`evolution-lab-lifespan-rederived-2026-09-13.md`).
- **Per-ant cost** is about 2.1 µs per ant per tick in the lab
  (`evolution-lab-playtest-2026-09-13.md`) and 4.3 µs outdoors at 1024 wide
  (`ant-sim-research-review` §8.3).
- **The founding cliff.** 200 J at 0.0524 J/frame is 3,817 frames, so
  identical founders empty together (`colony-economy-design-2026-09-09.md`).
  The staggered reserves shipped because of it.
- **A generation is ~8,600–12,000 frames**, depending on the bed.

### 2c. Dead ends this plan re-opens, and what re-opens each

| entry | its re-test condition | met by |
|---|---|---|
| forage drive `hunger`/`larder` (L1194) | "a brood to feed, or births that keep the nest hungry" | B3 |
| `always,keep` (L1195) | "a colony has a use for stored food other than its foragers' bodies (brood, a queen)" | B3 |
| `Feed` AND wiring (L1185) | "births can be paid from a store in the nest" | B3 |
| `HAUL_BITE` (L1204) | "a birth paid from a store, or ants that stay home" | B3/B4 |
| `BUD_SITE=nest` (ant-scenes §9–10) | "a reason for a ready ant to go home, and a way for a child to fit … born small (an egg or brood)" | B1 |
| queen-only breeding (breeding-clock §5) | dispersal and colony competition | B6, with `scent_spread` 2.0 already in |

**Re-run each switch's own A/B when its condition is met.** Do not assume the
old verdict holds.

---

## 3. What real ants do, and what this plan takes from it

Evidence tags follow `nest-biology-2026-09-19.md`: **[measured]** is a study
retrieved for this plan (DOI in §11); **[repeated]** is widely stated, source
not held; **[general]** is this plan's own synthesis.

| real ants | evidence | here |
|---|---|---|
| One or a few queens breed. A queen pheromone suppresses workers' ovaries. In *Lasius niger* it is 3-methylhentriacontane, carried on the queen's cuticle **and on her eggs** | [measured] Motais de Narbonne 2016; the signal class is conserved across independent origins (Van Oystaeyen 2014) | B2: graded suppression by proximity to a breeder **and to her eggs** |
| Workers respond in their own interest (an honest signal), and when the queen is gone some become reproductive (gamergates in *Harpegnathos*) | [measured] Brunner 2011; Pask 2017 | B2's queenless colony resumes, which is the owner's ruling. How strongly a worker responds could become heritable later |
| Egg → larva (fed, grows) → pupa → callow adult. Warmth shortens development | [measured] Trigos-Peral 2024 (*L. niger*); [repeated] the external literature review §9 | B1 egg; B3 larva and pupa; temperature as B3's stretch |
| Claustral founding: a mated queen seals herself in and raises her first brood on her own reserves (histolysed flight muscle). The first worker came at week 6 in *L. japonicus*. Founding queens recycle brood when short | [measured] Kurihara 2022; brood recycling as the energetic threshold of founding (Wu & Feng 2026, a model) | B5's founding rule; B6's claustral founding from the founder's own bank; brood recycling through corpses (B3) |
| Found-or-fly: heavy-bodied queens found better and fly worse. A light queen raised a third of the workers but flew four times as long | [measured] Helms & Godfrey 2016 | B6: the reserve an alate carries is a heritable trade-off (stretch: flight cost rises with it) |
| **Larval nutrition decides female caste.** Carbohydrate-supplemented colonies raised more female sexuals | [measured] Bono & Herbers 2003; review, Richard 2021 | B4: `made` from how a larva was fed |
| Queens live about **10× longer than workers**, up to ~30 years | [measured] Jemielity 2005 | B7: deferred, because it must be priced |
| Young workers nurse deep in the nest and become foragers later. **The switch is stochastic and its probability is age-independent** (>500 ants, >100 days) | [measured] Richardson 2021 | B4: nest-bound young are released by a constant hazard, not an age cutoff |
| Callows are pale and the least active. Foragers go back to brood care when brood lacks nurses | [measured] Korczyńska 2014 | B4: callows drawn pale; brood is a stimulus |
| Brood is kept humid and warm and moved between chambers. The queen sits deepest; the brood cluster sets the chamber's size | [repeated] and [measured] `nest-biology` §2, §5; Franks & Deneubourg 1997 as cited there | B3: brood counts toward room crowding, so the nest grows with it. Brood carry is B3b |
| Nuptial flights are synchronised and weather-triggered (warm, humid, after rain). Most founding queens die | [repeated] | B6: the trigger runs through senses; outdoors the weather drives it; the lab needs another trigger (§7) |
| Males are haploid, from unfertilised eggs; queens store sperm | [repeated] | **not modelled** — the world stays asexual (B7) |
| Colonies also reproduce by budding: a party of workers leaves and founds | [repeated]; `evolution-lab-fission-design-2026-09-12.md` §2 | B6b: the same founding mechanism with a walking party, the lab's cheaper dispersal |

**The ratios carried into the constants** [general]:

- **Development is about a tenth of a worker's life.** Six weeks egg to
  adult, against a worker life on the order of a year. So egg-to-adult is
  about **4,000 frames** at the ant's 40,000-frame median.
- **A breeder would live about ten times a worker.** Deferred (B7).
- **A founding colony's first brood is a handful.** It is paid from one
  animal's reserve.

---

## 4. The design in one page

```
 breeder = any animal with children > 0 (existing). Fed at home by nestmates;
 nest-bound for life if the species' founding rule made her (B5)
        │  lays: one `brood` cell beside her, paying the egg price
        ▼
 EGG    pale cell, organism-owned, `CellType::Seed`, Powder, rolls: false
        │  egg_frames; carries the fertility signal (B2); can be lost
        ▼
 LARVA  cream cell; a bank that nurses fill through Share, plus food
        │  beside it eaten at its tick; upkeep, so it can starve
        ▼  bank reaches its own target (stamp + grant)
 PUPA   tan cell; no feeding; `made` fixed from how it was fed (B4)
        │  pupa_frames, and room for the adult body
        ▼
 CALLOW adult body, drawn pale, nest-bound until a random release (B4)
        ▼
 WORKER / BREEDER, by what `made` expressed and by graded fertility (B2)
        │
 colony level (B6): a big, fed colony provisions high `made` → (Made, Fly)
 alates → leave on a warm wet day → land ≥ found_distance from any nest →
 found (door + claustral cut, claim_colony, colony_parents) → her first brood
 from her own bank
```

**Mechanism, policy and constants, kept apart as the owner's rule asks:**

| | code (mechanism) | genome (policy) | species file (constants, dials on the lab page) |
|---|---|---|---|
| laying | an egg cell, not a body | how rich before laying (`TRAIT_REPRODUCE_AT`); where (B1b `Lay`) | `egg_cost`, `egg_frames` |
| fertility | the bar scaled by distance to breeders and eggs | — (later: response strength) | `GRADED_MAX_SUPPRESSION`, radius |
| brood care | a larva has a bank and a target; `KinNeed` reads it | `Share` weights; `Provision` weights | `larva_upkeep`, `pupa_frames` |
| caste | `made` = joule-weighted `Provision` of whoever fed it | developmental block; `(Made, verb)` weights | plasticity (1.0) |
| callows | nest-bound with a random release | (later) release rate via `made` | mean dwell |
| founding | founding rule; a founding verb | `(Made, Fly)`, `(Made, Found)` | founding reserve, `found_distance` |

**The economy across the cycle.** No step creates energy, and each has a
ledger row.

| event | who pays | ledger |
|---|---|---|
| lay (B1) | the layer pays the whole adult price (1,040 J) into the egg's bank | live → live |
| lay (B3 on) | the layer pays `egg_cost` (first guess 120 J) into the egg's bank | live → live |
| a larva is fed | nurses through `Share` (live → live), or food beside it eaten at `diet_yield` | the existing harvest accounts |
| pupate → hatch | the larva's own bank: the stamp moves bank → `StoredInMeat`, and the remainder is the adult's first bank | as a bud's stamp today, one step later |
| brood dies | its bank and the stamp so far become a corpse, food, as any death | as today |

---

## 5. The steps

Each step is written as follows:

- **what it does**, in the world's terms;
- **mechanism**, by file and function;
- **switch and default**;
- **economy**;
- **counters**, with the effect counter on the far side of the call;
- **guards**, each of which must go red when its fault is put back;
- **measurement**;
- **what the owner sees**;
- **gate to default-on**;
- **traps**;
- **size**.

§6's shared protocol applies to all of them.

### B0 — Before anything: the crash, the owed sweep, the stale clock

**What it does.** It stops the held world crashing when an ant in a high slot
cannot fit a child, and it takes the two numbers the owner's rulings made
prerequisites for graded breeding.

- **B0a, the crash (`world.rs`).**
  - Make `denied_seen` a `Vec<u64>` grown to `slot / 64 + 1` on first write,
    or a `FxHashSet<OrganismId>`.
  - Correct the three stale 4,095 comments (§1d).
  - Guard: `note_birth_denied` on slots 4,095, 4,096 and 1,000,000 counts each
    animal once. The positive control is the panic above. B1's hatch-refusal
    bitset must reuse the same structure.
- **B0b, the switch in reach of a guard.**
  - Add `World::breeding: Option<(BreedingRegime, radius, max)>` overriding
    the environment for one world, with the `World::bud_at_nest` pattern, so
    both arms run in one process.
  - Read `GRADED_MAX_SUPPRESSION` from the dial.
  - Accept `PIXEL_PHYSICS_BREEDING=graded:<max>`.
- **B0c, the reference.**
  - `how-the-ant-works.md` §9 and §12 now carry `PIXEL_PHYSICS_BREEDING` and
    `_RADIUS`, added in the commit that carries this plan.
  - Add the `World` field when B0b lands.
- **B0d, re-take the breeding clock on today's `main`.**
  - Arms: individual, graded (6.0), queen.
  - **12 seeds, not 6.** *"Six seeds is not a sweep"* (`CLAUDE.md`).
  - Beds: the played bed (`labforage scenario=played_bed`), the lab box
    (`labstats`), and the colony bed at 20 and 80 founders.
  - Frames: 120,000 and 300,000.
  - `RAYON_NUM_THREADS` pinned.
  - Columns: `ANIMALS BORN`, `live`, deaths by cause, `gen`, `bgen`, `brdr`.
  - This is the baseline every later step is read against. The 2026-09-10
    numbers predate #366 and §Z36.
- **B0e, the owed sweep.**
  - `GRADED_MAX_SUPPRESSION` ∈ {2, 3, 6, 12} × radius ∈ {12, 24, 48}, same
    beds, 12 seeds.
  - Add one column: **the top breeder's share of each colony's births**. It is
    the concentration readout, and a distribution rather than a count.
  - Put the table to the owner, who picks.
- **B0f, the readout later steps need.**
  - Extend `labforage` and `labstats` with per-colony age structure (adults by
    age band), births by the parent's distance to its nest, and the top
    breeder's share.
  - Leave room for brood-by-stage columns.
  - Print the key's cardinality against what the run swept (*"a parse is a
    measurement"*).

**Size:** one session. **Files:** `world.rs`, `creature.rs` (the regime from
`World`), `examples/labforage.rs`, `src/lab/stats.rs`, the reference.

### B1 — The egg: a birth that starts small

**What it does.** An ant ready to breed lays one egg beside itself instead of
a whole ant. The egg lies where it lands. After a delay it hatches into a young
ant, as soon as there is room for the body. Until then it can be lost: eaten by
another species, burned, or buried.

**Why first.** `nest-biology` D11.1–D11.5 and the owner's question. It is also
the only change that makes nest-only breeding possible at all, because **one
cell fits where two in a line never do** (§1b, `BUD_SITE`).

**Mechanism.**

1. **The material.** `assets/materials/brood.ron`, one material for every
   stage; B3's stages are shades, so B3 adds no material.
   - `kind: Powder`, `rolls: false` and `falls_through_organisms: true`, as
     `crumbs`: it drops straight down through open air and bodies and
     otherwise stays where it is put.
   - A pale palette.
   - Laid on a breeder's back it falls to the floor. Laid over a shaft it falls
     down the shaft. That is *"an egg rolling down a shaft into the open is
     exactly the graded, visible failure Law 1 asks for"* (`nest-biology`
     §11.3).
2. **The organism.** Laying allocates the child's organism at once, as
   `plant::bear_seed_at` does at seed set.
   - Write genome, traits, fates, `made`, lineage, colony and generation
     **exactly as `Origin::Bud` does now**. Factor that block out of
     `place_creature` into a helper both paths call.
   - Mutate there, on the child's handle (`RNG_SLOT_BIRTH`), so **heredity does
     not change by a draw**.
   - The organism owns one `brood` cell whose `aux` packs `CellType::Seed`, so
     `World::set` keeps its cell list current under both drivers and a
     `relocated_seed`-shaped lookup finds it wherever it fell.
   - **`chain` stays empty until it hatches.** Every creature loop that reads
     a head position through `chain.first()` then skips an egg by
     construction.
   - New state: `OrganismState::brood: Option<Brood>`, with `stage`,
     `stage_since` and `target`. `None` means an adult or a plant, so nothing
     existing changes.
3. **Where.** The first of the eight neighbours, in `DIRS` order, that is
   empty. One cell is placeable in any of them.
   - No room → `note_birth_denied`, as today.
4. **The tick.** Schedule the egg as `ActiveKind::Creature` at `frame +
   egg_frames`.
   - In `creature_tick`, **before `reconcile_chain`** (which would kill an
     organism with an empty chain), send a brood organism to `brood_tick` and
     return.
   - `brood_tick` finds its cell. If the cell is gone, the egg was destroyed:
     free the organism and count `eggs_lost` by cause. If the egg is due, try
     to hatch. Otherwise reschedule for when it is due.
   - **An egg costs nothing per frame.** It costs one dispatch per hatch
     attempt.
5. **Hatch.** `place_creature(…, Origin::Hatch { organism })`, a new arm that
   **reuses the organism** instead of calling `push_organism`:
   - clear the brood cell;
   - lay the body with its head on that cell through `founding_spine_walk`,
     which curls the spine to fit;
   - `brood = None`, `hatched_frame = frame`, `forage_anchor` = the head,
     `since_nest = 0`.
   - **No room → wait.** Count `hatches_denied_no_room` (attempts, plus
     distinct eggs through B0a's structure) and retry every `HATCH_RETRY`
     (60) frames.
6. **Old age** reads `frame − hatched_frame` where that is set, and
   `born_frame` otherwise.
   - The 40,000 calibration stays an adult lifespan.
   - `born_frame` keeps its job as the collision-proof identity the lab roster
     pins.

**The first trap: an egg is an organism-owned `Powder`, and four verbs treat
one as something else today.**

- **The mouth.** A seed is food. `adjacent_food_counted` must treat a brood
  cell as **kin** for `is_living_kin_id(world, id, gut)` (not food, so no
  nestmate eats its colony's eggs) and as **prey** for everyone else, priced
  at the egg's bank.
- **The jaw.** `jaw_can_cut` refuses Creature- and Plant-kind cells and "a
  live seed". Add "live brood" beside the live seed, or the digger clears eggs
  as spoil. **Round 28 found exactly this with a pip.**
- **The founding cut.** `is_diggable_ground` must refuse a brood cell as it
  refuses a corpse.
- **Footing.** `spoil_site_open` and `is_footing` must not read a brood cell as
  ground to stack a pellet on.

Each gets a scene test (guards below).

**The second trap: the census — eggs are organisms of the ant species.**

- `live_organism_ids()` has **222 callers** in `src/` and `examples/`.
- `live_creature_groups` has **36**.
- **57** sites filter on `.creature.is_some()`.

A count-only loop over creature organisms will **count eggs as ants**. That is
`CLAUDE.md`'s *adding a member to a set something sweeps enrols it in every
rule over that set* at scale. The empty `chain` protects the positional loops
and not the counting ones.

The work, **budgeted inside B1, not after it**:

1. Add `OrganismState::is_brood()`.
2. Make `live_creature_groups`, the lab's population strip, `nest_needs`
   attribution and the `CreatureStats`-fed readouts read adults only. Give
   brood its own column instead.
3. Grep the 57 + 36 sites and fix each counting one.
4. Guard: **adding brood to a bed changes no adult census**. Snapshot every
   census function on a bed, add eggs, and compare. The positive control is
   removing one `is_brood` filter and watching the guard go red.

**If that audit is unaffordable,** the fallback is brood as **records, not
organisms**: a `brood` cell whose `aux` holds an id into `World::brood`, a
table carrying the genome and bank. `rolls: false` means it only ever falls
straight down, so the record re-finds it by scanning its own column. That is
exact, because nothing moves it sideways except a carrier, which updates the
record.

It avoids the census entirely and costs three things:

- a brood term in the ledger's live total;
- a brood target type in `Share`;
- the kin, food and jaw exclusions keyed on the material rather than on an
  organism.

**Recommended: organisms.** It is `nest-biology` D11.4's route. The genome,
energy, kin and `Share` code apply unchanged. And the census audit is work the
repo will need the first time anything non-adult shares a species anyway.

**Switch.**

- `PIXEL_PHYSICS_EGG` = `off` | `<egg_frames>`, with `World::egg:
  Option<u32>`.
- The species field `brood: (egg_frames: …)` is absent → today's budding.
  **Every species that does not author it** — beetle, worm, flitter, hopper,
  the lab ancestor — **is untouched by construction**, the `Individual` arm's
  "provably today's code" pattern.

**Economy (D11.2, keep budding's shape).**

- The layer pays the **whole** adult price at laying, into the egg's bank
  (live → live).
- At hatch the stamp moves bank → `StoredInMeat`. The meat census therefore
  never sees a one-cell egg carrying two cells of meat.
- The only reallocation is the **delay** and the **loss**.
- D11.3's re-derivation is budgeted here: `reproduce_threshold` /
  `TRAIT_REPRODUCE_AT` and the lifespan constants are re-read against the
  sweep below. If B1 moves `live`, the constants are re-derived here, not in
  a later session.

**Counters decided up front, because readouts key on them:**

| event | counts |
|---|---|
| laying | `children` and `life.offspring` (the parent has bred, so suppression reads it at once), `eggs_laid`, `deepest_breeder_generation` |
| hatching | `CreatureStats::births`, `deepest_animal_generation`, `LogKind::Born`, `note_line_population(+1)` — **"animals born" keeps meaning adults that appeared** |
| always | `eggs_lost[cause]`, `hatches_denied_no_room` / `_animals`, eggs standing (census), a laid → hatched wait histogram, all per colony |

**Guards** (breeding tests pin `mutation_rate = 0.0` on their species copy,
`dead-ends.md` L1158):

1. The switch off (and a species with no `brood`) runs no new code: the branch
   returns first. A bed's hash with the switch off matches the same bed with
   `World::egg = None`.
2. Conservation: `expected_live_total() == Σ live energies` through laying,
   through hatching, and through an egg eaten by a beetle.
3. An egg in a sealed chamber hatches at `egg_frames` to within
   `HATCH_RETRY`. One with no room waits and is counted, then hatches once
   room is made (the positive control).
4. Heredity: the hatched animal's genome equals the egg's, so there is no
   second mutation.
5. Both drivers: an egg laid over a shaft lands on the same cell under
   `update::step` and `parallel::step`.
6. The four verbs: a nestmate never eats its colony's egg, and a beetle does.
   A digger never cuts one. The founding cut and the spoil drop leave it
   alone.
7. The census guard above.

**Measurement.**

- Arms: `egg_frames` ∈ {off, 600, 1,500, 3,000}. The biology in §3 puts the
  egg stage at about a third of the ~4,000-frame egg-to-adult, so **start at
  1,500**.
- Re-run `BUD_SITE=nest` with eggs on. It failed on geometry, and geometry is
  gone.
- Also census **where eggs end up**: roofed or open, in the founding cut or
  outside.

**What the owner sees.** A GIF crop of the founding chamber on the colony bed
(`filmstrip gif=1`, or `labgif` in the lab): pale eggs appearing beside
breeding ants, settling on the floor, and hatching. `meta`: eggs laid /
hatched / lost / standing, and births with eggs off, as the paired number.

**Gate.**

- **Outdoors**: `live` and starvation within the arm's own noise on the colony
  bed (24 paired seeds), and no `ascii` worst-frame regression.
- **Lab**: the `gen`/`bgen` cost stated, and the owner rules (§10 Q1).

**Docs, same commit:** the reference (§1 step 9, §9, §12), `wiki/ants.md`
("New ants, and why you will not see any yet" becomes eggs, with a freshness
note), README's creature status, and the index.

**Size:** ~600–900 lines with tests; 1–2 sessions.

**B1b (optional, only on the owner's word): `BrainOutput::Lay`.** Laying
becomes a won roll against a brain output wired `(AtNest, Lay, +)` and `(Bias,
Lay, −)`, so *where* an ant breeds is heritable, which is the owner's
2026-09-23 wish.

It is **a genome append**:

- every seeded draw shifts;
- `mutation_rate` is re-derived (round 15: an output row costs 33 live slots);
- every breeding test flips unless it pins `mutation_rate`.

### B2 — Graded fertility on: the colony finds its breeder

**What it does.** The ant that is breeding makes breeding harder for ants near
it. Far from any breeder, or once a breeder is dead and her eggs have hatched,
others breed again.

- In a nest this concentrates breeding on one or a few animals, which is the
  colony finding its breeder.
- Her nestmates, suppressed, bank what they would have spent on young. They
  become exactly the rich donors `Share` moves food downhill from.

**Mechanism.**

- `graded` exists. Ship B0e's winning `(max, radius)` as the ant's defaults,
  as dials on the lab page.
- **Eggs as a signal source**, as *L. niger*'s queen pheromone is on her
  eggs:
  - `nearest_breeder` also considers the colony's standing brood;
  - a per-colony brood candidate list works like `colony_breeders`: pushed at
    laying, validated live on read, never trusted, and with nothing on any
    death path;
  - effect: **a queenless nest full of brood stays suppressed until the brood
    hatches**, a graded delay rather than an instant resumption.
  - Measured as its own arm (`graded` against `graded,brood`).
- **The breeder, readable on screen** (§10 Q2). A render readout, not an
  engine fact: any animal with `children > 0` gets a mark under a render
  toggle, and the CELL page shows "BRED n".

**Economy:** unchanged. Watch where the suppressed workers' surplus goes, in
`colonybooks`.

**Counters:**

- `births_suppressed` (attempts, plus distinct animals);
- breeders per colony;
- **the top breeder's share of births**;
- breeder tenure (frames from first laying to death);
- `gen` against `bgen`.

**Guards:**

- `graded_regime_scales_the_bar_and_a_queenless_colony_resumes`, which already
  exists.
- A queenless chamber of eggs holds a worker suppressed until they hatch.
  Removing the eggs releases it at once (the positive control).
- **The ruling's principle check**: max 1.0 is bit-identical to `individual`.

**Measurement.** B0d's beds; arms individual / graded / graded,brood.

**What the owner sees.** A pair: individual against graded on one seed, the
nest crop with breeders marked. `meta`: breeders per colony, the top breeder's
share.

**Gate.** The owner's ruling ("I lean graded") plus the sweep → on for nesting
species outdoors. The lab's default waits on §10 Q1.

**Size:** small, given B0b and B0e.

### B3 — Brood that must be fed: larvae, nurses, and the nest's purpose

**What it does.** An egg now hatches into a larva: a grub that cannot move and
must be fed before it can become an ant.

- Ants bring food to the brood pile and pass it mouth to mouth.
- A well-fed pile becomes tan cocoons and then young ants quickly; a hungry
  one does so slowly.
- In a famine larvae die and are eaten, so **the colony shrinks from its brood
  before it starves its workers**, which is graded rather than a cliff.
- **Laying becomes cheap, and growing an ant is what costs the colony.** A
  breeder can lay far more eggs than the colony can raise, and **the
  colony's income decides how many it does** (`creature-reproduction-economics`
  §3.3's mass provisioning, charged through `diet_yield` as its §6 insists).

**Mechanism.**

- **Stages** are Egg → Larva (at `egg_frames`) → Pupa (when fed to target) →
  adult (after `pupa_frames` and room). The stage sets the brood cell's shade:
  egg white, larva cream, pupa tan.
- **The larva's bank** fills three ways, and none of them is new plumbing:
  1. **`Share` from adjacent adults**, live → live, downhill only (a quarter of
     the gap). Only richer ants feed a larva, and B2's suppressed workers are
     the rich ones.
  2. **Food beside it**, eaten at its stage tick through `diet_yield` and
     `EAT_YIELD_THRESHOLD`. This is a second mouth, so it takes the same diet
     filter (`dead-ends.md` L1169: `try_bud`'s top-up was a second mouth once
     already).
  3. Nothing else.
- **Upkeep.** The larva pays the adult's idle cost for one cell (first guess),
  so an unfed larva starves: `DeathCause::Starved`, and a corpse that is food.
  **Brood recycling then emerges** from corpses being eaten, as founding
  queens recycle brood (Wu & Feng 2026).
- **`KinNeed` reads each kin against its own target.**
  - An adult: the donor's `start_energy`, **exactly as today**.
  - Brood: its pupation target.
  - No existing scene contains brood, so every existing reading is unchanged,
    and a hash guard proves it.
  - This is not the `kin_deficit` doc's forbidden change ("never a change to
    the plain energy fraction"). That doc was protecting *adult* need from
    being re-read against a breeding bar, and adult need does not move.
- **The target** is the stamp (960) plus the grant the layer's
  `TRAIT_BIRTH_GRANT` names, fixed at laying. **`TRAIT_BIRTH_GRANT` keeps its
  meaning** (what a newborn adult starts with). The layer now pays
  `egg_cost`, and the colony pays target − `egg_cost`.
- **Room.** Brood cells count as occupants in the nest-room crowding
  (`Crowding` at the nest). A growing pile raises the dig urge, so **the nest
  grows with its brood** — the cluster-sets-chamber rule, emergent.

**B3b — the rescue.**

- A nest-bound adult beside brood that is open to the sky, or outside the
  founding cut, picks it up. It rides the mandibles the way a
  `SeedPassenger` rides the crop, and a `carried_brood` set keeps its id
  alive, as `carried_seed_organisms` does.
- It walks it back through the storeroom carry's target and pull machinery
  (`store_target`, `home_pull`) and puts it down under cover.
- **This is the player's verb**: dig into a chamber and the colony carries
  its brood away. It rides an existing output roll, as the storeroom carry
  does.

**Stretch — warmth.** Development time is scaled by the field temperature at
the brood cell (Trigos-Peral 2024). It would be the first consumer of the
depth-graded temperature field (`nest-biology` D5.1).

**Economy re-derivation — the gate on whether B3 is scoped at all
(`nest-biology` D11.3).**

- Sweep `egg_cost` ∈ {80, 160, 320} × larval upkeep ∈ {0, ½, 1 × adult idle}.
- Keep `reproduce_threshold` a **margin** over `egg_cost`, never the floor
  (L1798). At 1,100 against 120, the breeder keeps a ~1,000 J reserve and lays
  about one egg per 120 J above it.
- Judge on `live` and on the p10 of colony survival at 300,000 frames,
  24 seeds. **Not births.**
- **Watch for a correct mechanism at inherited constants reading as "brood
  killed the colony".**

**Re-run the four dead-end switches** of §2c with brood on.

**Counters:**

- joules into brood by channel (shared, eaten, laid);
- larvae starved, pupated and hatched;
- brood standing by stage;
- brood per adult;
- the egg → adult time histogram;
- brood recycled (joules of brood corpse eaten);
- an "into brood" line in `colonybooks`.

**Guards:**

- **Conservation** through lay → feed → pupate → hatch, and through a larva
  starving.
- **`KinNeed` unchanged** on a brood-free bed (hash).
- A larva between two rich nurses is fed and pupates (the positive control).
  One with no nurse and no food starves, and a nestmate eats the corpse.
- A larva of a plant-specialist gut ignores meat beside it.

**Measurement.**

- The colony bed at 20 and 80 founders, the lab box (24 seeds), and `digbox`
  at 200 ants, to 300,000 frames.
- **A famine scene**: food removed at frame N, and brood against adults read
  as a time series.

**What the owner sees:**

- the chamber GIF: a three-colour pile, nurses crowding it;
- the famine run, where the pile thins first;
- the rescue: a scripted cut into the chamber, with ants carrying brood away.

**Size:** the largest; 2–3 sessions.

### B4 — What the young become: callows, and castes from feeding

**What it does.** Newly hatched ants are pale and stay home, tending the brood
and the store. Each one turns into a forager at a random moment. How well a
larva was fed decides what it becomes: a richly fed one becomes an ant that
breeds readily and stays home; a thinly fed one becomes a worker that goes
out. A lineage can change all of this.

**Mechanism.**

- **`made` at pupation** is the joule-weighted mean of the `Provision` outputs
  of the animals that fed it. The larva keeps `made_sum` and `made_weight`.
  - **This is the parent channel generalised**: whoever feeds provisions, and
    their brains decide what they make.
  - The existing developmental block turns `made` into expressed traits.
- **Authored in `ant.ron`**, as the founder genome's first guess:
  - `Provision` wiring, e.g. `(Energy, Provision, +a)` and `(Bias, Provision,
    −b)`, so a nurse in a well-fed colony provisions high. That is Bono &
    Herbers' carbohydrate result in one wire.
  - Developmental weights: `TRAIT_REPRODUCE_AT` down with `made` (fertile when
    richly reared) and up when thin (a bar so high it is in effect sterile);
    `TRAIT_CROP_CAPACITY` up when thin (a forager).
  - **The principle check** (the ruling): zero the `TRAIT_REPRODUCE_AT`
    developmental weight and workers bud as in B2.
- **`(Made, verb)` weights** make castes *behave* differently, not only breed
  differently. For example `(Made, Move, −)`: a high-`made` animal stays put.
  **Per-caste verb counters come before any claim**, as breeding-clock §6.6
  asks: attacks, deliveries, shares, digs and lays, by `made` bucket.
- **Callows.** A hatched ant is nest-bound, released by a constant per-tick
  hazard. Stamp `nest_bound_until = frame + Exp(mean)` from its own stream at
  hatch; the first guess for the mean is `NEST_BOUND_FRAMES` 8,000.
  **Richardson 2021 measured the transition probability as age-independent**,
  and a random dwell is a distribution, not a cliff.
- **Render callows pale** for their first ~2,000 frames: a renderer readout by
  age since hatching. Cheap, legible, and true.
- **Replacing `caste=4` by id.** Only when the feeding-made castes plus B5's
  founding rule supply at least as many home ants, at founding and after, as
  `caste=4` does. The by-id caste exists because age-based home ants once gave
  "2–6 home ants and none before the first birth" (`nest-granary` §8b, §8h).
  Until then both run.

**Guards:**

- `made` from two nurses with known `Provision` outputs equals their
  joule-weighted mean, exactly.
- The principle check (hash against B2).
- The callow dwell's mean is within tolerance over N ants.

**What the owner sees.** A nest crop at three times: pale callows inside, dark
foragers outside, and the ratio moving with the colony's food. `meta`: home
ants, callows, and `made` by bucket.

**Size:** 1–2 sessions.

### B5 — The founding rule, and the breeder who stays home

**What it does.** A colony starts the way its species says. For the ant that
is one well-provisioned breeder in the founding chamber, with a cohort of
workers and a little brood, instead of 52 identical strangers. The breeder has
no reason to walk — she is home and she is fed — so she stays in the chamber,
laying, and the colony's size follows its food from then on.

**Mechanism.**

- **`CreatureDef::founding`** in `ant.ron`. First guess: `(breeders: 1,
  breeder_reserve: 3000, cohort: 51, brood: 8)`.
  - The cohort keeps `founder_reserve_spread`.
  - **The `Y` key still places 52 animals.** *"Fifty looks like a colony,
    five looks like a bug"* (`ant-sim-research-review` §2.1).
- **`found_colony_with`** places the breeder in the entrance chamber the
  founding cut already digs, nest-bound for life (`nest_bound_until =
  u64::MAX`, the existing mechanism).
  - She becomes the breeder by laying first, which her reserve makes
    immediate. **There is no flag.**
  - Her reserve is booked `Granted`, as every founder's grant is.
- **Her need.** Only if tracing her shows she cannot keep laying. Then use the
  `kin_deficit` doc's extension point, "a breeder close to its breeding bar
  with a thin bank should read as needier", as a **separate term**: `KinNeed`
  for a kin with `children > 0` reads against her own bar. **Measure first.**
- **Queenless.** If she dies, graded fertility lets the next animal at home
  that clears its bar take over, which is the existing rule. *"Her death is
  graded only if her workers outlive her — check by playing"*, so the time to
  the next breeder and the dip in births are measured.

**Counters:**

- **a per-tick trace of the breeder** — the owner's rule for "why" questions
  (`CLAUDE.md`, 2026-09-20), extended with her bank, laying and position;
- her bank over time;
- laying rate;
- successions.

**Guards:**

- A founding places exactly one reserve-holding breeder in the chamber, and
  the books close (the shape of
  `a_founding_that_places_fewer_founders_than_planned_still_closes_its_books`).
- She lays first.

**Measurement:**

- deaths per 500-frame window across the first 10,000 frames (the cliff
  should be gone);
- the colony's growth curve (sigmoid, not a cliff);
- breeder tenure.

**What the owner sees.** A time-lapse of one founding: the breeder in her
chamber, eggs gathering, the first hatchlings.

**Size:** one session. **Coordinate with the nest lane**, which owns founding
(§8).

### B6 — Colonies that found colonies: nuptial flight and claustral founding

**What it does.** When a colony is big and has food to spare, some larvae are
raised as breeders with wings. On a warm day after rain they climb out and fly
off together. Each one that lands far enough from any nest drops its wings,
digs a small chamber and raises its first few workers alone, from the reserve
it carried. Most die. A few become new colonies, and the world fills with
rival colonies descended from one another.

**This is the step that lets breeder-centred breeding pay.** An evolutionary
generation becomes *a colony founding a colony*, which is the breeding clock's
own re-open condition.

**Mechanism.**

- **Alates are a caste, not a type.** The ant species authors
  `fly_cost_in_moves` and `cruise_lift`, so flight becomes *possible*. Only a
  brain with a `Fly` urge uses it. Workers carry no `Fly` weight; a
  `(Made, Fly, +w)` weight lets high-`made` adults fly.
  - The trigger runs through senses: `(TempAboveAmb, Fly, +)`, and moisture.
  - **Outdoors, `weather::at(seed, frame)` is the weather**, and rain wets the
    ground the moisture sense reads.
  - **The lab has none** (`Pin::Clear`), so see §7.
- **One founding mechanism, two policies.** A landed animal with a bank at or
  above the founding reserve, at least `found_distance` from every nest site
  (the fission design's 120 cells in the 512 box), on ground
  `colony_ant_site` accepts, founds:
  - `paint_nest_patch_with` and `dig_founding_shaft` at her feet;
  - `claim_colony`, plus `colony_parents.push((new, parent))`, so the page
    names it `ANT 1b`;
  - nest-bound for life, which is B5's rule applied to her.
- The **trigger is a brain output**, the fission design's `Leave` (slot 16),
  because *"no creature verb founds a nest"* today and a rule that decides
  when to leave would be a behaviour. It is a genome append, priced as in B1b.
  - **Party size 1 with flight is the nuptial founding. Party size 8 walking
    is the fission design's budding (B6b)**, which is the lab's cheaper
    dispersal. One mechanism, two dials.
- **The claustral first brood emerges.** She lays from her own bank (B3's
  cheap eggs) and feeds her larvae by `Share`, as the only kin beside them. A
  founding succeeds only if her reserve covers the first few adults' price
  (~1,040 J each), which is the energetic threshold Wu & Feng 2026 model. So
  **most fail without any rule saying they should**.
- **Found-or-fly (stretch).** Flight cost scales with the bank carried above
  `start_energy`, so a heavy alate founds better and flies worse, as Helms &
  Godfrey 2016 measured. That makes how much to provision an alate a real
  trade.

**Colony competition is already in** (`scent_spread` 2.0: strangers).
`conflict_arena` and `rivalry` read it.

**Counters:**

- alates reared and flights;
- distance flown and landings;
- foundings attempted, and foundings still alive at +20,000 frames;
- colony-tree depth (via `colony_parents`);
- `gen` and `bgen`.

**Guards:**

- A founding by one animal closes the books.
- No founding happens within `found_distance` of a nest.
- Flights and landings are identical under both drivers.
- **Queen-only's re-run**: with dispersal on, re-take breeding-clock §5's
  sweep, as its own condition demands.

**Measurement.**

- Outdoors, `seedsweep.sh` over presets and seeds, to 300,000–500,000 frames:
  colonies over time, tree depth, and frame cost (`ascii` worst frame and
  `antcost` at the populations reached). **More colonies means more ants, and
  frame cost is a hard constraint.**
- **The limits must be food and room. Never a cap that decides whether a
  founding happens** (`CLAUDE.md`'s size-cap rule).

**What the owner sees:**

- the swarm: alates rising off the mound after rain, as a GIF;
- a wide strip of new mounds appearing across the world.

`meta`: flights, foundings, colonies alive.

**Size:** 2–3 sessions; the largest risk.

### B7 — Deliberately not in this plan

- **Mating, males, sperm storage, recombination, haplodiploidy.**
  - *"Mating in the world: hard dead end (asexual budding is the
    isolation)"* (`creature-direction.md` §3). With no gene flow, lineages
    diverge (`creature-evolution-plan.md` §6). `CROSS` lives on the specimen
    shelf.
  - **Re-open only on the owner's word.** The cost is a second 12,416-float
    genome per mated breeder (~50 KB), male alates, and a mating event.
- **Breeders that outlive workers** (~10× in ants, Jemielity 2005). There is
  no role-based lifespan without a type. Two engine-native routes exist, both
  to be priced first:
  1. a longevity slot the developmental block moves with `made`, with a
     standing cost like armour's, or it ratchets;
  2. wear-based ageing (age accrues with activity), which re-calibrates every
     ant.

  Decide after B5's succession numbers say whether breeder churn matters.
- **A bigger queen body.** Bodies longer than two cells do not survive
  (`wiki/ants.md`; `creature-articulated-body-2026-09-09.md`), and heritable
  body size ratchets (`creature-reproduction-economics` §5.4). B2's render
  mark is the substitute.
- **A population cap.** Never. Food and room are the limits.

---

## 6. How every step is measured

- **Beds.**
  - The colony bed at **20 founders**, the default, and at **80**.
  - The lab box: **24 seeds**. The pre-ship check, with births as one gate.
  - `digbox` at **200 ants**.
  - The played bed (`labforage scenario=played_bed`) for the clock.
  - Outdoor presets through `seedsweep.sh` for B6.
- **Horizons.** 120,000 frames for clock comparisons; **300,000 for
  survival** (§Z6's bar); B6 to 500,000.
  - **Read time series, not instants** (owner, 2026-09-28): snapshots at
    several frames.
  - **Run cascades to rest.**
- **Arms in one process**, through `World` fields (`egg`, `breeding`, …).
  **Never two builds.** Pin `RAYON_NUM_THREADS` for any counter downstream of
  `parallel.rs`. Brood is a `Powder`, so every brood guard runs **both
  drivers**.
- **Readouts.**
  - `live`, never `births`; `ANIMALS BORN`, never `BIRTHS`.
  - `gen` **and** `bgen`.
  - The top breeder's share.
  - Each new counter's positive control: e.g. `eggs_lost` must read non-zero
    in a scene with a beetle among eggs.
  - The key's cardinality printed against the sweep.
- **"Why" is a trace, never a split** (`CLAUDE.md`, 2026-09-20). Extend the
  decision trace with `brood.stage`, `made` and the breeder flag, and trace
  individuals: the breeder, a larva, an alate.
- **Frame cost.**
  - `ascii`'s worst frame, with the mean × frames ≈ worst check.
  - `antcost`'s per-ant slope.
  - Paired and alternating runs, on the whole frame.
- **Tests whose animals breed pin `mutation_rate = 0.0`** on their species
  copy.
- **Run all of `cargo test`.** `tests/*.rs` holds the preset and worldgen
  guards that sweep registries, and `--lib` cannot reach them. **A new
  material is a new registry member**: grep what enumerates materials by name
  (`paintable()`, census tables) before adding `brood`.
- **Pictures go to the owner**, in the chat or on the queue, **with the
  discrete counts in `meta`**, as a paired comparison.
- **Docs, same commit:**
  - `how-the-ant-works.md` for any mechanism it describes;
  - `wiki/ants.md` with a dated freshness note;
  - README status;
  - `Reports/README.md`;
  - `dead-ends.md` for anything reverted.

---

## 7. Per-game rollout

| step | outdoor sandbox (`Y`) | evolution lab | held world (`--bin druid`) |
|---|---|---|---|
| B0 | the fix lands everywhere | the clock re-take is the lab's | **the crash is most likely here** |
| B1 egg | on | on if the owner accepts the measured clock cost (§10 Q1); otherwise a short `egg_frames` | on. `C` founds through `found_colony_of`, so it inherits it |
| B2 graded | on | owner's call on the re-taken clock | on |
| B3 brood | on | on; every constant is a dial on the parameter page | on |
| B4 castes, callows | on | on | on |
| B5 founding rule | `Y` = 1 breeder + 51 + a little brood | colony entries in scenarios carry the rule | the `C` offer gets a breeder option, priced from the pool |
| B6 dispersal | nuptial flights, weather-triggered | the walking party (B6b), or a state trigger (§10 Q4) | flights |

**The one real divergence risk is constants, not code**
(`two-games-one-repo-2026-08-30.md` §4). Every brood timing goes in the
species file first, then the `tunables` registry, and only then a per-world
block, in that order.

---

## 8. Who owns the files, and how to run it

- **Files by step.**
  - `creature.rs`: `try_bud`, `place_creature` (`Origin::Bud` / `Hatch`),
    `creature_tick`'s brood branch, `act`'s share branch, `kin_deficit`,
    `adjacent_food_counted`, `jaw_can_cut`, `found_colony_with`.
  - `organism.rs`: `Brood`, `hatched_frame`.
  - `world.rs`: the bitset, the `World` fields, counters, candidate lists.
  - `brain.rs`: only for `Lay` / `Leave`.
  - `render.rs`: brood shades, pale callows, the breeder mark.
  - `assets/materials/brood.ron`, `assets/species/ant.ron`.
  - `src/lab/params.rs` and `src/lab/stats.rs`.
  - The examples named above, and the docs.
- **`creature.rs` is contested.** The nest lane owns founding, nest shape, the
  home test and dig/spoil. The foraging lane owns the walk, crop, drop and
  larder, and built `BUD_SITE` and `BIRTH_PRICE`.
  - **`try_bud` is neither lane's**, so a breeding lane owns `try_bud`, the
    hatch arm, `brood_tick` and the brood verbs.
  - **B5 and B6 change founding** and go through the nest lane. The founding
    path also reaches the held world (druid coordinator note).
  - **Before each step, run `bash scripts/branchcheck.sh --who-touched
    src/sim/creature.rs`.** `claude/ant-nest-mouth-4f6s79` holds +439 lines
    there today.
  - **Land each step quickly.** `creature.rs` and `world.rs` are two of the
    most-landed files in the repo.
- **As lanes.**
  - B0d and B0e are measurement only (`examples/`, run scripts) and can run
    beside B1's build.
  - The render work (brood palette, pale callows, breeder mark) is a separate
    lane on `render.rs` and `assets/materials/`.
  - B3 onward is sequential.
  - **Invoke the `lab-coordinator` skill before spawning anything.** Pass
    `model:` explicitly, never inherited.

---

## 9. Risks, ranked

1. **The lab's evolutionary clock.** Breeder-centred breeding and a
   development delay both slow it. Mitigations: dispersal (B6), per-game
   defaults (§7), and `gen`/`bgen` reported at every step so the price is
   always a number in front of the owner.
2. **The economy re-derivation** (D11.3).
   - A correct egg or brood at inherited constants reads as *"eggs killed the
     colony"*.
   - The stamp is deferred, not removed (L1770).
   - A floor is not a margin (L1798).
   - Budget the sweep inside B1 and B3, never after.
3. **The census.** Eggs as organisms of the ant species reach 222 + 36 + 57
   call sites (B1's second trap). The guard, *adding brood changes no adult
   census*, is the defence, and the records fallback is the escape.
4. **The four verbs that see an organism-owned `Powder` as something else**:
   the mouth, the jaw, the founding cut, and footing (B1's first trap). The
   pip was dug as spoil once already.
5. **Genome appends** (`Lay`, `Leave`) shift every birth draw, void baselines
   and re-derive `mutation_rate`. Take them only where a heritable *where* or
   *when* is the point.
6. **Memory.** Every egg carries a 12,416-float genome, about 50 KB. 500 brood
   is ~25 MB per world, multiplied by the lab rack's copies.
   - Measure resident memory in B1.
   - If it binds, keep the mother's genome once, shared, and draw the child's
     at hatching from the same `RNG_SLOT_BIRTH` key: identical heredity,
     deferred.
7. **Frame cost in B6.** More colonies means more ants. Brood itself costs one
   dispatch per stage, not per frame.
8. **Identity.** Old age must read `hatched_frame`, but the lab roster pins
   `(handle, born_frame)`, so `born_frame` must never be rewritten.
9. **Concurrent lanes in `creature.rs`** (§8).

---

## 10. Questions for the owner

These need no picture, so they are for the chat. That is the owner's standing
instruction of 2026-09-10.

1. **The lab clock.** Should the evolution lab run the full life cycle and
   accept fewer generations per session, measured at B0, B1 and B2? Or keep
   budding there while the outdoor game and the held world get the life
   cycle? Or run it with short brood timings?
2. **A breeder you can spot.** OK to mark any animal that has bred, as a
   render readout that the engine never reads?
3. **Breeders that outlive workers.** Wanted, priced and heritable? Or one
   lifespan for all?
4. **Dispersal in the sealed lab.** It has no weather. A walking party
   (budding), or flights triggered by the colony's state?
5. **Asexual world.** Confirm the standing dead end stands: no mating, no
   males.
6. **`caste=4` by id.** Retire it once feeding-made castes supply the home
   ants, or keep it as a floor?

---

## 11. Sources

**In this repository** (read for this plan):

- `Reports/how-the-ant-works.md` — the living reference
- `evolution-lab-breeding-clock-2026-09-10.md`
- `evolution-lab-direction-2026-09-09.md`
- `colony-economy-design-2026-09-09.md` (§5c, §7)
- `creature-reproduction-economics.md`
- `creature-birth-grant-2026-08-30.md`
- `creature-gate0-births-2026-08-30.md`
- `creature-signature-and-castes-2026-09-06.md`
- `creature-direction.md` (§3b, §7b)
- `creature-evolution-plan.md`
- `creature-genome-flexibility-2026-09-02.md`
- `creature-locked-fields-2026-09-05.md`
- `evolution-lab-fission-design-2026-09-12.md`
- `evolution-lab-lifespan-rederived-2026-09-13.md`
- `evolution-lab-genetics-2026-08-31.md`
- `nest-biology-2026-09-19.md` (§10–11)
- `nest-colony-size-2026-09-28.md`
- `nest-granary-2026-09-28.md`
- `nest-one-entrance-2026-09-29.md`
- `ant-scenes-2026-09-23.md` (§9–10, §22n–p)
- `ant-sim-research-review-2026-09-19.md`
- `ant-sim-literature-review-external-2026-09-19.md`
- `population-dynamics-research.md`
- `colony-starvation-separated-2026-09-08.md`
- `colony-food-economy-design-2026-09-14.md`
- `two-games-one-repo-2026-08-30.md`
- the lane notes in `Reports/lanes/`
- `dead-ends.md` (L1158, L1169, L1185, L1194, L1195, L1204, L1770, L1798)
- `open-bugs-handoff.md` (§Z6, §Z12, §Z21, §Z36)

**External**, retrieved from PubMed for this plan:

- Motais de Narbonne M, et al. (2016). Biological activity of the enantiomers
  of 3-methylhentriacontane, a queen pheromone of the ant *Lasius niger*.
  *J Exp Biol* 219:1632–8. [doi:10.1242/jeb.136069](https://doi.org/10.1242/jeb.136069)
- Van Oystaeyen A, et al. (2014). Conserved class of queen pheromones stops
  social insect workers from reproducing. *Science* 343:287–90.
  [doi:10.1126/science.1244899](https://doi.org/10.1126/science.1244899)
- Brunner E, et al. (2011). Queen pheromones in *Temnothorax* ants: control or
  honest signal? *BMC Evol Biol* 11:55.
  [doi:10.1186/1471-2148-11-55](https://doi.org/10.1186/1471-2148-11-55)
- Pask GM, et al. (2017). Specialized odorant receptors in social insects that
  detect cuticular hydrocarbon cues and candidate pheromones. *Nat Commun*
  8:297. [doi:10.1038/s41467-017-00099-1](https://doi.org/10.1038/s41467-017-00099-1)
- Kurihara Y, et al. (2022). Thoracic crop formation is spatiotemporally
  coordinated with flight muscle histolysis during claustral colony foundation
  in *Lasius japonicus* queens. *Arthropod Struct Dev* 69:101169.
  [doi:10.1016/j.asd.2022.101169](https://doi.org/10.1016/j.asd.2022.101169)
- Wu X, Feng T (2026). Energetic thresholds and hygienic cannibalism govern
  claustral ant colony founding under fungal challenge. *J Theor Biol*
  633:112560. [doi:10.1016/j.jtbi.2026.112560](https://doi.org/10.1016/j.jtbi.2026.112560)
- Helms JA, Godfrey A (2016). Dispersal polymorphisms in invasive fire ants.
  *PLoS One* 11:e0153955. [doi:10.1371/journal.pone.0153955](https://doi.org/10.1371/journal.pone.0153955)
- Bono JM, Herbers JM (2003). Proximate and ultimate control of sex ratios in
  *Myrmica brevispinosa* colonies. *Proc R Soc B* 270:811–7.
  [doi:10.1098/rspb.2002.2287](https://doi.org/10.1098/rspb.2002.2287)
- Richard G, et al. (2021). Contribution of epigenetic mechanisms in the
  regulation of environmentally-induced polyphenism in insects. *Insects*
  12:649. [doi:10.3390/insects12070649](https://doi.org/10.3390/insects12070649)
- Jemielity S, et al. (2005). Long live the queen: studying aging in social
  insects. *Age* 27:241–8. [doi:10.1007/s11357-005-2916-z](https://doi.org/10.1007/s11357-005-2916-z)
- Richardson TO, et al. (2021). Ant behavioral maturation is mediated by a
  stochastic transition between two fundamental states. *Curr Biol*
  31:2253–2260. [doi:10.1016/j.cub.2020.05.038](https://doi.org/10.1016/j.cub.2020.05.038)
- Korczyńska J, et al. (2014). The effects of age and past and present
  behavioral specialization on behavior of workers of the red wood ant
  *Formica polyctena*. *Behav Processes* 107:29–41.
  [doi:10.1016/j.beproc.2014.07.009](https://doi.org/10.1016/j.beproc.2014.07.009)
- Trigos-Peral G, et al. (2024). Urban abiotic stressors drive changes in the
  foraging activity and colony growth of the black garden ant *Lasius niger*.
  *Sci Total Environ* 915:170157.
  [doi:10.1016/j.scitotenv.2024.170157](https://doi.org/10.1016/j.scitotenv.2024.170157)
