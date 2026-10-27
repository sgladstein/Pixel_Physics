# colony-mind seat: status

Thread: "Build the colony mind seat demo" (cmsg_01Mei3dwkcdzPxa5Jmuqi2bBEgF6GcJFQCizK4Vq4jzPFk)
Branch: claude/project-thread-6xokl9

Step (10:40Z): VERDICT = NO STANDALONE DEMO on today's trained ants. PR #905 MERGED 8329b37 (10:12Z). Owner card pending (recommended: fold in).
- step 1 review ../colony-mind/feasibility-review.md; step 0 ../colony-mind/step0-measurements.md (read its CORRECTION);
  plan ../colony-mind/plan-v1.md; plan review ../colony-mind/plan-review.md.
- Louder food trail (swell) and hush: noise on trip_deliveries (food actually home). My first +19-43% was MindCounts::delivered = re-handled crumbs.
- Alarm, warmth: no response. Close the stomach: works (sharing -95%, hunger spread x2, 16/16) but no survival payoff.
- Stake: demo_mind famine bed kills 14/16 colonies by 150k frames with or without any policy.
OFFER: my working parts for other seats. (a) Minds::closed_stomach (off by default, on my branch) for the keeper: it
gates "a place spreads by meals" (recall-head share_on_feed). (b) favour = a weighting on queen's who-lays (agreed).
(c) World::scale_pheromone (lower a scent cell). (d) examples/mindprobe.rs: paired verb probe vs an arm=none twin.

## COUNTER TRAP FOR EVERY SEAT: never score food with MindCounts::delivered or creature_stats.deliveries
Both book crumbs picked up in the nest and put back (2.4-5.8x income). Use creature_stats.trip_deliveries
(how-we-test.md §4). It turned my anchor verb from "+43% food" into noise.

Shared pieces I expect to need (will confirm after the review):
- sending never-foraged ants without starving them (with scout)
- a hook for choosing who lays (with queen)
- the common demo launcher (whoever creates shared/launcher.md first owns it)

Combinations: late stage of queen -> scout -> mind. Queen and scout threads: let's agree the arc's handoff early.

## Answer to queen on shared/who-lays.md (08:20Z)
The mind wants a **weighting over individuals**, not one pick: favour is a number per living adult (ant id) that
rises when it answers a swelled signal and decays over time. So please let the store hold (genome or ant id, weight)
entries and let `egg_genome` draw the egg's parent from that weighted list, with the layer's own genome as the
fallback when the list is empty or all weights are 0 (today's behaviour). Nice to have: a mix knob p (chance an egg
draws from the store rather than the layer), so favour can be graded rather than total. I will write weights each
few hundred ticks; I don't need per-egg callbacks. Launcher: fine by me, I'll use DemoSeat::Mind and demo_mind.ron.

## FINDING FOR EVERY SEAT (08:40Z): check claims against the TRAINED ant, not the hand-built one
The lab runs the trained mind by default since 2026-10-24 (src/bin/lab.rs:297-306). A mind ant returns into
mind::tick at creature.rs:7998-8022, BEFORE sensing, act(), chooser_step. So everything in act() and the hand-built
walk is dead in the default lab: the forage drive and the "met a returning forager" stamp (never read, never even
written by a mind delivery), RECRUIT, MEET_WAY, store rules, the nurse-scent walk (creature.rs:16162). A mind reads
only its ~45 option features + context senses (mind.rs:72-150; options.rs). Trained ants also do not mutate at birth
in the lab (lab/mod.rs:1026-1030), and the 30 pool members are near-copies (8-107 of 3,285 slots differ), so
"favour a line" re-weights near-twins: selection inside one lab session cannot move a gene. Pool weights (linear
read): alarm strength -19.6 (they step away, expect scatter not mobbing), corpse target -40.6, hungry larva beside
-26, warmth +91 (largest). Details: ../colony-mind/feasibility-review.md.
