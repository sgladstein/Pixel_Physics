# queen seat: status

Thread: "Build the queen seat demo" (cmsg_01Mei3dwkcdzPxa5Jmuqi2bBFTL11Fjw5VvcaTYAb9JLd7)
Branch: claude/project-thread-epio3i

Step: 4 DONE (plan v2), step 5 (build) in progress, 2026-10-25 09:55Z.
Verdict: FEASIBLE WITH CHANGES. Review: seat-demos/queen/queen-feasibility-review.md. Core that works on trained ants: the queen as the colony's only layer (lay rate), choosing whose mind each egg carries, a wild (mutated) clutch, feeding larvae. Dead on trained ants: provision/castes, walk genes, brood carry/gravity, tandem memory (FOOD_WAY zeroed), kin bite, shelf-as-father (drops the mind). Key risk: 30 near-clone minds + no in-box mutation = selection shows uniformity, not behaviour; and queen rule starves the colony if the player stops laying (measured 41->27).
Plan: not yet.
Shared pieces I own: launcher.md, steered-ant.md (built on my branch, tests green; mind ants now obey keys), who-lays.md.
Shared pieces I expect to need: "choosing who lays" hook (queen owns it unless colony-mind claims it first); common demo launcher.
Combinations: queen -> scout -> colony mind (life-cycle arc); queen + enemy (seasons). Will post interfaces in shared/ before building.

## 09:55Z
- Launcher + mind-ant steering: PR #904 at merge desk.
- Plan v1 -> independent review (sound with changes) -> plan v2: seat-demos/queen/plan-v2.md.
- M0 done (seat-demos/queen/m0/): pool members barely differ (whose-line is weak); wild egg is a real gamble (3x: 2 jewels/11 broken of 20; 10x: 2/17, visibly odd diggers). Core verb = wild egg; Y keeps a jewel's line.
- Who-lays store (shared/who-lays.md) is being built now on my branch: World-level father store with weights + p + a queue; colony mind can drive weights.
- Finding for everyone measuring lab colonies: which founders starve in the thin start follows the ant's id, not its mind (40 of 52 on steady_income). Shuffle member order per seed.

## 11:40Z: BUILT, step 5 done (PR #908 at the merge desk, head 3831f7bd)
- Playable: `cargo run --release --bin lab -- demo=queen` (seed=N). Pictures: https://claude.ai/artifact/BaPrQYh4UVRA58Ewc2v751
- Report: Reports/teaching-the-colony-2026-10-25/queen-seat-demo.md (+ queen/ with review, plans, M0 data).
- Built and shared: sim::fathers (World::fathers, who-lays), the hold (Steered::hold/last_lay, steer::holding_queen), Renderer::line_tint (id -> colour over any creature mode, adults + brood), demo `seed=N`, demo opens without the welcome window, seat_takes() in demo_key (merged in #904).
- Instruments: examples/queendemo.rs (plays any seat headless through demo_key/demo_frame; GIF/stills), examples/queen_m0.rs.
- For everyone: L in a shaft lays nothing silently (no pile site) -- the queen demo now says "no room for an egg"; an ant is 2-3 px at 1:1, so line colours need zoom (my GIF is 8x).
- Next (after owner verdict): open zoomed on the nest; say why a wild line stays one ant until kept; queen -> scout handoff.

## 12:20Z: MERGED
- PR #908 merged as a239eb0 on main. Waiting on the owner's verdict on the Artifact.
