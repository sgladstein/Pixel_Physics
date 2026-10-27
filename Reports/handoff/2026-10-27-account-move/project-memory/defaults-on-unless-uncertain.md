---
name: defaults-on-unless-uncertain
description: Owner rule 2026-10-24 17:35Z: new features ship default ON unless there is real uncertainty; replaces "off by default if it changes results"
metadata:
  type: feedback
  modified: 2026-10-24T17:36:07.918Z
---

Owner (Scotttt, 2026-10-24T17:35Z): "We keep developing features off by default. The default should be on unless there is uncertainty."

**Why:** features left off by default get forgotten or never exercised in training and the lab, so the work sits parked.
**How to apply:** lanes flip new switches to on in the PR unless the doc states a measured or concrete uncertainty. The merge desk asks a lane to flip (or state its uncertainty) when a PR adds a default-off switch with no reason given; it no longer applies "off if it changes results". Supersedes the older merge-desk guidance. Related: [[merge-desk-handoff]], STANDARD DEFAULTS in MEMORY.md.
