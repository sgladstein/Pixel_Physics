# The brood pile east of the door: where it starts (trace, 2026-10-07)

Way home lane. Asked by the coordinator after the 12-seed read ([`build6-read-2026-10-07.md`](build6-read-2026-10-07.md)).

**Three runs have the die-offs:** seeds 3 and 7 with WAY_HOME on, and seed 6 with it off. Each grew a brood pile 8-24 columns east of the door. **Laying: please check this against your heap-laying chain.**

**Inputs:**
- each run's `broodlog.csv`: every egg laid, with its layer's zone and energy, and every move;
- maps every 1k frames;
- `hungry.csv.gz`, every decision of every hungry ant;
- the every-ant rerun of on seed 3 to 76k.

Base: heap 90, NEEDS_FIRST=on CARRY_HOME=on, build 6 (304d2e80).

## Answer

**It starts with a second opening dug below ground 10-24 columns east of the door. That happens in both arms, before the walk home does anything there.**

1. **The opening appears suddenly.** Open cells below ground at x+8-24 (rows 162-175):
   - on seed 3: 38 -> 67 at 52-54k;
   - on seed 7: 65 -> 90 at 98-100k;
   - off seed 6: 117 -> 153 at 208-212k.

   No other run gains one. The off seed 3 baseline stays at 47-52 throughout. On the maps it is a second shaft just east of the door, opening from the mound's tunnels at ground level (seed 7 at 110k: brood filling a column at x+10-14 down to row 182).
2. **Food is put down there.** On seed 3, every-ant record:
   - deliveries at x+10-19 went from 0 before 52k, to 126 at 52-54k, to 664 at 54-56k;
   - deliveries at the door went 166 -> 245 over the same frames.

   It is the first cell of home a carrier coming back from the heap reaches. **Corrected by Laying's review:** a delivery reads the dug home (`NestHome::Dug`: open cells at or below the old ground line, joined to the door through open cells, rebuilt every 256 frames), not cover. The opening becomes a place to deliver once the drain joins it to the nest below ground.
3. **Eggs are laid there, by ants fed there.** Eggs laid at x+8-24, 20-300k, by layers in the mound's tunnels / in the nest:

   | | in the mound's tunnels | in the nest |
   |---|---|---|
   | on seed 3 | 3,415 | 1,776 |
   | on seed 7 | 4,853 | 2,448 |
   | off seed 6 | 1,709 | 772 |
   | off seed 10 | 50 | 31 |
   | every other run | 0 | 0-10 |

   - The layers' median energy was 344-702 J, so 1.7-3.5x the grant.
   - About 80% of all eggs are laid by layers in the mound's tunnels on every run, so laying in the mound is normal. What is new is laying *beside the new store*.
4. **The larvae there eat the store**, and then the boom-then-famine chain follows (seed 3: brood ate 30-60k J per 4k frames, against 1-6k off, then the colony grew 353 -> 602 and grown ants starved by the main door).

## The anchor lead: a consequence, not a cause

Hungry ants anchored at x+8-24, of all hungry ants, per 2k frames:

| | before the opening | at and after it |
|---|---|---|
| on seed 3 | 2-7 (40-50k) | 21 -> 106 -> 101 -> 93 (52-58k) |
| on seed 7 | 0-3 (86-96k) | 43 -> 82 -> 109 -> 116 (98-104k) |

- **Walks home toward those anchors were 0** before and during the opening.
- **On seed 7 they started at 104k (209 decisions),** after it.
- Ants anchor there because they touch the nest there. The walk home then walks some of them back to it, but it did not start it.

## Is it WAY_HOME or the stack?

- **The stack.** The same opening, store, laying and die-off happen on off seed 6.
- **Frequency:** 2 of 12 seeds with WAY_HOME on, against 1 of 12 off, plus a trace of it on off seed 10. That is too few to say whether the switch makes it more frequent.
- **Inferred, not traced:** a bigger, better-fed colony digs more, so a second opening may come sooner.

## Not traced

- ~~Why the diggers open a second shaft there.~~ Traced below (06:30): it is not dug as a shaft. It drains.
- **Whether the delivery really reads the mound's cover there.** That is inferred from where the deliveries land.
- **Why grown ants by the main door starve while the east store holds food.** The two parts of the nest may be poorly connected. That is not traced.

## Tools

The numbers come from short inline scripts over `broodlog.csv`, the maps and `hungry.csv.gz`. They are summarised above, and the same counts can be re-run from `tools/latchsplit.py` and `tools/doorcrowd.py` for the walk-home side.

## Who opens it, and how (traced 06:30, for Nest building to review)

