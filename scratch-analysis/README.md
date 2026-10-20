# Scratch instruments from the 2026-10-20 brain investigation (NOT for main)

Code changes on this branch, all env-gated and off unless set:
- `src/sim/creature/mind.rs` `scratch_log`: `PP_MIND_LOG=<file>` writes one `M,...` row per mind decision
  (seed, frame, ant id, x, y, chosen verb, target x, y, last verb, 22 senses, 12 memories before the decision,
  9 job scores, 9 best-target scores, the chosen target's 45 features). `PP_ABLATE=name,name` makes those senses read 0
  at every decision.
- `src/train/challenges.rs` (soil arena): with `PP_MIND_LOG`, every cut (`C`: seed, frame, id, x, y, depth, was ground),
  put-down (`D`: seed, frame, id, x, y, away from the hole) and a world-end row (`E`: seed, full loads, full cuts).
- `src/train/mod.rs`: `PP_GIF_EVERY` / `PP_GIF_DIR` frame dump (`Reports/brain-training/tools/gif-frames.patch`);
  `examples/train.rs`: `PP_GIF_SEALED=k` draws sealed world k for `pic=`.
- `examples/reliab.rs`: every genome solo on two independent 48-world sets, to a CSV.

Analysis (Python; run with `python3 -I`):
- `pellets.py LOG [SHAFT] [FRAMES]`: every pellet's fate, rebuilt with the scorer's jaws rule (asserts it reproduces each
  world's full loads), the time budget, trip phases, and every cut traced to the decision that made it.
- `brainmath.py GENOME LOG`: the wiring map, the exact job-score recomputation (checked against the logged scores), and
  per-term attribution of verb choices by situation. Layout constants mirror `mind.rs` (GENOME_LEN 2797).
- `reliab.py CSV`: rank agreement, variance split, worlds needed, offline racing schemes.
- `biblio.md`: literature notes.

Log a brain: `PP_MIND_LOG=/tmp/x.log PIXEL_PHYSICS_MUTATION=off target/release/examples/train brain=mind challenge=soil
step=4 shaft=4 foot=1 budget=1200 holdout=24 sealed=1 start=<start> score=<genome> threads=1 noise=1`
(`threads=1` keeps the worlds in order, so rows can be assigned to worlds; an `E` row closes each world).
