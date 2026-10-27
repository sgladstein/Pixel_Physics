---
name: soil-physics-and-roof-sense
description: Soil realism report + four off-by-default soil switches (PR 751, 2026-10-21): roof span, touch roof sense, dry root cost, seed emergence depth
metadata:
  type: project
  modified: 2026-10-21T08:15:22.478Z
---

Report: Reports/soil-physics-realism-2026-10-21.md (PR #751, branch claude/project-thread-3o3u0y). §10 lists the switches.

- The ant's roof forecast `f::ROOF_DROP` (cut_drops_roof) matched a present-tense touch bit on all 116,388 of s171's Dig options over 48 soil-arena worlds (0 disagreements). Existing 44 features alone: AUC 0.994.
- Switches built overnight 2026-10-21 at the owner's request ("switches off by default"), all env vars read once:
  - PIXEL_PHYSICS_ROOF_SPAN (on=12): wetness-dependent span limit, roofs sag into a dome (re-lining stops cave-in chimneys). Kept off by the owner's ruling (see below).
  - The touch roof sense shipped on main as the default (bite_is_hanging, be3bf9a2); my duplicate switch was dropped at merge.
  - PIXEL_PHYSICS_DRY_ROOT_COST (on=2): root cost up to 2x in dry soil. The lab's root front is mostly wet (mean bill 1.07-1.16x under light rain, ~1.3x after 60k frames with no mister); no clear stand change.
  - PIXEL_PHYSICS_EMERGENCE_DEPTH (on=6 cells, scaled by cbrt(endowment/0.2)): only 1-14 seeds per lab box are buried 7+ deep, so the stand doesn't move.
- SETTLED 2026-10-21: the owner wants ONE physics for every world (not per-world rules). ROOF_SPAN stays off. The "glass ant farm" reading was wrong: in real farms long horizontal tunnels stand, because a roof spans its shorter horizontal size (the gap between the panes), so a tunnel's real span is about a body in any reading. A rule keyed on on-screen width can't tell long tunnels from halls. The worthwhile addition is wetness-driven shedding (soaked walls shed grains, dry rims crumble), which is not built yet.
- Gaps still open: plant_available_fraction is texture-blind; nutrient is position-keyed (spoil can't fertilise); §Z39 unfiled.

**Why:** next sessions on soil, ant senses or plant soil should start from these numbers rather than re-measure.
**How to apply:** check PR 751's state before citing; swapping ROOF_DROP's default is the brain lane's call.