It is **not dug as a shaft**. On seeds 3 and 7, a single cut through a room's packed wall lets the loose soil behind it drain into the room. The drain runs up through the ground into the spoil mound's tunnels above. Inputs: the cell-change log (`cells.csv.gz`), `cuts.csv` and the per-dig rows (`digrows.csv.gz`). Both reruns match their originals frame for frame (seed 3 to 75k, seed 7 to 100k).

**On seed 3, at 53,110:**
- **Who.** Ant 3145884 cut packed soil at (268,169). It was a nest worker, 324 frames old.
- **Where it was.** In the nest room east of the shaft, 10 rows under the founding ground, under its grant (0.85).
- **What it was doing.** It had just stepped on `HUNGRY_OUT`'s pull towards (264-265,168-169). The dig was the ordinary in-nest roll, at `dig_p` 0.805.
- **The drain.** Three frames later the loose soil above the cut began to fall: (268,168), then (269,166), (269,165), (267,164), on up to ground level. That made 559 soil-to-empty moves below ground in 53.1-54k, with no cause recorded, so it was gravity. It opened ground level at x 264-271 into the mound's tunnels.
- **What followed in the next 500 frames:**
  - Carriers began putting food down at (264-277,159-160): 27 deliveries, then 95, against 0 in every 500 frames before.
  - Very fed layers there (1,400-1,700 J, 7-8x the grant) laid eggs at (265-267,159-163) from 53,249. The eggs fell down the new hole into the room's east end, which is the eastern brood pile.

**On seed 7, at 98,870:**
- **Who.** Ant 6291840 cut packed soil at (267,167). It was a nest worker, age 8,719, at 0.95 of the grant.
- **What it was doing.** It had been walked "back to the face" (`FACE_TRIP`) and its dig cue fired (`dig_p` 0.802).
- **The drain.** Three frames later (268,165) fell, then (268,164), (269,164), (268,163) and on: 589 loose-soil falls in the 250 frames from 98,750.

**On off seed 6 (rerun to 212k): the same place, a gentler start.**
- No single big drain. Many diggers cut east at x 263-274 from 208k (10-16 cuts per 500 frames).
- Repeated small drains followed: 26-65 falls per 100 frames at 210.1k, 210.5k and 211.7k.
- The opening grew 117 -> 153 over 208-212k.
- Then the same shape, smaller: brood ate 14-29k J per 4k at 216-228k (against 0-9k before), the colony went 265 -> 346, and 42 starved at 232k.

**What is measured:**
- A cut through a packed wall with loose soil behind it, under the mound east of the door, opens the nest into the spoil mound's tunnels.
- Food and laying move to that new opening within 500 frames.

**What is not:**
- How much of the ground east of the door is loose soil behind packed walls, and whether that is generated ground or old spoil.
- Why only 3 runs of 24 reach it.
- Whether a second entrance with a store near the food is a fault or the start of what Scott wants (separate chambers, food stored). **What went wrong after it** (traced on seed 3, above): the larvae beside the store ate it, and the colony boomed and then starved by the main door.

**Nest building: please review.** Is the loose-soil drain a known mechanism? Is this opening something you would want to tame or prevent? Please write to `trace-east-brood-pile-nestbuilding-review.md` here.

## Laying's review (06:10), and the laying bar on these runs

Laying's review is [`trace-east-brood-pile-review.md`](trace-east-brood-pile-review.md).

- **Same family as the heap-laying chain.** Laying's heap-30 runs, which have no WAY_HOME, show the same pile and the same die-offs. At heap 30 the diggings reach the heap's underside; at heap 90 the carriers build the store at the first cell of home they touch.
- **Correction:** delivery reads the dug home, not cover. This is folded in above.
- **An east pile alone is not the killer.** One of Laying's piles had rich layers that needed food in reach for only 9% of their eggs, and it faded with no die-off. **The killer is a pile on food, laid by ants that cleared the bar only on that food.**

**The laying bar (`LAY_BAR=body`) on these three runs**, from Laying's `review-laying/eastopen.py`. The figure is the share of east eggs whose layer needed food in reach to clear the 946 J bar, which are the eggs the bar would block:

| run | first 4k after the opening | to the die-off | total |
|---|---|---|---|
| on seed 3 (50-72k) | 6 of 69 | 45-66% | 172 of 427 (40%) |
| on seed 7 (96-122k) | 17 of 60 | 47-78% | 267 of 438 (61%) |
| off seed 6 (206-232k) | 17 of 33 | 0-74% | 122 of 225 (54%) |

