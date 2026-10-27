---
name: evolvable-pheromones-lane
description: Thread "Let ants evolve their own pheromones" (2026-10-25): genome-chosen scent chemistry, chemical-palette design, physics-rule question to owner
metadata:
  type: project
  modified: 2026-10-25T03:58:47.054Z
---

Thread cmsg_01Mei3dwkcdzPxa5Jmuqi2bBXcUsoiyAwhaYT7FpxPWsFj, branch claude/project-thread-iff53x. Owner asked 2026-10-25 03:54Z to explore letting the genome define pheromone physics so ants evolve their own signals; discussion first.

Initial-thoughts doc: https://claude.ai/artifact/B1AQYizEuAQb9Cvsfdv9Lh (to land in Reports/ before resolve).

**Facts found:** mind ant already emits into 3 planes (A fade 0, B fade 0.03, Alarm 0.35 active-space) via EMIT_CHANNELS in src/sim/creature/mind.rs, billed by emit_cost_in_moves 0.0625; planes carry no semantics. Lab already runs B spread 0.05 vs outdoor 0.25 (one-physics rule already bent). Switches exist: PIXEL_PHYSICS_A_RHO, PIXEL_PHYSICS_B_RHO, PIXEL_PHYSICS_B_DIFFUSE. TRAIL_A_RHO doc predicts "lifetime per species rather than per channel".

**Proposal:** chemical palette = ~5 fixed molecules on one volatility law (heavy lasts/stays, light spreads/fades), gene per emit output picks molecule, ant smells only its own molecules (brain keeps 3 scent slots). Step 1 = no-code landscape check via B switches on current founder pool. Then genes to Teacher lane.

**Owner answers (2026-10-25 04:02Z):** "fixed chemistry, gene picks molecule" IS inside the one-physics rule; ship OFF until a paired test passes; goal = BOTH better foraging AND visible emergent signals. Owner 04:08Z: adversarial review of the plan incl. feasibility of emergent communication in our training (reviewer running).

**Measured (examples/molecules.rs, uncommitted):** ground law: faster blend = vanishes sooner, not further reach (3x3 mean ~9x loss/cell); air law spreads 1 cell/pass (12 frames), square clouds; call (air, fade 0.10, fall 4) 31 cells in 180 frames. Palette v1: road(g,0,0.05) trail=A news=B fresh(g,0.10,0.25) alarm call. Plan: /mnt/project-files/evolvable-pheromones/plan-2026-10-25.md; sheet molecules-sheet.png.

See [[maze-lane]], [[trail-teacher]], [[teacher-lane]], [[body-genes-lane]].

History of runs: [[evolvable-pheromones-history]]. All results in Reports/brain-training/evolvable-pheromones-2026-10-25.md (s7 first test, s9 follow-ups).

New-channel run results (2026-10-26): see [[evolvable-pheromones-history]].
**2026-10-27 (owner 15:03Z: "I don't want to give up"):** full write-up doc https://claude.ai/artifact/Vt8E28wJ3A5vjSfSuYPXHX. Independent review (Opus sub-agent, scratch review/): my "1.4-2.2x on 15-16/16" was one genome per arm, n=2 replicates; rep 2 ~1.1-1.2x and closing; deaf arm still evolves A/B signals (relies on B 470->380, 8/8), so the result is only "third channel didn't beat two evolvable ones". Main suspect: ~70 extra live channel links kicked by every mutation (mutant/parent 0.66-0.85 free vs 0.84-0.92 deaf), lineage collapse, bed may leave nothing to say. emerge.py bugs: .raw.txt founder ignores ZERO/FILL; resume duplicates rows; elites no score history; extinct = 0 despite stored. Plan E1 mutant-load test (~1.5 h) -> E2 room-for-signal on beds -> E3 harness pilot -> E4 6x replicates on VM (ask owner) -> E5 evolvable vs frozen A/B.
**E1/E2 (10-27 ~17Z):** E1: free genomes' mutation load is mostly from NON-channel slots (2/3; map-driven fragility); gentle 0.013:0.25:1 fixes 5/6 (free rep2 best 0.70). E2: >=56% of empty out-of-nest ants miss the heap corridor after a finder on every bed incl. today's -> E4 on today's bed. Harness fixes + report corrections PR #943 (merge desk). E3 (HARNESS=1 vs 2, deaf, seeds 31/32, 40 gens) running in scratchpad e3/. Rule: run E4 with HARNESS=2.
