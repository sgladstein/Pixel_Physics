# What the trail loop taught us about scents (survey, 2026-10-06)

Collected for the needs-and-jobs design doc by a read-only survey of the repo's own reports and code on main 3ba1e7bd. Sources are cited per claim by the key below; INFERRED marks the surveyor's own arithmetic. Line numbers are as of main 3ba1e7bd.

I read these sources: the pheromone module and its constants, the trail-reading code in creature.rs, the status index plus §Z7, §Z29, §Z30, §Z32 and §R4 of the bug register, the matching dead-ends entries, ant-scenes §4–§12 and §22v–§23g, the forage-bed, decision-census, lifetime, master and quantisation reports, the food-trail plan and reader design, and four lane notes. Anything marked INFERRED is my own arithmetic or reading of the code, not a measurement in the repo.

**Source key** (absolute paths):
- [PH] /home/claude/Pixel_Physics/src/sim/pheromone.rs
- [CR] /home/claude/Pixel_Physics/src/sim/creature.rs
- [WD] /home/claude/Pixel_Physics/src/sim/world.rs
- [LS] /home/claude/Pixel_Physics/src/lab/scene.rs
- [ANT] /home/claude/Pixel_Physics/assets/species/ant.ron
- [AS] /home/claude/Pixel_Physics/Reports/ant-scenes-2026-09-23.md
- [OB] /home/claude/Pixel_Physics/Reports/open-bugs-handoff.md
- [DE] /home/claude/Pixel_Physics/Reports/dead-ends.md
- [FB] /home/claude/Pixel_Physics/Reports/ant-forage-bed-and-gates-2026-09-21.md
- [DC] /home/claude/Pixel_Physics/Reports/ant-decision-census-2026-09-22.md
- [PL] /home/claude/Pixel_Physics/Reports/pheromone-lifetime-and-wiring-2026-09-14.md
- [PM] /home/claude/Pixel_Physics/Reports/pheromone-master-2026-09-17.md
- [PD] /home/claude/Pixel_Physics/Reports/pheromone-trail-direction-2026-09-16.md
- [DQ] /home/claude/Pixel_Physics/Reports/decaying-gradient-quantization-2026-09-15.md
- [RD] /home/claude/Pixel_Physics/Reports/food-trail-reader-design-2026-09-30.md
- [FP] /home/claude/Pixel_Physics/Reports/food-trail-plan-2026-09-29.md
- [HW] /home/claude/Pixel_Physics/Reports/how-the-ant-works.md
- [MP] /home/claude/Pixel_Physics/Reports/ant-movement-plan-2026-09-22.md
- [RM] /home/claude/Pixel_Physics/Reports/README.md
- [IN] /home/claude/Pixel_Physics/Reports/instruments.md
- [DT] /home/claude/Pixel_Physics/Reports/lanes/druid-trail.md
- [RT] /home/claude/Pixel_Physics/Reports/lanes/evolution-lab-round-trip.md

### Failure modes, most important first

1. **The trail worked as a brake, not a steering wheel.**
   - Why: the along-reading fed only hidden units 0–3, which drive `Move` (whether to step), never which way. `Turn` does almost nothing on level ground (§R4). So a bad reading stopped the ant instead of turning it.
   - Measured, gap 90: the chance of stepping was exactly 0 on 69.5% of empty-ant and 83.2% of laden-ant decisions. For a frozen empty ant, trail B added −3.130 to the stepping sum (+0.103 when moving). On 78.7% of 721,196 stalled decisions, every heading the ant tried read downhill [DC §4–§5].
   - Laden ants moved on only 1.37% of 743,889 laden ticks, and the trail term (−2.12) was the entire deficit [FB §8b, §8f, §8h].
   - Fix: the `TrailAway` chooser, default since 2026-09-24, feeds the brain 0 for both trail readings in nesting species and reads trail presence where a step would land [CR 19412–19430, 19870–19891]. Founders' round trips went from 13.5 / 1 / 0 to 14 / 14 / 11 at 90 / 140 / 200 cells [AS §11].

