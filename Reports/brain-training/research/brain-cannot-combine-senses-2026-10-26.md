# The ant brain cannot combine two senses: the issue, the evidence, and a proposed fix

*Teacher lane, 2026-10-26. Written at the owner's request (2026-10-26 16:10Z), before an adversarial review in a
separate thread. This is a claim to be tested, not a decision. **Status: proposed; under review.***

## 1. What the owner asked, and the short answer

The owner noticed that this lane's recent "senses" (`PP_HOMELOW`, `PP_SOILWAY`) are not senses but behaviours dressed
as inputs. They asked why, and then: "if our brain is bad at combining two senses, we have a fundamental issue in how
our brain is designed".

The short answer is yes:

- The trained ant's step scorer is **additive in its context**. The ant's own state (hunger, load, depth) is the same
  number for every step it could take, so it can change *which* step wins only through the units' saturation. The
  measurements below say that path is effectively shut.
- So a habit of the form "when X, prefer steps that do Y" cannot be grown by selection or written by fitting. The
  lane worked around this by pre-multiplying X into Y and handing the product to the brain as one input. That is an
  instinct, not a learned habit.
- The proposed fix is a **context gate on each scorer unit**: a learned function of the ant's state that multiplies
  how strongly the unit listens to the step's features. The gate is identity at zero weights, so every saved genome
  runs unchanged.

## 2. The architecture, as the code computes it

`src/sim/creature/mind.rs` `decide` (layout 5; `GENOME_LEN` 2,797 plus riders). Options are steps or acts; each has 45
features (`options::f`). Context is 22 inputs (`mind::c`) plus 12 memory units.

For scorer unit `u` (16 units) and option `o` of verb `v`:

```
pre_u      = b_u + W_ctx[u] . ctx + W_mem[u] . mem          (once per decision)
z_u(o)     = pre_u + V[u][v] + W_feat[u] . feats(o)
raw(o)     = c + sum_u out_u * squash(z_u(o))               squash(x) = x / (1 + |x|)
score(o)   = squash(sharp * raw(o))                         sharp = clamp(1 + g[SHARP], 0.25, PP_SHARP_MAX=4)
```

The verb is picked by a **job head** (one row per verb: context, memory, the verb's best option score, a bias, and
8 hidden units). The target is then drawn among that verb's options, weighted `(0.05 + (score + 1) / 2)^2`.

Three properties follow.

1. **Within one decision, context cannot re-rank options except through the slope of `squash`.** Every option sees the
   same `pre_u`. The difference between two steps is `sum_u out_u * [squash(z_u(a)) - squash(z_u(b))]`, and the
   context's only effect on it is *where on the squash* the unit sits. "Hungry, so prefer the step toward home" needs a
   unit that is flat (slope ~0) when fed and steep when hungry. That gate is made of a saturation edge. It needs
   `W_ctx`, `b_u`, `W_feat` and `out_u` tuned together, and then the outer `sharp` squash must not flatten it again.
2. **The hidden units, the brain's only place to build an AND, feed only the job head** (`HID_OUT`, one row per verb).
   They can make "dig now" depend on two things, but not "dig *this* cell" or "step *that* way".
3. **A link at weight 0 rarely comes alive.** Point mutation touches only live links (`examples/inrun.rs`
   `neutral_mutate`). Structural events add or remove one link each: 4 events per child, half of them adds, drawn among
   ~2,500 dead slots. So any one particular link is born in about 1 child in 1,250, with a random weight, and it is
   billed for as soon as it exists (`PIXEL_PHYSICS_LINK_PRICE`). *Correction to the chat on 2026-10-26: I said they
   "never" come alive. Rarely is right.*

## 3. Evidence

