---
name: sense-caps-audit
description: Sense-cap audit thread (2026-10-24): which ant-mind senses are capped/binary; three opt-in switches in PR #857
metadata:
  type: project
  modified: 2026-10-24T14:38:16.979Z
---

Thread "Audit ant senses for caps" (cmsg_01Mei3dwkcdzPxa5Jmuqi2bBQTV37t9durr8sGohGgkHo5), owner's ask 14:05Z 2026-10-24 after the PP_HOMEVEC 40:30 ramp cap. Report Reports/brain-training/teacher-training/sense-caps-audit-2026-10-24.md; data/scripts /mnt/project-files/sense-caps/. PR #857 MERGED to main 2026-10-24 (c9c72dab): report + Reports/README.md index line + code are all in the repo. Findings sent to Teacher lane.

- PP_DIGGATE_DOOR=1: dig head "when" gate was hard-coded to column 128 (bed door); lab colonies at 256 never got it. Bit-identical on bed. DEFAULT ON since 53b25169 (Teacher lane's call), =0 old gate.
- PP_SCENT_SOFTCAP=1: scent A pinned at u16 max for 30-33% of bed decisions near home (41-43% of walk options flat). Soft deposit -> 0%. Needs a trained arm (untrained food -24%/+18%); Teacher lane arms it after farfood3 (~17:30Z 2026-10-24).
- PP_HUNGRY_GRADED=1: nestmate hunger feature 33 was one bit, never fired on the bed.
- Not caps but exposure gaps: bed energy 7-60 tanks vs lab 0-3; warmth ±0.003 bed vs ±0.67 lab.
- Far sense (slot 9 / f40) is the Teacher lane's (log version). See [[far-home-sense-lane]], [[far-food-1024]].

**Reusable lessons for any sense work:**
- Measure a sense's range on the bed AND in a lab box before trusting it: PP_MIND_LOG + /mnt/project-files/sense-caps/hist.py (per-slot p1..p99, zero share, share at max, distinct values) and sat.py; sample.py through a fifo keeps bed logs small (raw bed log is ~100 MB per 1k frames).
- Grep for hard-coded bed coordinates (128 = bed door) in default-on mind code: the lab lands colonies elsewhere (256; 4 columns in the 1024 box).
- A clamp in storage (u16 scent) cannot be fixed by a graded reading; fix the write.
