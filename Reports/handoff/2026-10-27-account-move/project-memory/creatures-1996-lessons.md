---
name: creatures-1996-lessons
description: 2026-10-25 research on the 1996 game Creatures (Norns): the one idea not yet taken is in-life reward learning + instinct genes; ranked ideas sent to Teacher lane; PR #882
metadata:
  type: project
  modified: 2026-10-25T03:16:48.932Z
---

Thread "Learn from the 1996 Creatures game" (cmsg_01Mei3dwkcdzPxa5Jmuqi2bB9sxTdic6Vw2CYiCU1sbGhb), owner ask 2026-10-25 03:05Z. Report: Reports/brain-training/research/creatures-1996-lessons-2026-10-25.md, PR #882 (docs only, handed to merge desk).

- Already taken from Creatures in rev 3: verb-then-target, drives as senses, one machinery per species, zero-entry growth.
- NOT taken: learning within a life (reward = a need going down; Creatures' "susceptibility" = eligibility trace; short/long-term weights) and instinct genes (lessons stored as data, replayed in sleep). Our mind ant has no in-life learning (rev 3 listed it as a non-goal "for later").
- Ranked ideas sent to the Teacher lane 2026-10-25: (1) last-food memory (rev 3 must, never built; f::LAST_FOOD slot 40 is borrowed by PP_HOMEVEC, needs a new slot); (2) one reward-trained unit per ant, founder rate gene 0 so bit-identical / can ship on, moving-food arena as positive control; (3) "bored" drive sense for idle fed mound-sitters; later instincts replayed in Rest, and a reward/punish player verb in the lab.
- Creatures failure to test against: Creatures 2's "One Hour Stupidity Syndrome" came from reward/punishment imbalance; Norns also unlearned eating. Any in-life learning must be tested at full game length (200k+ frames) by age.
- **Idea 1 BUILT and MERGED (2026-10-25), PR #884 -> main 1bf2c62:** last-food memory `PP_LASTFOOD` (default on, fade 4000 frames; off = old mind). Feature lives in slot 18 (f::BUILDING, aliased f::FOOD_WAY), NOT a new column (would shift every saved pool + slot tools). mind::LAYOUT 4->5: files marked layout<=4 (or unmarked) get slot-18 scorer links zeroed on load (mind::enter_last_food) in train::load_mind, lab std_pool/spider pool, inrun founders/spiders, labsurvive. Building scent will need its own slot. Panics with PP_HOMEDIR=1/PP_BOTTOM. Control test: wired walker 41 cols vs cut -5. Also inrun `foodflip=FRAME` moving-food arena (for idea 2). Teacher lane runs #1 as a colsel arm once merged; #2 after #1; #3 on hold for stay-home traces.
- **Game-design follow-up (owner 06:02Z 2026-10-25), PR #893 merged 31b1eb0:** Reports/brain-training/research/creatures-1996-game-design-2026-10-25.md. Fun = bond with one creature, player's hand teaches it for life, genetics surprise, death matters, sharing community. Boring = babysitting (starving beside food, OHSS unlearning), no goal, unreadable behaviour. Lab ranking: survival in owner's box first (our current state IS Creatures' worst failure), then reward/punish verb (needs in-life learning), ant biographies + marked deaths, job icon over selected ant, visible genetics, sharing lines + player goals. UI items (3,4,7) not handed to Lab interface lane; await owner.
- **Nature-game version of the teach verb (proposed to owner 06:15Z 2026-10-25, awaiting answer):** no tickle; the player arranges consequences and the ant learns (real ants learn odour/landmark-food associations in 1-2 visits). Verbs: bait a lure (player's flower/scent mark + food -> learned attraction, extinguishes), warn off (thorn/spider -> avoidance), carry a lost ant to food (learns route), lessons -> instinct over generations. Path: box survival -> idea 2 in-life learning (rate gene 0) -> one 'player mark' sense. Asked owner whether to start #2 now with the bait test.
- Environment: this container's network policy blocks creatures.wiki, Wikipedia, fan sites (WebFetch/curl fail); WebSearch summaries work.

**Why:** a later thread proposing lifetime learning or "instincts" should start from this, not redo it. **How to apply:** see [[teacher-lane]], [[far-food-1024]], [[far64-lab-playtest]].
