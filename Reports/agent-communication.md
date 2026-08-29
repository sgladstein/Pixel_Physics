# How sessions talk to the owner — the census, and the rule it produced

**Status: rule landed 2026-08-29 (`CLAUDE.md` §"Writing to the owner"),
enforced by `scripts/plaincheck.py`.** Commissioned by the owner, who runs
several sessions at once and reported that their messages "are often not
helpful… sometimes I don't actually know what they're doing because they're
being too specific and not giving me any bigger picture."

This report is the evidence behind that rule and the worked examples the rule
is too short to carry. The rule itself is in `CLAUDE.md`; read that first if
you only want to know what to do.

## Contents

- [What was measured](#what-was-measured)
- [The four findings](#the-four-findings)
- [Why the review card is the one healthy channel](#why-the-review-card-is-the-one-healthy-channel)
- [The rule](#the-rule)
- [Worked rewrites](#worked-rewrites)
- [What was deliberately left alone](#what-was-deliberately-left-alone)
- [Caveats on the measurement](#caveats-on-the-measurement)

## What was measured

Four corpora, all of them things the owner has actually received:

| corpus | n | source |
|---|---|---|
| review cards | 158 | `origin/review-queue:cards/*.json` |
| card image panes | 347 | the `items` inside those cards |
| PR merge subjects | 34 | `git log --format=%s`, last 400 commits |
| non-merge commit subjects | 249 | same |
| one full PR body | 1 | PR #86, ~1,100 words, read whole |

Chat and end-of-turn replies — the channel the owner named first — leave no
artifact in the repo and could not be censused. The written corpora are used
as a proxy for register, and the proxy is sound in one direction: an agent
that writes `§S's second seam` in a PR title writes it in chat too.

Two detectors, run over each corpus:

- **register code** — `§S`, `WP-3`, `P2`, `W4`, `bug L`, `issue #12`: an
  internal identifier the owner cannot expand without opening a document.
- **code symbol** — `` `backticks` ``, `Type::method`, `fn()`, `file.rs`,
  `snake_case`.

## The four findings

**1. The subject line names the machine, not the world.** 56% of PR merge
subjects (19/34) carry a register code or a code symbol; 47% carry a register
code alone. `§S` is the subject of nine of the last thirty PR titles. Among
non-merge commit subjects it is 36%. The subject line is the entire message
for a reader scanning several sessions at once, and more than half of them
resolve to "open another document first".

**2. Nothing states a direction.** Not one message in any corpus opens by
saying what the session is *trying to make true*. PR #86 opens *"Follow-up to
#85, which closed two of §S's three verbs and diagnosed the third without
fixing it."* That is a position in a queue. The direction — *make broken rock
carry weight, so digging near a pile stops caving in ground that should hold*
— is never stated, in 1,100 words. **A position in a queue is not a
direction**, and this is the finding the owner ranked first.

**3. The world-visible consequence is missing or buried.** PR #86 again: the
sentence "rock sitting on a pile of rubble behaved as though it were sitting
on bedrock" appears nowhere. The one number a player would feel — the whole
frame going 31.2 → 24.5 ms near a strike — is row six of a seven-row table,
below `pending @1,599` and `wrong-cell bbox`. Every number in that body is
correct, measured, and named for the instrument that produced it rather than
for the thing it says about the world.

**4. The evidence under a picture is labelled in harness vocabulary.** 262 of
347 card panes (76%) carry `meta` and 261 of those carry a number — the house
rule about putting the count beside the image is being followed, and this
finding is *not* that it isn't. What is not followed is the naming: **18 of
158 cards (11%) carry at least one label that names the harness rather than
what it measures**, across 17 distinct labels. Real ones, printed directly
under an image the owner is being asked to judge by eye:

    luma MAD (deep rock crop)        chroma MAD
    crystal Material::glow           FIELD_SCALE (cells per light value)

`MAD` is mean absolute difference; nothing on the card says so. The other
fourteen are of the same two shapes — a constant's name (`FIRE_TINT_HIGH`,
`FIELD_SCALE`) or a genome field (`gut_bias`, `turgor_source`,
`lens_roughness`).

**11% is the corrected figure and the first draft of this report had 76%**,
having read "76% of panes carry `meta`" as "76% are badly labelled". They are
different questions and only one of them was measured; the rule this report
produced is the place that error would have been most expensive, since
`CLAUDE.md` is loaded before every session. Caught by running the checker over
the corpus rather than by re-reading the prose — which is the argument for
building the checker.

At 11% this is the smallest of the four findings by rate, and it is kept
because it is the cheapest to fix and sits inside the channel built
specifically for the owner's judgement.

## Why the review card is the one healthy channel

Card **titles carry a code-ish token 1% of the time (1/158) and questions
0%** — against 56% for PR subjects, written by the same agents in the same
sessions. Median title 10 words, median question 28.

The difference is not care. It is that `.claude/skills/review/SKILL.md` gives
the card a **required shape** — a title, one answerable question, the count in
`meta` — and refuses posts that violate parts of it. Every channel with a
shape is clean; every channel without one is not. The chat reply has no shape
defined anywhere in the repo, which is why it is the worst of them.

That is the whole basis for the remedy: **give the reply a shape, and make it
a command rather than a discipline.** The lane-note precedent is explicit that
prose alone does not survive a real session — `Reports/lanes/README.md`
records its own convention being followed 9% of the time until a number and a
checker were attached to it.

## The rule

Owner's choice, made 2026-08-29 from three rendered alternatives: **a short
brief, with the detail kept below a fold** — not two lines, and not the
current body with a summary bolted on top. Then trimmed twice on their
reading of the draft, and both trims are corrections worth recording:

- **"A player could see it" was too strong.** The first draft required every
  message to open with a visible consequence. Not every change has one — a
  measurement, a retune, a documentation fix, a perf win that is bit-identical
  on screen. The clause is now *what it does, in the vocabulary of the world
  or of the work rather than of the code*, and when nothing shows on screen,
  say what it is **for**.
- **"Every message" was too strong.** The first draft asked for four numbered
  parts every time, which turns a one-line update into four headings. The rule
  now **scales to the message**: short stays short, and only carries the
  obligation to be less technical; a long message carries the whole shape.
- **And it was too long.** 763 tokens in an always-loaded file the owner
  already considers over-long; now 408. The census, the worked rewrites and
  the reasoning live here instead, which is what a report is for.

What survives is two clauses and an ordering:

1. **What it does** — in the world's words or the work's, not the code's.
   `wiki/*.md` is the vocabulary: it describes every material and mechanic in
   plain language, with no code and no file names, and `CLAUDE.md`'s file
   table maps each source file to its page. Needing a word the wiki lacks is
   itself a finding — that page is stale.
2. **Where it sits** — the arc, and this step's place in it. The part the
   owner ranked first and the part no message had.
3. **Then the mechanism**, as technical as it needs to be.

The governing property: **the message must be abandonable at any line.**

**Numbers stay.** This project judges by measurement and the rule must not
become an excuse to stop quoting them. What changes is the label: a number in
the plain part is named for what it says about the world. `wrong cells 35,102
→ 1,337` becomes *ground that collapsed when it should have held: 35,102 cells
→ 1,337*. Same number.

**Chat is the subject of the rule, and mid-session narration is the worse half
of it.** The owner's complaint is partly about not knowing what a session is
doing *while* it does it. "Now checking `load::ground_footing_distance`
against the oracle at frame 1,599" is unreadable from outside; "checking
whether rock on a pile now reports the right support depth" is the same
sentence.

## Worked rewrites

**A PR subject.** Register codes are useful and stay — they just cannot be the
subject:

| before | after |
|---|---|
| `§S: I had the sign backwards, and it is not the framework` | `Blast damage spreads too far — I had the sign backwards (§S)` |
| `relax_region: ground is a last-resort root, matching tick (§S2)` | `Rubble stops counting as solid ground when nothing else holds (§S2)` |
| `Leaf litter rots mostly to nothing: Material::decay_yield, 0.05 for litter` | `Fallen leaves mostly rot away instead of piling up as soil` |
| `Thread both sky walks: whole frame 21.98 → 20.67 ms, bit-identical` | `Sky costs less per frame, 22.0 → 20.7 ms, with nothing on screen changed` |

**A card `meta` label:**

| before | after |
|---|---|
| `luma MAD (deep rock crop)` | `speckle vs before (0 = identical)` |
| `chroma MAD` | `colour drift vs before` |
| `crystal Material::glow` | `how brightly crystal glows (1 = normal)` |
| `FIELD_SCALE (cells per light value)` | `light resolution: 1 value per 8 cells` |

**An end-of-turn reply** — the channel with no conventions today, and the
rule's whole subject. This is a *long* message, so it carries the whole shape;
a one-line update would be the first sentence and nothing else. Rendered from
PR #86:

    Rock sitting on a pile of rubble no longer pretends it's sitting on
    bedrock — so hitting the ground near a pile stops caving in ground
    that should have held.

    Why it matters: this was the last of three places where the world
    mis-read what was holding something up. All three are closed now, so
    digging, blasting and hitting each leave damage where you'd expect it
    instead of spreading outward. Frames near a strike also got ~20%
    faster, because the engine had been doing work to spread the mistake.

    Where it sits: end of the "broken rock should carry load" arc. Nothing
    queued behind it. Natural next step is making the rubble itself look
    graded rather than uniform — say if you want that.

    Landed on main, CI green.

    --- mechanism, if you want it -------------------------------
    A cell resting on loose grain stored a support distance of 0, which in
    that field means "bedrock is adjacent"; 8 such cells produced 34,009
    wrong ones. load::ground_footing_distance now walks the grain column
    and returns the real path length. Wrong cells 35,102 -> 1,337.

Note what survived: every number, the mechanism, the file and function names.
Nothing was dumbed down. The 1,100-word body is still the right PR body — it
is just not the right *first* thing to read.

## What was deliberately left alone

The owner was asked which channels cost them and named two: **chat and
end-of-turn replies**, and **review cards** — then narrowed it unprompted to
**"mostly chat"**. So the weight is: chat is the rule's subject, the card
`meta` labels get a one-line fix in the review skill, and nothing else moves.
Explicitly not chosen:

- **PR titles and bodies.** The 56% figure is real and PR subjects are the
  worst-scoring corpus measured, but the owner reads PRs deliberately rather
  than scanning them, so the rule reaches them only through the general
  register rule — no separate convention, no check. The rewrites above are
  offered, not required.
- **Reports and lane notes.** Read on purpose, by someone who has chosen to
  page in the context; and lane notes are agent-to-agent, where the technical
  register is correct. Unchanged.

Recording this so a later session does not "finish the job" by extending the
rule to channels the owner kept.

## Caveats on the measurement

**The detectors undercount, and the shortfall is not random.** They find
symbols, not register. Four PR subjects that score *clean* —
`root a ground-supported cell at what the pile stands on`,
`price every tissue, delete the fences standing in for prices`,
`thread both sky walks, bit-identical`,
`reject the per-tile momentum gate` — carry no symbol and are still written
from inside the implementation. The true rate is above 56%, and the number
should be read as a floor. Applying this file's own standing rule — *ask what
your number counts when nothing is wrong* — the detector was checked against
titles known to be fine and passes titles that are not, so it has a
specificity problem, and it is quoted here only as a lower bound.

**One PR body was read whole, not a sample.** #86 was chosen because it is a
*good* piece of engineering writing — well organised, honest about its
instrument's limits, carrying a noise bar. The finding is not that it is
sloppy. It is that excellent technical writing is still the wrong first
artifact for a reader running five sessions, which is the harder case to
argue and the reason a weak example would not have carried it.

**Chat replies were never measured, and they are the rule's whole subject.**
The channel the owner ranked first — and then narrowed to, unprompted — is the
one with no corpus, so the rule that governs it rests on a proxy. If the rule fails, this is the likeliest reason, and
the cheap next measurement is to keep a handful of real end-of-turn replies
and score them the way the cards were scored.