2. **The colony's own trail B became a hotspot that trapped ants.**
   - Why: laden ants, the only ones laying B, stalled in one band. B was brightest there (72% of the time lit, mean 708, at x 68–77) and nearly dark at the nest (7.8% lit). The whole colony made this trap, so no fix applied to one ant could work.
   - Measured: silencing the ants' own B laying took second laps from 87 to 182 (better on 19 of 24 seeds) [FB §7a]. After their last delivery, ants parked at a median x of 72 [FB §7c].
   - Laying at the vacated cell and aiming home at the nest centre were both negative (12/12 and 10/14 seeds) [FB §8g].
   - Silencing the own trail also raised food taken 6,062 → 7,512 (21/3 seeds), births 188 → 282, and cut starvation 57 → 29 [AS §22v].
   - Fix: item 1 plus the trip-load lay rule (item 16). After both, the own trail helped for the first time: +52% food against no rule [AS §23c].

3. **Trail B's slope points at the nest, not the food.**
   - Why: only laden ants walking home lay it, at a flat rate (EmitB 0.714). So the food end is always the older, fainter end. An ant climbing the gradient walks home; a reader that walks downhill cannot pick up a trail it is not already on.
   - Measured: the gradient read +0.04 to +0.23 toward home along the whole route [RM 3644–3658]. A downhill reader was on the trail 0.0% of ticks when started 24 cells off its end, against 13.4% for an uphill reader and 4.3% for no reader [PD §6.1].
   - The run-average profile was `[11736, 2525, 2420, 2398, 589]`, tallest at the nest [FP "What exists today"].
   - No lay rule fixed it. Each returner's line fades about 14% per cell behind it, and the live trail rose toward food at only 0.397 / 0.394 [AS §23a].
   - Trap: the test bed's hand-laid ramp rises toward the food, so a "follow rising scent" reader would have scored well on the bed and failed in the game [CR 19913–19919].
   - Fix: stop climbing B. Read presence plus an away-from-home term (item 4), and compare east against west only at the door (item 14).

4. **Trail presence has no direction (confirmed).**
   - Why: the chooser reads B as a saturating presence, `x/(1+x)`, so both directions along a route score the same.
   - Measured (S5 lattice): 21 of 24 ants got 60 cells east, but at a median of 1,189 decisions against 702, and they were on the trail row only 23.5% of the time [AS §6]. Adding `AWAY_GAIN × presence × cos(heading, away from home)` gave 24 of 24 at a median of 297 [AS §11; CR 19909–19932].
   - `TRAIL_GAIN` multiplies rather than adds: an added term would raise reversals from about 1 in 120 to about 1 in 4 [CR 19899–19907].
   - The away term has its own limits. It cannot tell a dead branch from a live one (item 12). It is undefined at the 5-column door, because touching the nest re-anchors `forage_anchor` [AS §22v].
   - Nothing reads trail height: the front-strength inputs were never wired [PH 138–151].
   - INFERRED: because `TRAIL_HALF` is a tenth of a deposit, one fresh laden pass already reads 0.877 presence (0.714 × 10,240 / 1,024 = 7.1). The chooser therefore cannot tell a busy trail from a light one.

5. **An ant's own deposit blinds its homing sensor (§Z29, still OPEN).**
   - Why: the mark lands at the head, so next tick `here` is the ant's own freshest deposit. `ahead` is in sky or rock about 70% of the time. The reading comes out negative whichever way the ant faces.
   - Measured: facing home gave a positive reading on only 1.2–8.2% of laden ticks [OB 13331–13346].
   - An 8-seed census of about 500k decisions: −0.2003 (5.3% positive) facing home, −0.2399 facing away [OB 13391–13408].
   - The average reading size looked healthy (0.23–0.53), so aggregate checks missed it. Making the plane 4x more readable (13.7% → 54.6%) gave 47 → 48 round trips [OB 13348–13363].
   - Fix: not a sensor repair. Laying at the vacated cell only moved 5.3% → 7.0% positive, still 93% wrong-signed.
   - What worked was a path-integration input, `(HomeAligned, Move, 3.0)`: completed laden returns rose 6.8% → 28.1%, better on 8 of 8 seeds [OB 13410–13427; RM 3168–3182].