| what | result | where |
|---|---|---|
| **Saturation, measured** (colony 13 + home-low, world 13007, 1.4M unit-by-Walk-option sums) | median unit slope **0.023**; 75% of sums past \|z\| 2.2 (slope < 0.1), 35% past \|z\| 9; final Walk scores past ±0.95 in **76%** | `scratchpad ns/sat.py` (method below) |
| **Does hunger open a gate?** same sums split by the ant's energy input | median unit slope **0.023 fed vs 0.022 hungry**: hunger moves no unit off its flat | same |
| Copying the come-home teacher into the existing weights (5 fits) | agreement with the teacher 0.28 → 0.40-0.46 on the ant's own states; starvation flat or worse | colony-selection log 2026-10-26; `dead-ends.md` (homefit entry) |
| ...at sharpness 2x and 1x | same plateau (0.46) | same |
| The same lesson onto a *pre-combined* input (home-low) | 16 weights fit to **0.79** at KL 0.0006; starvation **~5x lower** over 16 worlds | `assets/minds/homelow_pool.txt` |
| Selection alone, ~10 nest batches since 2026-10-22 | never found "go home when low"; nest batch 10 kept the habit once given, never grew it | colony-selection log |
| Dig job, underground with a cut in reach | Dig picked 2% of chances; the job's sum ≈ -6.3 | log 2026-10-26 |
| "Way out" for pellet carriers, pre-combined | open nest 378 → 644 with **no fit at all** (it reused home-low's weights) | log 2026-10-26 |

**Method for the first two rows:** each unit's `z` is recomputed from member 0's genome, using the logged context,
memory and per-option features (`PP_COPY_LOG`, `PP_COPY_SAMPLE=50:50`, every 4th logged decision, 80k decisions).
"Hungry" is energy input < 0.35 (2,352 samples) and "fed" is ≥ 0.5. *Caveat:* member 0 stands in for the colony,
whose other members are near-clones of it.

The pattern: whenever the needed behaviour was "context X decides how much feature Y matters", the existing brain
failed both to evolve it and to be fitted to it. When a hand-built input carried X×Y, a handful of weights learned it
at once. That is what an architecture without a multiplicative path predicts.

## 4. What this lane did about it, honestly named

`PP_HOMELOW` and `PP_SOILWAY` are **instincts**: built-in "when X, head for Y" drives. The brain can weight them but
cannot invent or reshape them. They work, but they sit close to the old-ant body rules the owner did not want made
permanent (owner ruling, 2026-10-23). Both remain off by default on `main`. `PP_SOILWAY` is scratch.

## 5. Proposed fix: context-gated scorer units

Give each scorer unit a gate:

```
gate_u   = 1 + tanh(A[u] . ctx + a_mem[u] . mem + a0_u)      (identity when A, a_mem, a0 = 0)
z_u(o)   = pre_u + V[u][v] + gate_u * (W_feat[u] . feats(o))
```

- **Backward compatible:** with the new weights at 0, `gate = 1` and every saved genome scores exactly as now. Old
  pools read the new slots as 0, as layout 5 did for `f::FOOD_WAY`.
- **What it buys:** "hungry, so follow home" becomes one gate weight on energy times the unit's existing weight on
  `f::HOME`. "Carrying soil, so follow the way out" becomes a gate on `c::SPOIL` times a plain "way to the door through
  the tunnels" feature, which is a genuine sense, always on. One weight is something selection can find, and it is a
  smooth target for a fit.
- **Cost:** 16 × 35 = 560 slots, and one multiply per unit per option. The gate is per decision, so it is computed
  once, like `pre_u`. The per-option work stays one dot product per unit.
- **Alternatives the review should weigh:**
  - (a) Let the hidden units feed the scorer. This is more general, but it adds a new path per option.
  - (b) Lower the sharpness. Tried at 2x and 1x: the copy still plateaued and selection got worse.
  - (c) Better search (more structural adds, a learning rule). The search is not the whole story, though: a direct
    gradient fit on the existing weights also plateaued.

### Test plan (each step must pass before the next)

1. **Can it represent the habit?** Fit the come-home teacher's labels on the gated brain from **plain senses only**
   (energy, `f::HOME`, the log distance home), with no home-low input.
   - Pass: agreement ≥ 0.75 on held-out states, and KL to the reference no worse than hf8's.
   - Positive control: the same fit on the ungated brain must stay at ~0.45. If it does not, the gate is not what made
     the difference.
2. **Does it act on it?** Run 16 worlds with the teacher off, against `homelow_pool` + the instinct.
   - Pass: starvation within the instinct's spread, food home no lower.
3. **Can selection find it?** One colony-selection batch from colony 13 with the gates at 0, on far moving food, plus
   a matched ungated control.
   - Pass: the come-home habit appears (starvation falling across rounds), measured by tracing individuals, not only
     by the garden medians.
4. **Retire the instincts.** Turn `PP_HOMELOW` and `PP_SOILWAY` off, and make the plain way-to-the-door feature
   standard.

## 6. Questions for the adversarial review

1. Why was the brain built option-scoring with an additive scorer and hidden units on the job head only? Was a gated
   or multiplicative path considered and rejected? (Start from `Reports/brain-training/README.md`, the design of record
   rev 3, and `ladder/dig-rung-design.md`. `mind.rs` cites a `review-brain-architecture.md` (E5-E8, F9) that is not in
   the repo or the shared folder's top level; finding it is part of the review.)
2. Is the saturation number a property of the architecture, or of how this pool was bred? The forager seed set the
   sharpness gene to 3, and every genome since sits at the 4x ceiling. Would a fresh, unsaturated brain combine senses
   fine?
3. Does anything shipped already depend on the context reaching the scorer only additively (the recall tilt, the dig
   head, the lay gate, the guest seat)?
4. Is the real bottleneck the search (mutation-only, ~32 colonies), with the architecture a red herring? Step 1's
   positive control is meant to answer this.
5. Frame cost at lab scale. Riders past `GENOME_LEN` (dig head, lay gate) and every tool that hardcodes the layout.

Data: scratchpad of the Teacher lane session. Copy log `ns/new13007.log`; scripts `ns/sat.py`, `ns/where.py`,
`ns/digjob.py`. The numbers here are reproducible from `assets/minds/homelow_pool.txt` and the `PP_COPY_LOG` recipe
in §3.
