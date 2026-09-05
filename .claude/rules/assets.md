---
paths:
  - "assets/**"
---

# Gotchas for the asset files

Here rather than in `CLAUDE.md` because it only bites once you are editing an
asset, and an edit is preceded by a read.

- **Editing an asset `.ron` does nothing until the next build.** Materials
  and species are compiled into the binary via `include_str!`; only the
  app's F5 reload reads the directory, and headless harnesses do not. A
  sweep that edits `tree.ron` and re-runs a prebuilt example produces
  bit-identical "runs" — three of them, once, before anyone noticed the
  knob was not connected. Identical output across settings is the tell;
  rebuild between sweep points.
