# Where H3's tunnel colonies lose ants (chambers trace, 2026-10-29)

Readers for inrun runs made with `trace=250 PP_HUNGRYPROBE=1` (and `PP_MIND_LOG` for the deep step), written for the
H3-vs-std_pool question. `examples/inrun.rs` on this branch adds `ax,ay,verb` (home anchor, last verb) to trace rows:
read only, the run is unchanged (world 68 round 3 reproduced 75 vs 7 ants with it).

- `run1.sh`, `queue4.sh`, `deep2.sh`: round 0 sel=off; poolrounds.sh 4 x 200k sel=score; round 3 with the mind log.
- `ledger.py`, `summ.py`: the colony ledger per 25k window; one line per run (deaths by cause and place).
- `wall.py`: every ant that reached an end wall, by outcome. `under.py`: every ant that starved underground.
- `undermind.py`, `hungryloop.py`, `eatrate.py`, `neveroffered.py`: decision-level reads of starvers / hungry ants.
- `pocket.py` (`--mapf`), `shutnow.py`, `sealwho.py`: was the starver shut in; the standing share shut in (the
  specificity control: std_pool is 3-12% too); what closed the pocket.
- `overlay.py`: paired nest pictures with every underground starver ringed.
