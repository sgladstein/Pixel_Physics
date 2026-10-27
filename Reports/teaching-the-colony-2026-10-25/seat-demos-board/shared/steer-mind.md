# Shared piece: steering any body that has a MIND (trained ants, spider, centipede)

Owner: **enemy** seat (thread "Build the enemy seat demo", branch claude/project-thread-1ekeut).
State: PROPOSED 2026-10-25 08:30Z. Needed by: enemy (centipede/spider body), guest (a non-ant body, if it has a mind),
and **queen + scout whenever the steered ant is a trained (mind) ant**.

## Why this is its own piece (engine fact, checked 2026-10-25)
`sim::steer`'s five hooks all live in `creature_tick`'s hand-built path. Any creature with a mind returns early into
`mind::tick` (`src/sim/creature.rs` ~7998 -> `src/sim/creature/mind.rs:2234`), which reads no steer input at all.
The lab opens on the trained ant by default (`src/bin/lab.rs` ~297), so **`steer::mark` on a lab worker or queen does
nothing today** unless the colony is the hand-built ant. Trained predators (spider, centipede) are minds too.

## Interface (proposal)
- Same entry points as today: `steer::mark(world, id, Some(SteerInput))`, `Steered::press`, `steer::take_input`.
  The queen's key layer (shared/steered-ant.md) fills `SteerInput` exactly as for a hand-built ant; nothing new for callers.
- New: in `mind::tick`, after the mind's own decision, **the player picks the option** (precedent: the scratch
  `d.pick` overrides PP_OLDLAB_ACT / PP_TRAIL_ACT already in that function):
  - `dir` held -> the Walk option whose heading is nearest `dir` (`steer::pick_heading`), within 90 degrees; none -> Rest.
  - `bite` -> the Attack option nearest the facing/dir; `dig` -> Dig option nearest dir; `eat` -> PickUp/Eat;
    `drop` -> Drop; `share` -> Feed. A pressed verb with no such option on the list -> falls back to the dir rule.
  - No key -> Rest (the body stands), exactly as the hand-built steer does.
  - The mind still ran, so its memory/hidden state keep time; handing back (input None) resumes the genome.
- Off is off: `World::steered == None` -> one Option read, byte-identical.
- Bodies: chain bodies (centipede) keep their following-chain physics; falls, bills, handling/satiation unchanged
  (a sated predator still cannot bite: that is the game's hunger ceiling).

## Status
**PR #903** (branch claude/project-thread-1ekeut, head 83b7e037), with the merge desk, 2026-10-25 ~09:45Z.
Final shape: `steer::ForcedPick { dir, verb, once }`, `steer::force(world, id, pick)`, `steer::unforce`,
`steer::take_forced` (called in mind::tick before deciding), `steer::choose(opts, head, pick)`; `World::mind_force`.
Memory follows the forced verb (scratch_log::remember). Dir: nearest Walk within 90 deg, else TurnAround, else (level
hold) walk straight up, else Rest. Tests in sim::steer; red with the hook removed.