6. **The nose sampled sky or rock.**
   - Why: `sensor_offset` 6 applied along both axes put the sample six rows off the ant's row for 6 of 8 headings. The formula `(0 − here)/(0 + here + SCALE)` then turned "no data" into a confident −0.909.
   - Measured: the off-row headings gave a usable reading on 0.2–2.3% of ticks against 10.9%. 89% of frozen laden ticks had the nose in sky or ground [CR 15585–15596; DE 1015].
   - On trail B (§Z32): on diagonal headings the nose read 757 while the cell under the ant held 7,670, on 46% of empty-ant ticks, with mean vertical offset exactly 6.00 [OB 13530–13545].
   - The original left/right sensors read exactly 0.000 over a cell holding A = 27 [DE 1075].
   - Fix: an "honesty" gate (a sample landing where nothing can stand reports no information) on 2026-09-19, then projection onto the ant's own row on 2026-09-20. Closed laps 144 → 376 (better on 23 of 24 seeds); reached food 53% → 72% [OB 13550–13566].
   - Caveat: on trail A for homing, projection worsened round trips, because six cells along the row is air over any dip. A surface-following sample is still unmeasured [CR 15496–15553; DE 2017].

7. **Reading 6 cells ahead past a dead end or into rock froze the ant.**
   - Why: the first chooser did not re-aim after a failed step, so one bad far reading meant a step chance of 0 forever.
   - Measured (fork scene S4): 21 of 24 runs froze for 100+ decisions (median 1,750) once the sample passed the branch's blind end (the reading dropped to −0.48 in one step).
   - A trail up the climbing branch froze every ant at the fork on both walks (readings −0.43 / −0.73; step chance exactly 0 even at full `Stillness`).
   - Lattice scene S5: 0 of 24 got through; longest stand-still 3,994 decisions [AS §4–§5].
   - Fix: stage 2 reads the cells 1 and 2 ahead where the step lands, and retires the throttle. No run froze (longest stand-still 17); 22 of 24 took the trailed level branch first and 18 of 24 the trailed climbing branch [AS §6; CR 19878–19883, 21398–21421].

8. **The brain's gate flattened the trail signal it was meant to pass (§Z7).**
   - Why: an additive gate parked the hidden units on the flat part of `squash` (0.960–0.973). The trail could then change the step chance by only about 0.0035 × the reading.
   - Measured: with and without a laid trail, ants near the target came out 1,903 = 1,903 and 595 = 595 exactly. Pooled: 7 with the trail against 336 without. Re-gated: 13,985 against 0 [OB 10589–10660].
   - It is a dated regression: commits e8314d16 and 656c5eea, 2026-08-31 [OB 10631–10643].
   - Fix: the homing pair was re-gated 2026-09-09. The food pair was left saturated on purpose, because re-gating it made colonies worse (item 13) [OB 10662–10706; DE 1954].
   - It is still saturated in the eight other ant-family species (about 0.003 on `Move`) [HW 1386–1395]. INFERRED: moot for nesting species since the throttle retired [CR 19451–19456, 19884–19891].

