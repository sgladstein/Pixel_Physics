# Lane A, round 36 — the food economy, made readable

*Branch `claude/evolution-lab-food-readable`. Brief:
[`../evolution-lab-round-36-brief-2026-09-14.md`](../evolution-lab-round-36-brief-2026-09-14.md)
§"Lane A". What round 35 shipped is
[§4 and §5 of its record](../evolution-lab-round-35-2026-09-14.md).*

**The three verdicts this lane exists to answer**, in his words:

| card | his words |
|---|---|
| `…063034460Z-989679` road + harvest, planted bed | *"No i cannot really tell. the amber hatch it bad. is it too much to track actual paths and make trail?"* |
| `…063101470Z-06554c` road + harvest, far larder | *"Why is the amber hatch drop like a huge box?"* |
| `…063026275Z-e7b8f3` the FOOD page | *"Nope. I don't understand what these visuals are trying to tell"* |
| `…063118804Z-711e5a` empty-handed walking | *"both is interesting"* — keep both arms, settled |

---

**The whole account — the reproduction, the two repairs, the cost
measurements, the page rebuild and the two defects found writing it — is
[`../evolution-lab-food-readable-2026-09-14.md`](../evolution-lab-food-readable-2026-09-14.md).**
It was promoted out of this note at the 12 KB lane cap.

## What another lane needs from here

- **`src/render.rs`, `src/food_road.rs`, `src/lab/ui.rs`, `src/lab/mod.rs`,
  `examples/labui.rs`, `examples/foodroad.rs` and `src/sim/world.rs` are all
  touched by PR #444.** The `world.rs` change is additive and small —
  `ColonyBooks` gains `raided_from` / `lost_to`, filled at `book_raid` — but
  that is the most contested file in the repo, so land against it rather than
  around it.
- **`Body::Choice` now carries a per-row tint** (default `GOOD`, so every
  other page draws as before). Anything adding a `Choice` elsewhere gets the
  old look for free.
- **`examples/foodroad` gained `ground=0|1` and `whole=1`**, and its `warm`
  default moved 1,500 → 6,000. A cost number taken with the old default
  priced every setting of `roadhalf` on the same young map.
- **`examples/labui frames=20000` panics** with *"the interface has no button
  for RosterCompare"*; `frames=6000` does not. It reproduces on `origin/main`'s
  `src/lab/ui.rs`, so it predates this branch — a harness fragility rather
  than engine behaviour, and not filed as a bug, but it will waste somebody's
  twenty minutes.
- **Still not answerable, and wanted:** which *plants* a harvested flower came
  off. Nothing attributes a harvested cell to the organism it grew on — the
  bite reads a material, not an individual. Whoever owns the plant line next
  is the one who could change that.
