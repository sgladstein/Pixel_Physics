---
name: write-gate
description: Brain layout 8 (2026-10-27, PR #942): per-memory-unit write gate, off by default (PP_WRITE=on); write-gated latch works, forgives damage, has a partial slope
metadata:
  type: project
---

Owner 2026-10-27 15:05Z chose "prototype it, off by default" after [[latch-landscape]]. PR #942 MERGED eeb7611 (16:26Z).

- mind::WRITE: 12 x W_IN 44 (context, memory, reach per verb, bias). z = clamp((1-rate)+y,0,1); m' = m + z(new-m). All-zero row = old blend exactly. GENOME_LEN 3609 -> 4137, LAYOUT 8, GENOME_LEN_V7. Trainable only under PP_WRITE=on (write_on). Pool member with riders = 4625.
- Old pools bit-identical (shipped-pool guard hash, lab box RESULT, layout-7 flip-flop arms same numbers). Frame cost none.
- Write-gated latch (unit 9: value 60 reach.PickUp - 10, rate gene 0.495, write 96 at_nest + 84 reach.PickUp - 30 energy - 84), 16 worlds: starved 40 (flip-flop 45.5), half row 49.5 n.s., worst knock-out 2 lost (flip-flop 13-14). One write weight (at_nest) beats no memory 13/15 p=0.007 (not past Bonferroni); +PickUp flat; full vs two-term 15/16 p=0.0005. Report Reports/brain-training/research/write-gate-2026-10-27.md (mkwrite.py).
- Next: Teacher lane asked (after #942 lands) for a selection batch PP_WRITE=on vs off from a no-habit pool; pass -> case for default-on.
- Teacher lane agreed 16:09Z: batch write1 (std_pool ratchet, PP_WRITE=on vs off, same seeds, clean1's winning setup) after #942 lands, queued after elite2/clean1/env1 on the VM or on a second machine. It owns that result.
