---
name: starve-trace-lane
description: Why hA1 ants still starve after selection (traced 2026-10-22): roamers with no homeward pull die at the box walls; pellet jam at home
metadata:
  type: project
  modified: 2026-10-22T14:01:48.527Z
---

Thread "Why selected ants starve" (2026-10-22, ~14:00Z). Write-up, picture, scripts and patch: /mnt/project-files/starve-trace/. Sent to [[teacher-lane]].

- Setup: compute branch 98ad9dc4, pack on, sel off, 16 colonies each: hA1 82 starved, shortr-1022 round-6 ants 65 starved, old ant 1, of 384 each. Selected genomes are in the bucket (runs/shortr-1022/jobs/hA1r200_K.tgz, r5/final); the SA key env GCP_SA_KEY_B64 plus google-auth reaches it, while gsutil is anonymous and fails.
- **Cause 1 (50% hA1, 62% selected): roamers.** They leave the food band fat (p50 35-48 tanks), walk to the side walls (about ±124 cols), sit there about 45k frames, and starve. Their forage_anchor is right. Past 60 cols, empty ants have zero homeward bias at every tank level; only food in the crop pulls home, and only inside 40 cols. Old ant: 99% of trips return; new ant 72-74%. Selection does not fix this.
- **Cause 2 (33-35%): pellet jam at home.** Jaws full, so no eat or pickup; Drop offered 2-12% and almost never taken; the store is never empty. This includes selected founders that dig from 3 tanks and die by 6k frames.
- For the jam, PP_NEEDDROP is NOT the answer: the foraging lane tested it on hA1 at 32 seeds and saw no change (4.0 vs 4.3 died). The jam fix is a body change, on the owner's card via the Teacher lane.
- PP_DLOG decision log patch: dlog-trace-anchor-vs-98ad9dc4.patch. Runs are about 35 s each on this box, so 48 runs take about 8 minutes on 4 cores.

**Why:** a later lane should not re-trace this. **How to apply:** aim the next founder lesson at homing when empty and far; judge it by deaths past 60 cols.

**Come-home lesson, attempt 1 (14:00-14:55Z), files /mnt/project-files/starve-trace/come-home/:** rule PP_HOMEFAR=R (9d19121c + shadow-rim patch). Teacher acting at R=45, 16 seeds: deaths past 60 cols 2.56 -> 0.81, home food 251 -> 289. DAgger copies FAILED at 32 seeds (base 4.3 starved, 2 collapsed): hm1 (R=45) 20.0 starved, 32/32 collapsed (empty ants stay home); hm2 (R=60 + noneL food-band anchor) 8.6, 11 collapsed. Likely cause: HOME_DIST squash is too flat to separate far from the food band; next step is a sharp "far from home" sense, then a re-copy. Do not hand over hm1/hm2.

**Attempt 2 (15:05-15:46Z):** PP_FAR_CTX=60 (far bit on ctx slot 9, d_alarm). hA1 has 16 live links on that slot, so turning the bit on alone hurt (8.7 vs 5.1 starved); hA1z (those links zeroed) + bit = hA1 (3.75 vs 4.31 at 32 seeds). Copy hm3: good fit, but 6.81 starved, 5 collapsed, roamer deaths unchanged (2.66). Owner chose "neither yet" (15:40Z). **Everything is in the repo: PR #828** (Reports/brain-training/teacher-training/starve-trace-2026-10-22.md plus folder), handed to the Merge desk.
