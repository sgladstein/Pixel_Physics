---
name: guest-seat
description: Guest seat demo (demo=guest, rove beetle + smell ladder) built 2026-10-25, PR #910; what it is, numbers, open judgments
metadata:
  type: project
  modified: 2026-10-25T11:34:25.546Z
---
Guest seat (one of the six seat demos, thread "Build the guest seat demo") finished all five steps 2026-10-25.
- Play: `cargo run --release --bin lab -- demo=guest` (land=, gap= args). PR #910 merged to main as b5785e8 (2026-10-25 12:38Z).
- GIF page: https://claude.ai/artifact/K2Aby5o8hVCMesmzEM7G2s. Report: Reports/guest-seat-demo-2026-10-25.md (+ planning docs in the folder beside it).
- Engine: smell ladder in src/sim/creature/guest.rs, World::visitors (None = off, hash guard). Trained (mind) ants only. Interface for other seats: seat-demos/shared/scent-ladder.md (keeper may reuse; point visitors not built).
- Body: species `rove`, Chain(3), nip hits rear cell first (head last). SteerInput::jump added (J).
- 12 seeds x 60 s: raw guest killed on 11/12, disguised (soak+rub+beg) on 5/12, fed on 11/12; colony trip_deliveries unchanged.
- Open for the owner's eye: raw death speed (~12 s), beg generosity (200->700 J/min), hold lengths. Larva kill tested but not filmed.

**Why:** a later thread picking up seats, the keeper symbiont combo, or owner feedback on the demo needs this without re-asking.
**How to apply:** owner feedback on demo=guest goes to this thread/branch claude/project-thread-rdj55n; see [[colony-mind-seat]] for a sibling seat.
