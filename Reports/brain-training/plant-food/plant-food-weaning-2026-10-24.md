# Ants feeding on plants: the planted bed and the weaning bed (2026-10-23 to 2026-10-24)

The plant-food lane, a child of the Teacher lane. Owner, 2026-10-23 16:37Z: *"explore ants in a planted world to eat instead of an unlimited pile of food"*. On 17:11Z the owner chose the **weaning bed** from a decision card: a planted bed plus a pile that shrinks round by round, delivery paid by energy, and seedlings pulled near the door.

**Where it stands (2026-10-24 15:00Z):**
- **Without breeding**, colony selection on the weaning bed made colonies that live on plants plus a 10-cell pile (plantwean3).
  - Paired against the starting ant in the same 64 worlds: median alive 21 of 24 against 0, nest 184.5 against 33.
  - Selected colonies did better in 53 of 64 pairs.
- **No colony lives on plants alone yet.** At no pile, all 64 of those colonies die. They eat plants in the field but carry almost none home.
- **With breeding** (the standard line since 2026-10-24), the first weaning run (plantwean4) never weaned past a 20-cell pile. It also made colonies worse than the untrained standard pool: nest 310 to 116.
  - The pile rule was fixed for breeding beds (`pilegate=`, PR #855).
  - plantwean5 is queued with it on the Teacher lane's VM.

Everything here is a switch or a scratch tool; nothing changes the game. Grazing (`PP_MIND_GRAZE`) has been on by default since 2026-10-24 (owner: "Yes to grazing"; `PP_MIND_GRAZE=off` restores loose food only).

**Defaults since 2026-10-24 evening** (owner, 17:35Z: "The default should be on unless there is uncertainty"):
- With `plants=`, `weed=30` and `fjoule=1` are on (`weed=0` / `fjoule=0` opt out). Every planted batch here ran both, and without weeding digging falls 246 to 46 cuts. Neither does anything on the pile bed.
- `colsel.py pilecap=` on a breeding bed steps on `pilegate=extinct:10:25` unless told otherwise (`pilegate=off`, or the args carry `brood=off`, for the starved rule). The starved rule is the one measured broken there (plantwean4).
- Still off, for a stated reason: `plants=` itself is a choice of bed that costs about 2.3x per frame and whose one breeding run (plantwean4) lost two thirds of the nest; `cap_to=` is a dead end (§3).

## 1. The mind could not eat a living plant

The mind's Pick up was offered only on food no organism owns (`src/sim/creature/options.rs`: `organism_id() == 0`). So a trained ant walked past every leaf, flower, fruit on the stem and fallen seed. On a planted bed it took about 5 bites, and all 24 ants starved by frame 2,600 (8 of 8 seeds). The old hand-built ant (`nomind=1`) on the same bed took 712 bites and all 24 were alive at 20k.

`PP_MIND_GRAZE=1` (`mind.rs` `scratch_log::grazeable`) lets Pick up take food a plant owns, through the game ant's own bite: `seed_survives_bite`, the passenger, `Eaten`, `eaten_plant_cells`. **Check first whether a task needs an action the ant does not have** (`parallel-training-guide.md` question 2 cites this case).

## 2. The planted bed (PR #836)

`examples/inrun.rs`, all off unless set, byte-identical without them (the two weaning levers below are on with `plants=` since 2026-10-24):
- `plants=MIX[:SPACING[:BAND]] grow=F rain=R` (`challenges::inrun::Garden`, `colony_in`) grows shipped species for F frames with the mister on, then lands the colony. It writes `garden.csv`, and `nestmap.txt` marks plant cells `%`; `nestcount.py`, `roomshape.py` and `inherit_nest` treat `%` as ground, because garden roots had read as up to 1,319 "nest" cells.
- Weaning levers: `fjoule=1` (pay by the food's energy, so a leaf pays 1/24 of a provisions cell), `weed=B` (pull plant cells within B columns of the door), `cap_to=C:F`, `film_hw=`.

**Cost:** about 2.3x the pile bed per frame (45-57 s against 18-24 s per 200k frames on 4 cores).

**Weeding is needed for digging.** Within 20k frames the stand seeds itself over the door, and its roots fill the soil. A jaw cannot cut plant tissue, so cuts fell 246 to 46 (to 102 with a falling pile). `weed=30` brought them back to 230. The alternatives were a twice-as-wide bed or letting ants cut roots, which is a physics change for every world.

Picture: [garden-vs-pile-s3.png](pictures/garden-vs-pile-s3.png) (no weeding: roots under the door). [weaning-bed-s3.png](pictures/weaning-bed-s3.png) (weeded).

## 3. One round cannot wean today's ant

hcs founder x30, selection off, 4 seeds, 200k frames, medians. Raw: [data/batch-weaning-2026-10-23.txt](data/batch-weaning-2026-10-23.txt).

| pile cap (garden + graze + weed=30 + fjoule=1) | alive (of 24) | bites | loads home | cuts |
|---|---|---|---|---|
| no plants, cap 60 (8 seeds) | 22.5 | 2,674 | 286 | 246 |
| 30 | 18 | 2,600 | 219 | 253 |
| 15 | 1.5 | 1,626 | 134 | 193 |
| 0 (plants only) | 0 | 441 | 18 | 72 |
| 60 falling to 0 by 150k (`cap_to`) | 1 | 2,125 | 250 | 230 |

The colonies die once the pile is gone, so weaning has to happen **across rounds, in colony selection**: `colsel.py pilecap=`. Breeding (`brood=lay`) on this bed boomed to 57-91 ants, then crashed on 4 of 4 seeds.

**After #841 (roots required) and #842 (plant old age), the garden got harsher.** Same seeds 1-4, paired: at cap 30, alive went from 22, 17, 19, 14 to 2, 2, 22, 0. The pile bed was byte-identical across the change. Re-baseline any planted bed after a plant change.

## 4. Colony selection on the weaning bed

All runs used colsel `lex+spoil+crater+shape`, 200k-frame rounds and a common garden (CG) every 2 rounds. In the CG each colony's carry is judged with selection off and the old-ant rules off, at the round's pile cap. Data in [data/](data/).

### plantwean1: stepping on alive fails

The pile stepped down on median end-alive. In-run births hold end-alive at 24 whatever happens, so the cap fell 30 to 0 by round 6, while starved per round ran 47, 9, 3, 6.5, 52, 195. All colonies were extinct by round 23. **But selection worked while the cap was liveable:** at cap 15 after 3 rounds, the CG held 19 of 24 alive, against 1.5 for the founder. The fix: step on median starved, with a floor.

### plantwean3: no breeding, the line that worked

hcs, 64 colonies x 32 rounds, `pilecap=45:5:10:10`, body-rule scaffold weaned over rounds 8-32 (Teacher lane's VM). Before the VM run, a local 8 x 8 run took the cap 45 to 20; its CG at 20 held a median of 20 alive against 4.5 for the founder ([cg-pw3mini-c8.png](pictures/cg-pw3mini-c8.png)).

The founder control is hcs x30 with the CG's own args, in the same 64 worlds (seed `c*1000+99`) at cap 10: median alive 0, starved 24, nest 33, crater 58.

| CG after round | cap | selected alive | pairs better / worse | nest | crater |
|---|---|---|---|---|---|
| 9 | 10 | 1 | 35 / 5 | 24 | 53 |
| 17 | 10 | 5 | 47 / 3 | 54 | 23 |
| 19 | 10 | 19 | 47 / 3 | 100.5 | 30.5 |
| 25 | 10 | 21 | 56 / 1 | 160 | 11 |
| 31 | 10 | 21 | 53 / 2 | 184.5 | 11 |

The crash at rounds 9-15 came when the cap halved from 20 to 10; the colonies took about 8 rounds to adapt. Digging came back with survival, and the crater shrank. Chart: [plantwean3-curve.png](pictures/plantwean3-curve.png).

**Less pile than 10**, probed on the round-31 carry in the same worlds. As a positive control, the local rerun at cap 10 reproduces the VM's CG exactly on colonies 1-8.

| cap | median alive | colonies with 12+ alive | dead colonies | nest | loads home |
|---|---|---|---|---|---|
| 10 | 21 | 45 | 10 | 184.5 | 3,710 (c1-8) |
| 5 | 4 | 20 | 19 | 105 | 1,575 |
| 0 | 0 | 0 | 60 | 32 | 196 |

**At no pile the ants still bite plants (about 2,000 bites), but carry almost nothing home.** The larder is fed by the pile. Plants-only is blocked on *carrying plant food home*, not on eating it.

### plantwean4: breeding on, and the pile rule breaks

std_pool, the standard line (breeding on), lesson c1:0.1, `pilecap=45:5:10:5`, 64 x 32.

- **It never weaned.** The cap went 45 to 25 by round 8, then bounced 25 ↔ 20 for 23 rounds. At 25, median starved was 6-17 (step down). At 20 it was 43-65, more than 3 x 10 (step straight back up). With breeding, a colony grows to what its food feeds, so starvation measures the cut to the pile, not the foraging.
- **The CG skips colonies that died out** (they have no carry), so its medians are survivors only. Count them as 0: the CG at cap 20 went 62 (round 9) to 65.5 (round 31), no progress.
- **The selected colonies were worse than the untrained pool.** Paired, CG worlds 1-16, local main 7dd6b0d4; the local rerun matches the VM on colonies 1, 2 and 4:
  - std_pool at cap 20: median alive 80.5, nest 309.5.
  - plantwean4's round 31 (dead colonies as 0): alive 71.5, nest 116, worse in 10 of 16 pairs.
  - Selecting at the edge of what the pile could feed traded digging away.
- **Where the untrained pool fails** (CG worlds 1-16):

| cap | median alive | dead of 16 | nest |
|---|---|---|---|
| 20 | 80.5 | 1 | 309.5 |
| 10 | 0 | 10 | 66.5 |
| 5 | 0 | 14 | 21 |
| 0 | 0 | 16 | 34.5 |

The cliff is between 20 and 10, and plantwean4 never trained there.

**The fix: `pilegate=extinct:LOW:HIGH`** (PR #855). The cap steps on the share of colonies that died out in the round. Down when at most LOW%, up when more than HIGH%. In plantwean4 that share was 2-9% at cap 25 and 11-25% at cap 20, so `extinct:10:25` would have held the cap at 20 from round 9. Job `plantwean5.txt`: plantwean4's line with `pilecap=30:5:10:5 pilegate=extinct:10:25`.

## 5. How to judge a weaning run

- **Pair against the founder in the same CG worlds at the same cap** (`tools/cgctl3.py`, `tools/cg4.py`). A CG median alone has twice misled here: plantwean4's "68-71 alive" was worse than the untrained pool.
- **Count colonies that died out as 0 alive.** The CG drops them.
- **Judge nest and crater as well as alive.** Food is not the only goal (owner). plantwean4 bought nothing in survival and lost two thirds of its nest.
- **Reproduce one VM CG row locally before trusting a local control** (`tools/cgprobe.py`). It was exact every time here.
- **Step the pile on a signal the bed does not hold fixed.** End-alive is held by births; starvation is set by the pile cut on a breeding bed. Colonies dying out moved with the cap in both kinds of bed.

## 6. Next

- plantwean5 (queued after farfood2). Judge it against std_pool in the same worlds at the cap it reaches, dead colonies as 0, nest as well as alive.
- If it stalls at the cliff again, the lever is *carrying plant food home*: a CG case for plant loads home, or plants nearer the door. `events.csv`'s deliver rows carry no food kind (`v` is 0), so that case needs inrun to log one first.
- Breeding on the planted bed (broodplant1c, Teacher lane): all 64 colonies alive, nests still food-packed.

The lane's dated working notes, as written during the runs (some numbers there were later corrected here, e.g. plantwean4's CG medians counted survivors only): [notes/findings-2026-10-23.md](notes/findings-2026-10-23.md), [notes/findings-2026-10-24-plantwean3.md](notes/findings-2026-10-24-plantwean3.md).

Tools: [tools/](tools/) (common-garden controls and the local runner). Data: [data/](data/) (colsel and CG csvs, job lines, `controls.csv` with every paired control run per world).
