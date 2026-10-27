# Build the queen seat demo: hand-off (2026-10-27)

**Thread:** "Build the queen seat demo". **Branch:** claude/project-thread-epio3i.

## Landed on main
- PR #904, merged as 648d705: the common seat-demo launcher (`demo=<seat>`, `lab::demo`) and a keyboard-steered body on trained ants.
- PR #908, merged as a239eb0: the queen seat demo (`demo=queen`).
  - New code: `sim::fathers`; the queen's hold (`Steered::hold`, `steer::holding_queen`); `Renderer::line_tint`.
  - New examples: `examples/queendemo.rs` and `examples/queen_m0.rs`.
  - Report: `Reports/teaching-the-colony-2026-10-25/queen-seat-demo.md`, with the feasibility review, plan v1, plan review, plan v2 and M0 tables in `queen/`.

## Unmerged
- PR #947 (head 5579bcfb), with the Merge desk. It adds the 224 KB of M0 raw runs and two demo stills to `Reports/teaching-the-colony-2026-10-25/queen/`, and changes no code. This is the last file that needs saving. The 6 MB nest GIF is deliberately not kept; it is regenerated with:
  `cargo run --release --example queendemo -- seed=2 frames=40000 gif=q.gif every=200 up=8 crop=220,134,80,50`

## Open owner decision
- **The owner has not yet given a verdict on the demo.**
  - Asked 2026-10-25: does the wild-egg gamble read as fun? Pictures were on https://claude.ai/artifact/BaPrQYh4UVRA58Ewc2v751, which the new account will not be able to open; the same stills are in PR #947.
  - The owner then asked two questions, both answered in the thread:
    - Can ants learn within their lifetime? No. Weights are fixed from the egg; only last-food memory and scent change during a life.
    - How do I play it? The key guide is in the report's "What the player does" table.
  - No ruling has come since.

## To-dos, if the owner wants the demo taken further
1. Open the demo zoomed in on the nest. An ant is 2–3 pixels at 1:1, so the line colours read as specks.
2. Say on screen that a wild line stays one ant unless kept with Y, because only the queen lays while she holds.
3. The life-cycle arc proposed in `seats-overview.md`: queen, then scout, then colony mind.
4. The enemy seat's finding (PR #917, `Store::wild_all`): mutating every egg at the training page's rate drove 3 of 6 lab colonies extinct within 175k frames. Keep wild eggs a per-egg gamble.

## Known issue, not fixed
- `lab::scenario`'s tests point `PIXEL_PHYSICS_LAB_SCENARIOS` at a scratch directory while holding `DIR_LOCK`. Other tests that call `Scenario::load` (in `sim::steer`, `lab::demo` and `sim::fathers`) do not take that lock, so running them in one process can fail with "No such file". Run separately, both sets pass.

## Practices learned here that are not in the repo
None beyond what the report and `how-we-test.md` §4 already record (shuffle member order per seed; founder deaths in a lab colony follow the ant's id, not its mind).

## Jobs running
None: nothing on the VM or anywhere else.
