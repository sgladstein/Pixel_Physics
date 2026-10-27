---
name: teaching-the-colony-plan
description: 2026-10-25 game design plan for how a player teaches the ant colony (queen vs keeper seats, verbs, memory-layer trap); PR #896, Claude Doc, awaiting owner's seat decision
metadata:
  type: project
  modified: 2026-10-25T07:08:23.087Z
---

Owner ask 2026-10-25 06:30Z (thread "Design how players teach the colony", cmsg_01Mei3dwkcdzPxa5Jmuqi2bB9sxTdic6Vw2CYiCU1sbGhb): a deep game-design session on every way a player could teach the colony, outside (gardener/keeper) or inside (queen, genetics). The owner said subagents were OK.

- Report: Reports/teaching-the-colony-2026-10-25.md + notes folder (keeper, queen, perspectives, biology, ml-teaching, critique, engine-brief). PR #896 merged 3ee73ac (2026-10-25). Claude Doc: https://claude.ai/artifact/AQmZ5E5towdrhgfXhtTJyt
- Recommendation: queen as main seat (who is born / what fed / where brood lies / presence-absence / feed or bite in the act / tandem / bless deeds via replay), keep lab hand tools; keep queen body after first hatch. Settle by two FAKED 10-minute slices first.
- Critique findings (verified): shipped std_pool.txt is layout 4 so FOOD_WAY weights zeroed -> anything writing last-food memory does nothing on today's ants; a sense at weight 0 ships dead (link price strips unused senses; keeper smell untrainable). Fix: generic smell/place-value inputs, heritable priors, hand-seed or heritable-gain pull, train on foodflip bed, default-on only if learners beat frozen.
- PIXEL_PHYSICS_RECRUIT (returner contact count) already exists, left off for harm (442 vs 298/323 starved).
- Third perspectives offered to owner: guest/symbiont (favourite), colony mind, enemy, lone scout.
- 2026-10-25 07:11Z owner: write design docs for the other four seats too (guest, colony mind, enemy, lone scout). Claude Doc https://claude.ai/artifact/SQS7Efw6B47ekdLChmk4KC (overview + one tab each); landing in Reports/teaching-the-colony-2026-10-25/.
- 2026-10-25 07:19Z owner: once docs are done, one NEW THREAD PER SEAT: adversarial review + feasibility; if feasible, detailed implementation plan to a playable demo; plan independently reviewed, revised, then implemented to a playable demo. Owner confirmed SIX seats (07:21Z) and: "make sure everyone collaborates as they are all interested and some could be combined" -> shared board in /mnt/project-files/seat-demos/, shared engine pieces built once, combinations flagged.
- Four seat docs done 2026-10-25 ~08:00Z: Reports/teaching-the-colony-2026-10-25/seat-{guest,colony-mind,enemy,scout}.md + seat-*-critique.md + seats-overview.md, PR #898 merged e94382f. Verdict: queen stays main; mind+scout = later stages of queen->scout->mind arc; enemy = seasons in queen game ("price of an ant" score per delivery); guest's scent model grafted onto keeper gnome. Every draft had 3-5 must-fix engine errors -> always run an independent code-checking reviewer on design docs.
- Engine facts found: trained std_pool ants CAN Attack (slot 8 = 1 in member 0; options.rs:395 comment was stale, fixed in #898); link price has NOT yet made selection shed links (training bed never feels the bill); mind ants judge strangers by colony number, not scent, and skip act() (no stranger alarm); sim::steer drives only an ant.
- Six seat-demo threads UP 2026-10-25 07:58Z: queen cmsg_01Mei3dwkcdzPxa5Jmuqi2bBFTL11Fjw5VvcaTYAb9JLd7, keeper ..6MX7UvF4jg8EL1hHuUi6hs, guest ..QgBqwFCyX9Urs4yn39hxGt, mind ..EgF6GcJFQCizK4Vq4jzPFk, enemy ..6bSGLzkCV4hUfCkREaLmTi, scout ..4BAX9fz9kWQxgEhyKnVPS3 (prefix cmsg_01Mei3dwkcdzPxa5Jmuqi2bB); shared board /mnt/project-files/seat-demos/ (BOARD.md, status/<seat>.md, shared/<piece>.md one owner each, designs/).
- Pending owner decisions: seat; may ants learn in life; biology liberties.

**Why:** a later thread on player verbs, the queen game or in-life learning should start here. **How to apply:** see [[creatures-1996-lessons]], [[teacher-lane]].
