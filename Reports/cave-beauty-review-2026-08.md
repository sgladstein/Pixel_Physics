# Cave beauty review — what real caves have that ours don't yet

2026-08-20, reviewing session. The owner asked what makes beautiful cave
photographs beautiful, and how our merged round-3 caves measure against
that. Note on sources: the sandbox's network policy refused image
downloads (gateway 403), so the criteria are drawn from the canonical
photographic subjects — Carlsbad's Big Room, Luray's Dream Lake, Son
Doong's dolines, Waitomo, Naica — described from knowledge rather than
from freshly fetched photos. If reference photos get dropped into the
repo, re-run the comparison against exactly those.

Ours were judged from fresh merged-tree renders: `viewshot vault=1` on
rolling s7 (geode vug), canyon s1 and wetland s3 (full systems), zoomed
4x (`target/filmstrips/beauty_*_zoom.png`, plus
`merged_cave_glow_s1.png` from the merge review).

## Seven features of beautiful natural caves

1. **Vertical exuberance at every scale.** Speleothem *forests*: one
   monumental anchor formation, many mid-size, thousands of soda-straw
   fingers. Heavy-tailed size distribution, never even spacing.
2. **Pairs and near-misses tell time.** A stalactite reaching for its
   stalagmite across a tall room; fused columns. The gap is the drama.
3. **Rooms with necks.** Compression and release: tight dark passage,
   then the ceiling leaps and the floor falls away. The photograph is
   always taken at the release point.
4. **One light source, darkness preserved.** The best shots are mostly
   black; a single shaft or lamp lights part of the scene and the
   falloff gives depth. Full illumination kills it.
5. **Still water doubles everything.** A glass-flat pool mirroring the
   formations above it (Luray's Dream Lake).
6. **The rock has grain and flow.** Flowstone drapes, ribbed curtains,
   bedding lines; ceilings break along structure into stepped, blocky
   profiles; breakdown boulders below explain where the roof went.
7. **Color restraint with one accent.** Ochre/grey everywhere, so one
   patch of white calcite, crystal, or blue pool reads as treasure.

## Verdicts on ours

**Already strong:** (4) darkness with pooled crystal glow is our best
feature — the wetland pair of glowing stalagmites in a black chamber is
the postcard; (7) restraint-plus-accent holds by construction; (6) is
half-earned (systems stretch along the bedding; floors are rubble).
The rolling s7 water-filled vug — glowing pale ring around dark blue
water at a shaft's foot — is a legitimate small jewel.

**Gaps, in order of what they cost the picture:**

1. **No vertical exuberance (the big one).** ~125 speleothems per
   system, mostly small teeth at enforced ≥4-column spacing — reads as
   a comb, not a forest. No monumental anchor formation anywhere. Want:
   one huge fused column or floor-to-ceiling near-pair per system, plus
   5–10x more soda-straw-scale fringe, clustered (drip concentration)
   rather than spaced, with a heavy-tailed size draw.
2. **Chamber/passage contrast too small.** Canyon s1 has real
   pillar-divided rooms; wetland s3 is a root-web of near-equal-width
   strands — no compression/release. Chambers should be conspicuously
   taller than passages; some passages tighter.
3. **Water is inert.** Pools are correct but a thin blue line. No
   renderer reflections (don't chase them) — but formations standing
   *in* the pool (glowing crystal at the waterline, its light already
   spills across water) buys half the Luray effect.
4. **Pairs too short to be dramatic.** The near-meeting rule shipped
   but at 2–7 cells; the money formation is a rare tall pair, two-thirds
   of chamber height each side, one-cell gap.
5. **Ceilings are noise-smooth.** Worley curves with no structural
   grain; stepped/blocky ceiling breaks along the strata would also
   explain the breakdown mounds below.

## Suggested round-5 scope (not yet specced or approved)

All five gaps are realise-pass work on the existing anatomy — no new
mechanism. Formation exuberance + heavy-tailed sizes + clustering (1, 4),
chamber/passage contrast via a second threshold shaping pass or
per-junction chamber dilation (2), waterline formations (3), ceiling
grain along the strata (5). Owner has not yet approved this round;
judge against the criteria above by rendering, not by counters alone.
