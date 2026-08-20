---
name: review
description: Show the owner a rendered artifact and get a judgement back — images, before/after pairs, blind A/B comparisons, or frame sequences — via the shared review queue. Use when a change is judge-by-eye ("does this look right", "which of these reads as real breakage", "is this satisfying in motion"), when you need a verdict that tests cannot give, or when you are about to claim something looks better. Also use to retrieve verdicts on cards you posted earlier, possibly in a previous session.
---

# The review queue

This repo's method says *look before you measure* and *verify live before
declaring done*, and its ethos says the thing has to **feel satisfying** — none
of which a test can answer. The review queue is the channel for asking the
owner directly: you post a rendered artifact with a written question, they
judge it in a local web page, and you collect the verdict later.

It is shared by every worktree of the clone, so a card you post from
`.claude/worktrees/foo` is in the same queue as one from the main checkout.

## Posting

```
python3 scripts/review.py post  --board fracture \
    --title "Debris size after a hammer strike" \
    --question "Does the rubble read as a distribution, or all one size?" \
    --image target/filmstrip.png
```

From a worktree whose branch predates this tooling, `scripts/review.py` will not
exist. The queue keeps its own copy — this invocation always works:

```
python3 "$(git rev-parse --path-format=absolute --git-common-dir)/pixel-physics-review/bin/review.py" post ...
```

`post` prints the card id and a URL. Hand the owner the URL in chat.

### A/B and before/after

```
python3 scripts/review.py ab --board fracture --blind \
    --title "Graded fragments vs uniform blocks" \
    --question "Which collapse looks like real breakage?" \
    --a before.png --b after.png --a-label "current" --b-label "graded"
```

`--blind` shuffles the panes and hides the labels until the owner has chosen.
Use it whenever you have a stake in the answer — which is most of the time.
The stored verdict records the real label, so blinding costs you nothing.

Pass several files to `--a`/`--b` and they become **frame sequences** sharing one
scrubber. For anything about motion this beats a GIF, which cannot be paused on
the frame that looked wrong.

### The general form

Anything with captions, per-item counters, or multi-line context goes through a
JSON spec on stdin — it avoids the shell-quoting mess of expressing all of that
as flags:

```
python3 scripts/review.py post --json - <<'EOF'
{
  "board": "fracture",
  "kind": "frames",
  "title": "Falling column: straight drop vs lateral spread",
  "question": "Which fall reads as sand rather than a solid bar sliding down?",
  "context_md": "Same 90 grains, same seed.\n\n- **A** is current behaviour\n- **B** adds lateral jitter",
  "blind": true,
  "items": [
    {"label": "A — current", "files": ["a_00.png", "a_01.png"], "caption": "no lateral term",
     "meta": {"bodies spawned": 0, "cells removed": 894, "worst frame ms": 4.1}},
    {"label": "B — proposed", "files": ["b_00.png", "b_01.png"], "caption": "jitter x1.8",
     "meta": {"bodies spawned": 41, "cells removed": 902, "worst frame ms": 4.4}}
  ]
}
EOF
```

## House rules for a card

- **Put the counter in `meta`.** If the change adds a discrete "this happened"
  event, the count belongs on the card, because the page renders it directly
  under the image. This repo has already read a collapse as "chunks are
  working" from a picture while the body count was **zero for the whole run** —
  two very different mechanisms look identical at the zoom an image is judged
  at. An image says *what* and *where*; only the number says *whether it fired*.
- **Ask one answerable question.** "Thoughts?" wastes a round trip. "Does the
  rubble read as a distribution, or all one size?" gets an answer.
- **Say what you already measured**, in `context_md`, and what you want ignored.
  The owner's playtest reports have overturned three models that looked correct
  in tests — give them the reading you are unsure of, not a summary of your
  confidence.
- **Post the artifact you actually judged by.** Not a re-render with different
  settings, and not a crop that excludes the part you were unsure about.
- **Prefer a paired comparison.** Outcomes here have enormous spread, so a
  single run against a remembered impression is a sample from a wide
  distribution. Two runs side by side cancel everything the question is not
  about.

## Getting the verdict back

Posting is **fire-and-forget**. Carry on with other work; the answer keeps.

```
python3 scripts/review.py inbox --mark-seen   # answers to my cards I have not read
python3 scripts/review.py get <id>            # one card and its response
python3 scripts/review.py list --status open  # what is still waiting on the owner
```

Run `inbox` when you pick a thread back up, including at the start of a later
session — that is where an answer from yesterday is waiting.

Set `PIXEL_PHYSICS_REVIEW_AGENT` to a stable name if you want `--mine` to track
you specifically; otherwise it matches on branch and worktree.

## Blocking — only when it really blocks

```
python3 scripts/review.py post --json - --wait --timeout 1800
python3 scripts/review.py wait <id> --timeout 1800
```

The test is: **would I otherwise have to guess, and would a wrong guess waste
work I am about to do?** Not "would the answer be nice to have". Waiting on a
question you could park is a session sitting idle.

A wait that times out exits non-zero and changes nothing — the card is still
queued, still answerable, still in your `inbox`. So blocking is never a risk to
the question itself, only to your own time.

Cards posted with `--wait` are marked in the page and sorted to the top, so the
owner can see that a session is parked on them.

## Serving the page

The owner runs this once, from any worktree:

```
python3 scripts/review.py serve --open
```

One server covers every worktree. Do not start one on the owner's behalf unless
they ask; posting works whether or not it is running.

## Checking the tooling itself

```
python3 scripts/review_selftest.py
```

Covers what can actually break: concurrent posts from several worktrees, a post
killed mid-write, media outliving its worktree, a blind verdict resolving to the
real label, and a `--wait` timeout leaving the card intact. The visual half —
pixelated upscale, the scrubber, the blind reveal — has to be judged by eye in
the page.
