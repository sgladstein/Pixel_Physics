# Worldgen data track, round 3 — cave systems worth finding

The owner looked at a vault and said the true thing: **it is a plain
oval.** A bubble, not a cave. This round turns the sealed-vault pass into
sealed cave *systems* that read as natural caves, using the shape language
the project's own research already picked out
(`Reports/worldgen-design.md` §7): **Worley F2−F1 — near zero along cell
boundaries, so thresholding it yields chambers linked by passages** — one
mechanism producing exactly the chamber-and-passage anatomy real caves
have, and cheap here because the world is 2D (one field, no sheet
problem).

Same contract as rounds 1–2, inherited whole from
`Reports/worldgen-implementation-tasks-2026-08.md`'s Ground rules —
branch `claude/worldgen-data-track-r3` from
`claude/game-world-gen-planning-h12713`, image-backed commits, findings
over improvisation, gates green at every land (`cargo test`,
`clippy -D warnings`, at-rest suite, `worldgen_sweep.sh compare`). The
Findings sections of rounds 1–2 carry forward; append round-3 findings to
the round-1 file. Owned/forbidden files unchanged, with the standing
note that `examples/viewshot.rs` belongs to the reviewing session — its
`vault=` mode may need retargeting for systems; flag any change you need
there the way round 2 did (small additive edits are acceptable, flagged).

**What must not change**: the total-seal contract (whole envelope + rind
verified stone before a single write; reject wholesale otherwise), the
depth band (`vault_min_depth` / bedrock margin), rarity (a system is
still an event — 0–2 per world), arrives-at-rest, and the three-GLOBAL-
passes pin. A cave system is bigger than a vault but it is still sealed,
bounded, and genesis-only.

## Task 1 — Chambers linked by passages (the anatomy)

Replace the grotto's union-of-ellipses interior with a **Worley F2−F1
field thresholded inside a bounded envelope**:

- Envelope: ~180×70 cells (constants, tune by eye), centred on the
  placement draw, entirely inside the depth band. Feature points from a
  new appended `Purpose` stream, density such that the envelope holds
  ~6–12 Worley cells — chambers the size range the r2 arithmetic already
  cleared (widths ≤ ~36) with passages between them.
- Threshold `F2−F1 < t` gives the void. Two sub-thresholds worth
  exposing as constants: `t_chamber` near cell centres... measure and
  look rather than trusting this sketch — the acceptance is the image.
- **Keep only the connected component containing the seed point** (flood
  fill over the collected void cells, 4-neighbour) — a disconnected
  satellite chamber is a second system nobody can reach from the first.
- **Bedding anisotropy**: evaluate the field in a sheared frame so the
  system elongates *along the local strata dip* — `column.rs`'s
  `strata_offset` gradient is the dip, the same one lenses lie along
  (round 1). A cave following the visible bedding is what makes it read
  as geology rather than as noise.
- **Ceiling-span guard**: after collection, for every maximal horizontal
  run of void with stone directly above, if the run exceeds ~36 cells,
  raise the floor of the *threshold* locally (drop void cells from the
  run's middle upward) until compliant. State the final guard in a test
  that walks every ceiling run of a forced-generation world.
- Seal: bounding box of the kept component + 2-cell rind, all stone,
  else reject the whole system. Same collect-then-verify skeleton.

## Task 2 — Speleothems (the wonder)

Stalactites and stalagmites: short attached fingers grown from ceilings
and floors after the void is cut —

- 1–2 cells wide, 2–7 tall, tapering (wider at the root); material:
  stone family for most, **crystal for a minority** (the geode material
  round 2 added), noise-drawn per formation.
- Placed on ceiling/floor cells drawn from a `Purpose` stream, denser
  where the ceiling is high (drip height), occasionally paired
  (stalactite above stalagmite, almost meeting) — pairs are the
  postcard shot; a few per system, not a forest.
- Attached stone, structurally trivial (short, rooted in the massif);
  they must survive the at-rest suite untouched. Never let one bridge
  floor to ceiling fully (a column splits the passage the player walks).
- The geode vug stays as a rare second system type — a single
  crystal-lined bubble is the jewel variant — but its lining gets
  thickness variation (1–3 cells, noise) so the rim is no longer a
  perfect ring.

## Task 3 — Floors and water (the variety)

- **Breakdown floors**: keep the flat gravel fill as the base, then heap
  1–3 gravel mounds per large chamber (repose-shaped, routed through the
  existing `taper_cover`-style clamp so they arrive at rest) — a cave
  floor is rubble, not tile.
- **Aquifer waterline per system** (the research's aquifer note, and the
  round-2 ruling's queued variety): one draw per system picks a
  waterline — `dry` (above every floor), `pools` (between the lowest
  and median chamber floor), or `flooded` (above the median). Water
  written as ponds writes it (level, full) in every void cell below the
  line. Connectivity does the rest: low chambers pond, high ones stay
  dry, and one system can hold both — which is the picture worth
  finding.

## Task 4 — Counters, gates, images

- The pass table row becomes `caves` (or keeps `vaults` — your call,
  say it): report systems placed, chambers, passage cells, speleothems,
  water cells. Sweep re-baselined at both sizes with the r2 dual-size
  guard pattern (zero at 512×320, non-zero at 2048×640 across the seed
  set).
- Forced-generation test preset (high density, shallow band) so the
  at-rest suite, ceiling-run guard, seal assertion and determinism hash
  all exercise a real system at harness size. The r2 lesson stands: a
  guard that cannot see the feature has no teeth.
- Acceptance images, the deliverable the owner will judge: `viewshot
  vault=1 reveal=1` showing a branching multi-chamber system in
  magenta; a breached-system strip with the shaft entering a chamber;
  and an interior crop (the reviewing session will zoom). If `vault=1`'s
  chamber-finder needs to target the system's largest chamber, flag the
  viewshot edit.

## After task 4

Stop and push. The erosion-driven work (boulder realise pass, world-age
plumbing) remains gated on the frontier erosion core, which is being
built in parallel — do not touch `column.rs`'s elevation chain beyond
what task 1's bedding read needs (read-only use of `strata_offset`).
