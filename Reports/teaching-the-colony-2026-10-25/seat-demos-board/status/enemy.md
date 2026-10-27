# enemy seat: status

Thread: "Build the enemy seat demo" (cmsg_01Mei3dwkcdzPxa5Jmuqi2bB6bSGLzkCV4hUfCkREaLmTi). Branch: claude/project-thread-1ekeut.

**State (2026-10-25 11:30Z): demo built, PR #909 merged as 6033776; birth mutation PR #917 merged as 7ffd841.** All five owner steps done: feasibility
review, plan v1, independent plan review, plan v2, demo. Report: Reports/teaching-the-colony-2026-10-25/enemy-seat-demo.md
(plans in enemy/ beside it). Owner page: https://claude.ai/artifact/Pesqv6DRSuAkWf5vWayg5V

Play: `cargo run --release --bin lab -- demo=enemy` (`oldant` = hand-built colony). Centipede lands 70 cols from the
door at frame 26,000 after a visible dig-in. Keys: arrows, B bite, E eat, Q spit/put down, G cut crumbs, Y card, R new raider.

Findings for other seats:
- 36 scripted runs (examples/enemyhunt.rs, main 648d705): the door is defended, 12/12 raids die on the mound, both
  brains. Stalking lone foragers from 70 columns out: trained colony kills the raider 4/6, hand-built 0/6.
- The hand-built ant's gnaw (Eat at an animal cell) wounds with NO ENCOUNTER_BITE row: count bites taken off the
  body's `gnawed` meter, not encounter_log.
- examples/enemyhunt.rs drives any steered body through Lab::demo_key end to end; `drawcost` prices a demo overlay.
- Raider is amber like the ants: a seat body needs its own colour at hunt zoom. The BATTLE panel auto-opens at the
  first predator kill and covers a top-left HUD (enemy.rs turns it off at landing).
- `warm=` must not land a body before the colony has a door (nest_sites empty).

Shared pieces: shared/steer-mind.md (PR #903, merged 0ea4e4e). Hooks added in #909: Steered::may_breed, Steered::last_act.
Open: Season 2 (colony adapts across seasons) awaits the owner's decision card.
- 2026-10-25 owner playtest: no-spit fix merged (#920, 92a8b65); hit and run (Shift sprint, centipede regrows tail) PR #922.
