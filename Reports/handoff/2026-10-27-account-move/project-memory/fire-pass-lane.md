---
name: fire-pass-lane
description: Fire pass speedup thread (2026-10-22): FireTraits table + get_four, box -3.6% / plant bed -2.8% Ir bit-identical, branch claude/project-thread-j3ettt
metadata:
  type: project
  modified: 2026-10-22T14:53:34.867Z
---

Fire pass speedup thread (cmsg_01Mei3dwkcdzPxa5Jmuqi2bBHEutFeyTkBTz7SoErbMwfs), split off [[lab-world-perf-lane]] 2026-10-22. Branch `claude/project-thread-j3ettt`, based on main e7f294c.

- What landed on the branch: `fire::FireTraits` (compact per-material guards: inert, conductivity, flammability, ignition/melt/boil/cool points + has-product bools, reacts), served by `CellSurface::fire_traits_of`; ChunkView reads a table `parallel::step` builds once a frame (registry is never edited mid-step; get_mut is tunables/tests between frames). `phase_change_is_quiet` skips try_phase_change when it would draw/write nothing; try_react skipped when no reactions. diffuse_heat returns its 4 neighbour reads to try_ignite. `CellSurface::get_four` + `Chunk::get_four_world`: one bounds test + one index when the cross is inside the chunk. update_from no longer re-checks EMPTY (callers do).
- Callgrind cg=1 window 1200, single thread: box 6.492B -> 6.260B (-3.6%), plant bed 5.130B -> 4.986B (-2.8%); hashes identical. NOTE these window totals (6.49B box) differ from the 1.739B quoted in lab-world-perf-lane; compare only same-command runs.
- After the change fire.rs self cost is ~2.6% of box, ~2.1% of bed; what is left is mostly the 4 neighbour reads per conducting cell and the float diffusion math.
- Env traps: line-level profile needs a separate build with `CARGO_PROFILE_RELEASE_DEBUG=line-tables-only CARGO_TARGET_DIR=...`; a quick callgrind line aggregator lives at /tmp/claude-0/fire/tools/lines.py (scratch, gone with the container). docscheck exits 1 in this container from a Python DeprecationWarning in deadendindex.py, not a real failure; deadendindex refuses to regenerate in a shallow clone (`git fetch --unshallow origin main` first).

- Shipped as PR #819, MERGED to main as ed4c2a8 (2026-10-22, all checks green). perfbench 8 rounds: box -3.3%, plants -3.1% wall, hashes identical on all 7 scenes; fetch/trail wall +5/+14% was noise (callgrind -0.09/-0.25%).

**Why:** a later perf session should not re-profile the fire pass or retry these.
**How to apply:** PR goes to the merge desk ([[merge-desk]]); don't watch CI.
