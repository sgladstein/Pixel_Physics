# Re-derivation sweep, 2026-08-28 — IN FLIGHT

`scripts/rederivation-sweep.sh` is running detached and writing here.

15 arms x 8 world seeds, competitive bed (24 trees / 512 columns), 28,800
frames per run — 8 whole day/night periods, so the day phase is pinned.

**Constants swept**, each around its shipped value, one per arm, with a
rebuild per arm because these are Rust `const`s and `.ron` is `include_str!`'d
(identical output across settings is the tell that a knob was never
connected):

| constant | arms |
|---|---|
| `LEAF_CONSTRUCTION_MULTIPLE` | 0.6, 0.9, 1.5, 2.0 |
| `WOOD_CONSTRUCTION_MULTIPLE` | 0.4, 1.2, 1.6 |
| `MAINTENANCE_PER_NODE` | 1.0e-5, 4.0e-5 |
| `TRANSPIRATION_PER_RATE` | 0.05, 0.2 |
| `reproductive_allocation` (tree.ron) | 0.05, 0.20, 0.30 |

plus `BASELINE` at the shipped values.

**Read the binding counters and the who-won Gini, not stand totals.** A
stand's output is pinned by world width, so a lever can reorder the stand
completely while moving no total — which is why the sweep exists in this
form.

**Progress:** `cat PROGRESS` — one line per completed arm.

**When it finishes** (`PROGRESS` reads `SWEEP COMPLETE`): `git add -f` the
`*_s*.log` files and `PROGRESS`, delete `.gitignore`, and commit. The logs
are the result. The `.plant.rs.orig` / `.tree.ron.orig` files are the
script's restore-from copies and must never be committed.

**The trap restores `src/sim/plant.rs` and `assets/species/tree.ron` on any
exit, including a kill** — so an interrupted sweep leaves the tree clean, not
holding a swept constant.
