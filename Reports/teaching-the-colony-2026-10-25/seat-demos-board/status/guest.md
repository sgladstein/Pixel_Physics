# guest seat: status

Thread: "Build the guest seat demo" (cmsg_01Mei3dwkcdzPxa5Jmuqi2bBQgBqwFCyX9Urs4yn39hxGt)
Branch: claude/project-thread-rdj55n

Step: ALL 5 DONE 2026-10-25 ~11:40Z. Playable: `cargo run --release --bin lab -- demo=guest`. PR #910 MERGED to main as b5785e8 (12:38Z, CI 15/15 green). GIF page: https://claude.ai/artifact/K2Aby5o8hVCMesmzEM7G2s. Report: Reports/guest-seat-demo-2026-10-25.md.
Result (12 seeds x 60 s): raw guest killed on 11/12, disguised on 5/12 and fed on 11/12; colony food runs unchanged.
Verdict: FEASIBLE WITH CHANGES, as "the beggar" only. Review: seat-demos/guest/review-feasibility.md. FOR EVERYONE: the shipped beetle (2x2 rigid) cannot enter a nest (galleries 1-2 wide); trained ants kill a visitor of another colony number in ~100 frames and its scent changes nothing (probe, examples/guestprobe.rs); given the host number it is never bitten.
Plan: seat-demos/guest/plan-v2.md (v1 + its review beside it). Box: steady_income (played bed grass mat blocks the rove; played bed 30k-frame warm-up = 57 s on a 4-core box). Trained ants only.
Shared pieces: OWNER of shared/scent-ladder.md (ants grade a visitor by smell; keeper's gnome is post-demo).
  Using queen's launcher (DemoSeat::Guest, demo_guest.ron, src/lab/demo/guest.rs) and steered-ant key layer.
  Body: my own species `rove` (ant body, hand-built brain) steered by the existing creature_tick hooks; not steer-mind.
Combinations: keeper + guest (keeper as symbiont, after both demos); enemy registers its raider on the ladder if wanted.
