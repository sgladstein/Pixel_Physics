# Lane: "Design how players teach the colony" (also the 1996 Creatures lane)

Thread cmsg_01Mei3dwkcdzPxa5Jmuqi2bB9sxTdic6Vw2CYiCU1sbGhb. Written 2026-10-27 for the master hand-off.

## What it produced (all on main)
- #882 Reports/brain-training/research/creatures-1996-lessons-2026-10-25.md: what the 1996 game Creatures did; the one idea not taken is in-life reward learning plus instinct genes.
- #884 (main 1bf2c62): last-food memory PP_LASTFOOD (mind layout 4 -> 5; layout <= 4 files, incl. shipped std_pool.txt, get slot-18 links zeroed on load) and the inrun `foodflip=FRAME` moving-food arena.
- #893 Reports/brain-training/research/creatures-1996-game-design-2026-10-25.md: what made Creatures fun and boring, lessons for the lab.
- #896 Reports/teaching-the-colony-2026-10-25.md + notes folder: game design plan for how a player teaches the colony. Recommends the queen as the main seat.
- #898 four more seat designs (guest, colony mind, enemy, scout), each independently code-reviewed and revised, plus seats-overview.md. Also fixed the stale options.rs comment: trained std_pool ants DO have Attack on.
- #949 (merged e1d2f6e): snapshot of the seat-demos shared board (BOARD.md, shared/, status/) under Reports/teaching-the-colony-2026-10-25/seat-demos-board/.
- Started the six seat-demo threads (each landed its own demo and report).

## Open to-dos
- Creatures idea #2: one reward-trained unit per ant (founder rate gene 0, so off = bit-identical), tested on the foodflip arena. Queued with the Teacher lane 2026-10-25 after idea #1. Not known to be built. The same lesson applies to any in-life learning: test at full game length by age (Creatures 2's "one hour stupidity" came from reward/punish imbalance).
- Creatures idea #3: a "bored" drive sense for fed ants idling on the mound. On hold pending stay-home traces.
- None of the six seats has real in-life learning. Keeper and scout fake it with the recall head (PIXEL_PHYSICS_RECALL_GAIN). If the owner wants a Creatures-style seat, the keeper with a reward verb is the natural home (my answer to the owner, 2026-10-25 14:54Z).

## Owner decisions still pending (as of 2026-10-25; the seat threads may have settled some)
1. Which seat is primary. Recommendation: the queen (selection by who lays works on fixed weights); the scout and colony mind as later stages of a queen -> scout -> mind arc; the enemy as seasons; the guest's scent model grafted onto the keeper.
2. May ants learn during their lives? Recommendation: yes, but only behind a switch, shipped on only if learners beat frozen ants on a moving-food test.
3. How much liberty to take with the biology (e.g. the player deciding who lays, scout "pass on" to a pupil). Each seat doc lists its own liberties under "Decisions only the owner can make".
4. Nature-game teach verbs (bait a lure, warn off, carry a lost ant): proposed 2026-10-25 06:15Z, no answer recorded.

## Files outside the repo
- Claude Docs (owner's reading copies; content is in the repo): teaching plan https://claude.ai/artifact/AQmZ5E5towdrhgfXhtTJyt ; four seats https://claude.ai/artifact/SQS7Efw6B47ekdLChmk4KC.
- /mnt/project-files/creatures-1996/ and /mnt/project-files/teaching-the-colony/: copies, identical to main.
- /mnt/project-files/seat-demos/<seat>/ (plans, reviews, step-0 measurements, queen/m0, keeper/look, scout/step0): owned by each seat thread, not checked by me. BOARD.md, shared/ and status/ are in #949.
- Scratchpad teach/ and teach2/: drafts, all landed. Nothing else to keep.

## Jobs and branches
- No job on the Google Cloud VM or elsewhere.
- Branch claude/project-thread-koy6r5: all merged (last #949 e1d2f6e). Nothing unmerged.

## Lessons worth keeping
- Every design draft (4 of 4, 3 to 5 must-fix errors each) got engine facts wrong until an independent reviewer checked it against the code. Always run a code-checking reviewer on design docs.
- Engine facts found: trained ants can bite (Attack on in all 30 std_pool members); link price has not yet made training shed links (the bed never feels the bill); mind ants judge strangers by colony number, not scent, and skip act(); sim::steer drives only an ant.