9. **8-bit storage flattened the gradient.**
   - Why: a scent that both decays and is read as a difference ran out of bits at its faint far end.
   - Measured:
     - At blend 0.1, spread died two cells out, and a seam test was comparing two columns of zeroes [PH 112–122; DE 1291].
     - The reading was exactly +0.000 at 0.7 and 0.9 of the way out while the plane peaked at 39–98 of 255 [PL §3c].
     - The trail stopped steering at frame 48 while still 77 cells long [PL §1c].
     - The forced minimum fade (which prevents permanent "ghost trails") capped a 40-unit deposit at 40 passes; it actually died in 12 passes, 144 frames against a 2,200-frame round trip [PL §1b; PH 96–109].
     - The trail network collapsed 342 → 35 cells when the colony fell 46 → 20 ants [PL §1e].
   - Fix: `Scent` became `u16` in 8.8 fixed point (2026-09-15, PR #450). Trail life 144 → 1,476 frames; steering range 36 → 1,080 frames; network 208–342 → 1,405–2,057 cells; the pass got cheaper (0.85–0.92x the time) [PL §3c; PH 46–66].
   - General rule recorded: exactly zero at several sample points is the signature, and per-cell decay safeguards cannot see a difference between two cells [DQ; DE 1294].

10. **Diffusion, not evaporation, erases the trail, and the blend trades tracking against memory.**
    - Why: on a one-cell line, the 3×3 blend removes 16.7% per pass against decay's 2.9%.
    - Measured: setting decay to 0 left a trail's life unchanged at 144 frames; setting the blend to 0 raised it to 432. Sweeping decay across the literature band moved it only 144 → 60 [PL §1b; PH 166–184, 1052–1073].
    - At a fork with 10:1 traffic, the weak branch stood at 19 / 2 / 0 for blend 0.10 / 0.25 / ≥0.50; the doc says 0.25 "sits close to the cliff" [PH 153–163].
    - The sweep that chose 0.25 re-laid the trail every pass, so it measured tracking, not forgetting [PH 124–136; IN 162].
    - A blend of 0.05 on trail A smeared its ramp [DE 1292]. More evaporation does not cure lock-in to the first path found (P-12) [DE 1073].
    - Workaround: a per-channel `set_channel_diffuse` dial, and a blend of 0.05 for B in the lab (item 12).

11. **Timing: the homing plane faded faster than an ant walks home.**
    - Measured:
      - At one pass every 4 frames, the minimum-fade ceiling was about 1,000 frames against a 2,200-frame round trip. Deliveries were 0, with total trail A at 100 [PH 80–95; DE 1289].
      - On `u16` at decay 0.03, the trail was gone at 0.67 of a trip and useless for steering at 0.49 [DE 1290].
      - A laden ant nets about 0.014 cells/tick, so 90 cells takes 2,000 passes; 0.995^2000 = 4.5e-5. Median trail A was 0 from 30 cells out. At the food, one ant read 0.0000 for 1,728 frames and moved 2 cells [PH 218–231; PD §7.42].
    - Fix: one pass every 12 frames, scaled with the creature clock [PH 1226–1236; WD 6991–6994]. `TRAIL_A_RHO = 0` (2026-09-19): readable route cells 13.6% → 53.2%; ants reaching food and getting home 0.88% → 14.84% (23 better / 6 worse / 7 tied) [PH 218–224].
    - Open: colony intake fell by a median third (15/21 seeds) [PH 247–253].

12. **Trail B's lifetime is wrong in both directions (confirmed, with a correction).**
    - Too short: at the engine's settings (fade 0.03, blend 0.25) B never builds beside an endless pile. It sits at 2–30 of 255 behind the laden ants and near 0 between waves; one pass lays about 29 and is gone in about 144 frames [PH 904–913; IN 143].
    - Too long: fade 0.005 with blend 0.05 on the goal box (12 seeds, 200k frames) cut food 104,340 → 82,820 (lower on 8/12) and mean ants 4,025 → 2,756 (lower on 10/12).
      - Old side paths stayed lit: B read 5–17 across the west half, away from the only pile, against 0.
      - Mid-run foragers spent 62–74% of trip time heading the wrong way, against 45%.
      - Four seeds alone split 2–2 and read as no harm [PH 917–931; DE 1130].
    - Fade 0 floods the corridor at 70–160 with no shape [PH 912–913]. Fade 0 on B gives a third as many ants reaching food [PH 255–258].
    - Faster fade helps on the played box (54,722 / 94,501 / 127,339 J at 0.03 / 0.10 / 0.25) but does nothing on the single-pile bed [PH 260–270].
    - Fix: a lab-only pair, fade 0.03 and blend 0.05: food 104,340 → 136,761 (higher on 9/12) [PH 933–982; LS 1044–1049].
    - Correction: the losing arm changed fade and blend together. The clean fade comparison is 82,820 (fade 0.005, blend 0.05) against 136,761 (fade 0.03, blend 0.05).

13. **Stale trails recruit to food that is gone.**
    - Measured: letting empty ants actually read B gave 25.0% in the mirrored race (0 of 6 seeds), against a 41.2% control.
      - Unvisited larder rose 57% → 74%. The 16–48-cell band was eaten out (284 → 49) while the band past 128 cells went untouched (1,285 → 1,514) [OB 10674–10684; DE 1954].
      - One seed at faster B fade (0.15) did not rescue it [OB 10724–10729].
    - The door reader kept saying "food this side" for 800–1,400 frames after a pile ran out [CR 21707–21711].
    - Fix: the door read is gated on the colony's news (frames since a forager last came home), with the window cut to 700 frames. It passed all bars on 48 seeds and shipped as default 2026-10-01 (#535) [AS §23e; CR 20517, 21736–21740].

14. **Presence saturates at the door, so the door gave neither news nor a direction.**
    - Measured: B at an ant's departure read 0.99–1.00 regardless of how long ago food last came home (median 432 frames between laden returns, against a 273-frame half-life).
    - It read 0.877 for ants about to go east and 0.893 for ants about to go west. East share was 0.667 with the own trail, 0.704 with none, 0.728 with a painted road. West-goers reaching the pile: 0/53, 0/50, 0/40 [AS §22v].
    - The 1–2-cell ring around the door sits in its own delivery smear and reads saturated on both sides [CR 21700–21703; RD §1].
    - Fix: the trip-load lay rule made B six cells out track the last return (Spearman −0.53 to −0.82) [AS §23a P0.10]. A door-only reader, `g = (bE − bW)/(bE + bW + TRAIL_HALF)` at gain 6, cut west departures 36% → 6% (90 cells) and 34% → 16% (140), and raised food taken 10,546 → 17,532 [AS §23e; CR 21712–21752].

15. **Nest scent saturates over the spoil mound and locks headings (confirmed).**
    - Measured on the dry goal box (laying lane's evolved founder, seeds 1–4, 300k frames, 2026-10-05) [CR 19688–19712]:
      - Trail A presence was 0.98–0.99 on every heading over the mound, so the hold (1 + 3 × presence) multiplied every heading about 4x. Going straight scored 3.9 against a pull home of at most 1, faded to 0.37.
      - 84–89% of carries lost the pull home at least once, and 51–67% of all food trail was laid while lost.
      - In the last 22 steps before giving up, a step toward home was open 83–86% of the time and the ant stepped away on 60–66% of steps.
      - Carriers stayed a median 98–107 decisions at the heap per carry.
    - The nest-contact sense never fires on the mound, which holds 39% of door-zone decisions [RD §1].
    - Tries:
      - Counting presence above the weakest heading, for every laden ant: carries that lost the pull home fell from 45–58% to 9–26%, but colonies halved and seed 4 died; never shipped [DE 1260].
      - `CarryHome`: shipped off 2026-10-06; on seed 2 the colony fell from 650 ants to 150 between 200k and 300k frames [DE 1262; CR 19720–19733].
    - INFERRED: with weight 1, one plane's presence tops out at 65,535 / 1,024 = 64, or 0.985. So 0.98 implies trail A at ≥ ~50,000 raw (77% of the u16 range) over the mound, near the storage ceiling, unless the evolved weight is above 1.
    - The phrase "false roads" does not appear in the repo; the source calls it "the even fog over the mound".

16. **The wrong ants laid trail B.**
    - Why: any ant with food in its crop laid B, including ants carrying a packed lunch out and ants eating store food.
    - Measured: 13.9% of all B-laying steps were west of the door, where there is no food, and only 6% of those came from a trip load [AS §22v].
    - Fix: `FOOD_TRAIL=lay` (only a load from a trip lays B), shipped 2026-09-30 (#523). Food taken 6,062 → 9,217 (23/1 seeds), starved 57 → 14, born 188 → 416 [AS §23c].
    - Side effects:
      - Stray B had been keeping hungry home ants walking; home B coverage fell 88% → 31% and one seed's deaths rose 6 → 10 [AS §23c].
      - At 200 cells the lay rule alone raised starvation 113 → 181 (p 0.004) by sending under-fuelled ants out. The forage throttle then cut starvation 554 → 307 over 72 seeds [AS §23f–§23g].
    - The eight other species still lay B whenever carrying, including dig spoil [HW 1394–1395].

17. **Laden-only, flat-rate laying, only on successful moves: the colony could not build a connected road.**
    - Measured: a hand-laid trail was decisive (92% vs 3% of ants reached food; 4 of 6 vs 0 of 6 colonies survived). The colony's own laying was indistinguishable from none: about 3.4 route cells of 90. Ants added nothing to maintaining a laid trail [PM §3.2].
    - The ant-laid trail was 8% connected against 100% for the hand ramp; seed 4's "78% of peak" was a 43-cell gap in a 91-cell route [RM 3588–3593; FB §4b].
    - Laden ants intended to lay on 100% of ticks but moved on 1.37%, and a mark is only laid on a successful move [FB §8b; DE 1065].
    - 34% of ants finished digesting their load one cell short of the nest, which left the nest end dark [FB §7a].
    - Trail A's emission falls 78% over a 141-tick trip [PM §3.6]. A colony ranging out to 143 cells lays a home trail nobody is near [RT "Where to look next"].
    - Rejected: a food odometer driving EmitB; the colony did worse the more B it laid [DE 2011].

18. **An ant on a trail could not give up, and then the trail itself triggered give-ups.**
    - Why: the give-up pull home was scaled by (1 − presence), and the away-from-home term had no patience.
    - Measured: a given-up ant took 1,392 frames to get home against 504 with no trail, and still headed outward on 43% of steps against 11% [AS §22v].
    - The Stage 3 bound was set off by the door's own smear (presence 0.005, five cells out). 399 of 490 west give-ups counted as "lit", and the walk on dark ground fell 37 → 19 cells, which stopped ordinary scouting.
    - Letting go of ants that gave up on a lit trail turned them away from a live pile: 29 of 43 still reached it, against 12 of 30 [AS §23d].
    - At 200 cells the give-up took back about a third of the reader's gain [AS §23f].
    - Fix: the lay rule (1,392 → 510 frames) [AS §23c]; the bound now arms only at presence ≥ 0.5; and `noreturn` cut starvation 167 → 156 (p 0.031) [AS §23d, §23f].

19. **The planes ignore terrain (confirmed as stated, but never measured).**
    - The pass blends each cell toward its 3×3 mean with no material test [PH 762–792]. Scent spreads into rock and sky and leaks between parallel tunnels [HW 1108–1109, 263–265; MP 143].
    - I found no measured size for the leak.
    - INFERRED: the honesty gate only fires when both planes read exactly 0 at the sample [CR 8827–8830], so leaked scent inside rock counts as a real reading. The chooser's 2-cell sample is not checked for passability [CR 21414–21417].

20. **Comparing over time, or between two points ahead, failed.**
    - A hidden-unit "compare scent now against a moment ago" circuit was a level detector: the level term was 2.8x the signal, with up / down / still readings of +0.66 / +0.62 / +0.67 [DE 2019].
    - Wiring "scent rising under my feet" into `Move` for every ant tethered empty ants to the nest: reached food 199 → 156 (p 0.0072). The sign was right for laden ants and backwards for empty ones [DE 2025; CR 8799–8808].
    - A two-point forward comparison moved the reading (p 0.011) but not the laps (91 → 88). The trail the ants then laid went from +0.0617 to −0.0160 on the pair the ant reads (6 and 12 cells) [DE 2023; RM 3200–3216].
    - Rule recorded: any change to how a population reads a field it also writes is sized against a field it will then change.
    - The "scent rising" input now ships gated to carrying ants, but it is computed and wired to nothing [HW 265–267].

21. **Laying height (the player's trail in the held world, same plane and constants).**
    - The gnome's marks hung 3 cells above the floor, then after a first fix sat 4 cells buried (64 of 64 marks).
    - A lone mark lasted 3.5 s against a ~37 s round trip.
    - A full-strength one-cell line lasted 4.8 s; a 3-cell-wide line at the normal deposit lasted 10.8 s [DT "The headline", "What was wrong"; IN 166].
    - Fix: lay at +1 cell, and re-lay the remembered route every pass (30 s).

22. **The alarm scent was modelled as a conserved substance.**
    - Measured: a wound was audible at 4 / 0 / 0 at 1 / 2 / 4 cells [RM 3695–3709].
    - Fix: falloff by distance instead of diffusion gave 148 / 88 / 24, and alarm fade went 0.25 → 0.35 [PH 336–392].
    - The trail planes refuse that spread mode, because it would delete the way deposits add up [PH 1097–1102].

23. **Instruments that faked or hid scent results.**
    - The polarity metric scanned a fixed window and counted one shoulder of a diffusing blob. "The home ramp inverts" (correlation +0.64 to +0.91) was the instrument; under a metric that passes its controls, the correlation is −0.010 [PM §1, §5 item 5].
    - `filmstrip` never steps the planes (§Z30, OPEN): decay 0 and 0.9 produce byte-identical images. This is the only "frozen scent" case I found [OB 13600–13651].
    - 11 of 20 founders were off the nest and read the food trail as exactly 0 [RM 3572–3575].
    - A harness that copied the sensor geometry measured the old sample point after the fix [CR 15617–15623].
    - The lab overlay scales by value/65535, so one deposit draws faint, and a smooth ~250 → ~50 trail A gradient read as flat [FP 48; IN 143].
    - The druid trail harness's frame gating printed lifetimes 12x too long, then a flat curve [DT].

24. **Frame cost and tile sleep: no failure measured.**
    - Settled planes cost 0.0014 ms with 0 tiles processed [DE 1297]. Setting trail A's fade to 0 raised tiles per pass 16.7 → 17.4, with the mean frame unchanged at 0.78 ms [PH 242–245].
    - The `u16` pass runs at 0.85–0.92x the old time [PL §3c]. The chooser costs about 9% of the ant scene's frame [CR 19418–19419]. The lay rule costs nothing measurable [AS §23c].
    - Tiles sleep only when a tile's maximum is 0, with an 8-neighbour wake so a trail can cross a seam [PH 517–524, 625–651].
    - The seam guard had been passing on 8-bit rounding [PL §3c].
    - INFERRED: the planes are allocated up front; the doc's ~40 MB per channel is the 8-bit figure [PH 497–504], so at `u16` it is about 80 MB per channel.

### Your five, checked
- **Terrain-blind planes:** confirmed [HW 1108–1109], but no measurement exists (item 19).
- **B fade 0.005 cost a fifth of the food:** confirmed (−20.6%), but that arm also changed the blend to 0.05 (item 12).
- **Trail A at 0.98–0.99 over the mound:** confirmed [CR 19694–19698]. It was measured on the goal box with an evolved founder, and "false roads" is not the repo's term (item 15).
- **No polarity, so an away term was needed:** confirmed (item 4).
- **Brain reading only a throttle on `Move`:** true of the ant before the chooser. "Scattered, not steered" is §R4's line about `Turn` on flat ground [OB 7451]; the trail version is "a timer, not a rudder" [FB §8h]. The throttle has been retired for nesting species since 2026-09-24.
  - The living reference still says the trail goes "only into `Move`" and that no other code path reads the planes [HW 1113–1116]. That appears stale against [CR 21398–21421, 21712–21752, 21108–21128, 21374].

### Numbers about the planes
- **Storage:** `Scent = u16` in 8.8 fixed point; `SCALE` = 256; range 0–65,535 raw, which is 0–256 old 8-bit units [PH 46–72]. Deposits saturate rather than wrap [PH 613–623].
- **Deposit:** `DEPOSIT` = 40 × 256 = 10,240 raw per successful move, times the brain's emit [PH 287; CR 8109–8112]. EmitB is 0.714 (wire `(CarryingFood, EmitB, 2.5)`, [ANT 1428]), so about 7,311 raw or ~29 old units per pass [PH 908–910].
- **Update interval:** `PHEROMONE_INTERVAL` = 12 frames, scaled by the creature clock [PH 110, 1226–1236; WD 6991–6994].
- **Blend:** `DIFFUSE` = 0.25, computed as `here + (mean3x3 − here) × diffuse`, rounded. Lab B uses 0.05 [PH 164, 783–791, 982; LS 1044–1049].
- **Fade (rho):**
  - Trail B: `DECAY_RHO` = 0.03 [PH 204], same value in the lab as `LAB_B_RHO` [PH 975].
  - Trail A: `TRAIL_A_RHO` = 0.0 [PH 271].
  - Environment-variable overrides: [PH 885–969].
- **Minimum fade:** 1 raw unit per pass (`min(v − 1)`), with 16-bit fixed-point decay [PH 77–78, 471–495]. INFERRED: the "255-pass" and "40-pass" lifetime ceilings in [PH 80–109] are 8-bit-era numbers.
- **Tile:** 64 cells [PH 292].
- **Alarm:** fade 0.35, deposit 240 × 256, falloff 12 × 256 per cell [PH 336, 347, 392].
- **Sensor:**
  - `sensor_offset` = 6 [ANT 1138].
  - Along-reading = (ahead − here)/(ahead + here + SCALE) [CR 8821; HW 257].
  - Row projection [CR 15624–15630]; honesty gate [CR 8827–8830, 15651–15659].
- **Chooser:**
  - `TRAIL_HALF` = `DEPOSIT`/10 = 1,024 raw, so presence = x/(1+x) of the larger of the cells 1 and 2 ahead [CR 19897, 21403–21421].
  - `TRAIL_GAIN` 3.0 [CR 19907]; `AWAY_GAIN` 1.0 [CR 19932].
- **Door:**
  - `FOOD_TRAIL_GAIN` 6.0 [CR 20507]; news window 700 frames [CR 20517].
  - `DOOR_READ_RISE` 4 rows [CR 21677]; reads 6 cells east and west on the ant's row [CR 21741–21744].
- **Measured peaks:** 15,160–25,004 of 65,535 on `u16` (39–98 of 255 on 8-bit), so the "halve `DEPOSIT` if trails pin" rule (P-14) never fired [PL §1e, §3c; PH 278–286].
- **Stale comment:** `step` still says values are 8-bit with a maximum partial sum of 2,295 [PH 694–700]. INFERRED: the sum is still exact at `u16`, since 9 × 65,535 < 2^24.