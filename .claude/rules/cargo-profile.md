---
paths:
  - "Cargo.toml"
---

# Gotchas for the build profile

Here rather than in `CLAUDE.md` because it only matters when `Cargo.toml`
itself is being changed.

- **A `cargo` flag can be a performance change, and the obvious half may be
  the worthless half.** There was no `[profile.release]` in `Cargo.toml` at
  all until 2026-08-24 — every release build ran without LTO at
  `codegen-units = 16`. Adding it is worth ~4% of the frame, but the split
  is the lesson: `lto = "thin"` **alone measured no gain at all** (10.58 ms
  against a 9.84 ms baseline), and the entire win is `codegen-units = 1`,
  which is also the whole of the +50% build-time cost. Measure the settings
  separately before attributing a win to the one whose name sounds like it
  did the work.
