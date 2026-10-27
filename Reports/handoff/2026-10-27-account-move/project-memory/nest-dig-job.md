---
name: nest-dig-job
description: Why the trained nest stays small (2026-10-26): crumbs fill ~60% of the dug room; the Dig job sits at its floor; a stronger depth->Dig weight failed
metadata:
  type: project
---

Teacher lane, 2026-10-26. Setup: homelow_pool (hf8) with the sense flags. Tools in the scratchpad ns/: where.py, digjob.py, inpellets.patch (a tally of pellet releases).

**Where the room goes.** I flooded the nest from the door over 8 worlds (hl/t/hf8g.*):
- Open cells: median ~381.
- Crumbs (half-eaten fruit put down): median ~616.
- Together that is ~1,000 dug cells. The hand-built ant on world 13007 has 1,438 open and 9 crumbs.
- So the gap is about 60% crumbs and 40% less digging.
- Crumbs cycle: on world 13007 they peaked at 536 at frame 125k, then fell to 315 as the colony ate them.

**Soil "let go inside" (spoil_in ~360) is mostly not real.**
- Every release is the mind's own Drop, by fed ants.
- About 2/3 of them land at row 0 of the door column. The world clears soil there (PP_MIND_DOOR_OPEN=drop keeps the clear).
- Only ~80-100 pellets per run are dropped 12 or more rows deep.

**The Dig job is at its floor.**
- Underground, with empty jaws and a cut in reach: Dig picked 25 of 1,504 decisions.
- Job sum about -6.3: bias -5.65, hidden -4.23, a negative weight on the best cut's score, at_nest +5.86.

**Failed:** raising the Dig-job depth weight (slot 1590) to 5, 9 or 14 (was 1.65) over 8 worlds.
- Open nest 363 / 403 / 263 vs 378.
- Crater 37-40 vs 7.
- Dug cells barely moved.

**Owner rule:** no colony-wide count senses, so use local ones only (like the lay gate's food or crowd in reach).

**Next idea:** a local "larder crowding" sense at home (crumbs and food in reach) to drive Dig. It needs a slot, so it waits on the layout decision.

See [[nest-priorities]], [[home-lesson-fit]], [[dig-lesson-oldant]].

**Larder-dig hand test (PP_LARDER_DIG=k, Dig-job term, scratch, not landed) FAILED.** Dig picks underground rose from 2% to 30%, but cuts rose only 7%, open room fell to 158-282, and crumbs rose.

**Hauling is the bottleneck.** Underground, half of all decisions are an ant carrying a soil pellet, and a third of their steps go deeper.

**PP_SOILWAY=1 (options.rs, scratch, needs PP_HOMELOW) WORKS.** It writes slot 18 for an ant carrying a soil pellet inside the nest: +1 for a step toward the door along NestWay.at, -1 for a step deeper, and +1 for a step out. It reuses home-low's fitted weights, with no new fit.
- 8 worlds vs hf8g:
  - open 644 vs 378 (6/8 higher)
  - dug 1,273 vs 1,046 (8/8 higher)
  - starved 30.5 vs 43.5
  - alive 79 vs 78
  - crumbs 486 vs 616
  - food home 3,918 vs 4,890
  - BUT crater 60 vs 6.5 (7/8 higher)
- Worlds 9-16 and a film were running at 09:40Z (ns/run6.sh).
- Worlds 9-16: open 546 vs 470 (6/8), crater 123 vs 40, starved 43.5 vs 44, food -21% (8/8). Page https://claude.ai/artifact/CdtFxitcZ9yau316uAV2hP.
- Crater cause (first look): shallow cuts (0-7 rows) unchanged; the single cavern widens at the bottom (cuts 16+ rows deep 1,051 vs 843 per world) and its top slumps into a funnel.
- PP_SOILWAY committed 3feb590a (branch, default off). nest11 launched ~10:35Z on the VM at that SHA: swctl and swshape from homelow_pool with SOILWAY on (the hlctl/hlshape analogues of nest10 are the no-SOILWAY reference). Deciding measure: open nest and crater, paired against homelow_pool + SOILWAY over 16 worlds. Waiter: runvm-lg2.sh to $S/nest11.
- nest11 DONE 11:48Z (1.8 h). Training: colonies died out more (7-26 of 32 alive per round). Garden r47 crater 103-173. Its 4 best colonies (carries in scratchpad n11c/) were all worse than homelow+SOILWAY (open 231-582 vs 644, cuts 630-1,000 vs 1,273), so selection bred digging out. Logged + dead-end (Dig-job pushes) in PR #931 (head 80ee55f7) with the merge desk. Page v2 updated. Next idea: nest SHAPE (rooms) as the crater fix. Owner card on brain slots (layout 6) is still open: cmsg_01Mei3dwkcdzPxa5Jmuqi2bBAXtwxCGkm53oWqRZrngmLH.
