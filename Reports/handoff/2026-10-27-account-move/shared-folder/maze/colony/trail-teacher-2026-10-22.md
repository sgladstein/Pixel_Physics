# Trail teacher session (maze thread, 2026-10-22)

Agreed with the Teacher lane: silencing the crowd trail is a weight edit (theirs to judge at 64 seeds); useful trail
following is tried as a teacher session (hand rule, DAgger copy onto hA1), judged here at 32 seeds before handing on.

Bed: Teacher lane in-run harness, compute branch ee38ae5b + trail-teacher-vs-ee38ae5b.patch (scratch), night2 settings,
sel=off, one 200k round, pack on, seeds 1-32, 24 ants (founder + 23 mutants). Home = deliveries within 8 cols of nest.
Paired sign counts (better/worse seeds) against hA1-ladenB unless stated.

| arm | what | alive | died | field bites | home | dig |
|---|---|---|---|---|---|---|
| hA1-ladenB | B laid only while carrying (emit crop*12-2) | 21.2 | 2.8 | 3,590 | 271 | 339 |
| hA1-noB | no B laid | 19.6 | 4.4 | 2,968 (8/24) | 374 (28/4, p 2e-5) | 338 |
| actT | rule 1 acting: empty, off nest, step up B | 16.8 | 7.2 | 2,535 (2/30) | 148 (4/28) | 282 |
| tA1 | rule 1 copied | 5.0 | 19.0 | 1,269 (0/32) | 92 (0/32) | 207 |
| tA1ctl | copy pipeline, lesson weight 0 | 22.1 | 1.9 | 3,563 | 241 (11/21, n.s.) | 312 |
| sup | ladenB + lay less where B is high (B-here -8) | 21.1 | 2.9 | 3,545 | 283 | 341 |
| supOut | sup + rule 2 acting: stay on trail (B there >= half best), step outward | 21.5 | 2.5 | 4,303 (22/10) | 391 (28/4) | 342 |
| oA1 | rule 2 copied | 16.8 | 7.2 | 2,569 (4/28) | 239 | 324 |
| oA2 | rule 2 copied, anchor weight 12 | 18.7 | 5.3 | 2,996 (9/23) | 262 | 302 |

supOut vs hA1-noB: home 18/14 (n.s.), field 30/2 (p<1e-4), died fewer on 22 seeds vs 9 (p 0.03).

Traces (seeds 1-3/4, decision log):
- Rule 1: empty climbers pile onto the B peak 10-20 cols out, short of the food (food band 15-35); the peak runs away
  (65,535 saturated at +15). Far roaming falls, but food in view falls too.
- Rule 2 acting: empty walks past 60 cols 40% -> 21% (the roamers that starve at the walls), 10-30 cols 34% -> 60%.
- Rule 2 copies: fit 55-64% of the teacher's steps; the copy dithers (back-steps 59% of sideways steps vs 45%), and
  more ants die at home and far. Two copies failing the same way: the approach (copy into today's brain) is wrong for
  this rule, not the weights. Same symptom as the Teacher lane's come-home-when-far copies.

Files here: g-hA1-sup.txt, tA1/oA1/oA2.txt, copier_t.py (target_only: reshapes only which step, never the verb),
dlogT.sh/dlogO.sh, judge.sh, summ.py, deathtrace.py, emptytrace.py.
