---
name: training-efficiency-audit
description: Audit of 52 training batches (2026-10-24): gardens 30% of CPU, 7% exact reruns, controls needed but cacheable, colonies/rounds needed; how to read the bucket
metadata:
  type: project
  modified: 2026-10-24T14:35:19.700Z
---

Thread "Review training compute efficiency" (cmsg_01Mei3dwkcdzPxa5Jmuqi2bBDuE22iVpNnwXs8BCg1ChdU), asked by Scotttt 2026-10-24 14:15Z. Analysis only. Page: https://claude.ai/artifact/FXPApab9z3BFE36NJ6Wvwn . Scripts + reduced tables: /mnt/project-files/training-efficiency/ (fetch.py streams bucket small/*.tgz; needs `pip install google-auth google-cloud-compute`, creds from GCP_SA_KEY_B64 via gcp_train_job.gcs()).

- Colony-selection CPU (28 batches, 1,114 CPU-h): ideas 56%, common gardens 30% (CG_EVERY=2 -> 24 gardens per 48 rounds), fresh controls 8%, exact reruns 5.5%. All batches: 7.1% of colony-rounds were byte-identical reruns (determinism), e.g. cs23ctl==cs21shape, bp1pile==brood1ctl, cs9hcs==cs8hcs, kid3==st0==c1 ctl, hA1 x4.
- Controls: judging an idea vs its own first garden matched the real call on only 9/18 decision measures (3 backwards: inherit retry, crutch crater, bp1 starved) because controls improve under selection too. But of 31 same-batch controls: 6 reruns, 13 large effects already settled, 7 n.s., 5 changed/protected the call.
- Colonies: 32 keep 99% of large-effect (>1 sd) and 85% of medium calls at 64; no subsample ever reversed a call. Arm-to-arm colony correlation ~0 after 48 rounds (pairing buys nothing; historical controls are as good). Caveat: colonies are also the selection population.
- Rounds: rule changes visible at first garden (r1); learned changes settle r23-31; erosion needs 48. At r15 ~58% of r47 calls match, 8% opposite.
- Recommended: CG every 8 rounds (~22%), control cache keyed by build+founder+args+seeds and resume shared prefixes (~6-8%), one control per ratchet turn for several ideas, 32x16 screen then full 64x48, name the deciding metric first; gardens drop extinct colonies (count as 0).

**Why:** later lanes asking "how many colonies / do we need a control" should start here. **How to apply:** see [[teacher-lane]], [[ratchet-founder]], [[inrun-cloud]].
- 2026-10-24 15:00Z: Teacher lane ADOPTED all six as its batch-design rules (farfood3 already queued keeps CG_EVERY=2). Repo guidance: PR #859 (Reports/how-we-test.md §7 + report), handed to merge desk.