- **The bar blocks about half of the east eggs. It does not block the first ones.**
- In the 4k right after the opening, the layers were very fed (median 1,446-1,463 J after laying on seeds 3 and 7) and needed no store to clear the bar.
- Whether blocking the later half is enough to prevent the boom is Nest race's `LAY_BAR` test.
- Laying suggests that test add off seed 6 at heap 90, with and without the bar, paired. I've passed that to the coordinator.

## Nest building's review (06:00), and the two checks it asked for

The review is [`trace-east-brood-pile-nestbuilding-review.md`](trace-east-brood-pile-nestbuilding-review.md), a code read of build 6.

**What it says:**
- **The drain is a known mechanism, and by design.** Every cut packs its 8 neighbours into wall, except spoil (`SPOIL_PACKS` off) and the founding shaft's own columns.
- **Tame it, don't prevent it.** The harm is downstream: laying on food (Laying), food not moving inward (the jobs spec), and hungry ants pulled to one door.

**Check 1: what drained.** From the cell-change log, before the cut at 53,110 on seed 3:
- **The first cell to fall, (268,168), was spoil.** A pellet had been put down there at 47,935, in the room's wall right above the cut.
- **The 21 other cells that drained in the next 15 frames were native loose soil.** They were last logged before 6k, that is, never touched since the colony was founded.

So Nest building's "buried spoil" is the plug, and the body behind it is the generated ground: loose soil reaching up to the founding ground, held by a packed skin and one spoil pellet. *Inferred:* the ground east of the door is loose wherever no cut has packed it, so a hole in the skin drains up to the surface.

**Correction:** "559 falls" counts fall steps, not cells. The open cells below ground at x 262-277 went 59 -> 106 at 53-54k (+47).

**Check 2: did the fallen soil cut the room in two? No.**
- A fill through open cells below ground (brood, food and corpses count as open, as in the dug home) from the founding shaft reaches every open cell at x 262-277.
- That held in every map at 52-72k: 58 of 58 at 52k, 106 of 106 at 54k, 161 of 162 at 70-72k.

So the grown ants starving by the main door were not cut off from the east store. That leaves Laying's lead (the larvae draw 37-51% of their food from nestmates' bodies) and Nest building's (`HUNGRY_OUT` maps the founding door only). Both are inferred and not traced here.

## The plug: NEEDS_FIRST's `pack_behind` (traced 06:15, from Nest building's code lead)

The spoil plug and the unlined column above it were left by an ant using `pack_behind`, five thousand frames before the drain.

**Seed 3, from the every-ant rerun's dig rows and cell log:**
- **Who.** Ant 1048804, a soil holder at 0.91 of the grant, with spoil kept. It had been on "soil way out" and "hungry out".
- **47,920:** it cut (268,168) from the room.
- **47,930-47,970:** it walked straight up through native loose soil, (268,167) to (268,159), one cell per step, `spoil_why` = `packed` on each. Every step put its pellet in the cell its tail left: spoil at (269,169) and (268,168) up to (268,161).
- **47,970:** it came out into the mound's tunnels at (268,159), then hauled spoil on the mound.
- **The rest of the frame.** No packed lining was laid around the column, consistent with `pack_behind` not calling `line_burrow` (Nest building's code read). The column was a stack of spoil in loose ground, joining the room's ceiling to the mound floor.
- **53,110:** ant 3145884's cut at (268,169) took the support from under the bottom pellet. The column and the loose ground beside it drained into the room, which opened the second entrance.

**Seed 7:**
- **At 82.8k,** the same signature: spoil set from tail cells at (269,167), (268,167) and (267,166) up to (267,162), a column from the room up through the ground.
- **At 98,870,** the cut at (267,167) drained it, 16k frames later.

**What this makes the first link.** A NEEDS_FIRST rule (`pack_behind`: an encased carrier cuts ahead, steps in and drops its pellet behind) leaves an unlined spoil column through native loose ground. A later ordinary cut under it drains it into a second entrance. NEEDS_FIRST is not on main; that is the stack these runs are on.

**Off seed 6 has the same column** (rerun cell log). At 146.1-146.2k, spoil was set from tail cells at (272,171) and (271,170) up to (271,161), a column from the room to the founding ground. Its drains came at 208-212k. **So it is 3 of 3 pile runs.**

**Not done:** any remedy. Nest building's addendum 2 (inferred from code): lining the column would stop only the ground beside it draining. The spoil column itself still slumps once its bottom is cut, so a plug that holds needs the backfill itself to pack. Both candidates belong to NEEDS_FIRST, and neither is proposed for building yet.
