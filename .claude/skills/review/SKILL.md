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
scrubber, so both sides step to the same instant.

## Animation: GIF or frame sequence

Both are supported. They answer different questions, and picking the wrong one
wastes the round trip.

**GIF — when the question is *feel*.** Does this collapse read as destruction,
does this fall read as sand, is this satisfying. It plays on its own and loops;
the owner does nothing but look. This is the right default for "watch this".

```
python3 scripts/review.py post --title "..." --question "..." --gif fall.gif
```

**Frame sequence — when the question is *detail*.** Which frame is wrong; do A
and B differ at the same instant. Scrubbable, steppable with the arrow keys, and
synchronised side by side for an A/B. A GIF cannot be paused on the frame that
looked wrong; a sequence can.

A card may carry both — a GIF to judge the feel, frames to find the moment.
**Prefer a frame sequence to a GIF.** Tested head to head on one card, with
the same motion posted both ways: the sequence played and the GIF did not.
The GIF was valid by every check available on the posting side — 24 frames,
distinct payloads, 60 ms delays, a `NETSCAPE2.0` loop block, stored with its
extension and served as `image/gif`, inside a plain `<img>` the page does not
re-source — and it still showed as a single static frame for the owner. The
sequence uses the page's *own* timer instead of the browser's GIF decoding,
so it does not depend on any of that. Reach for `--gif` only if a sequence is
impractical.

**And render frames people can actually see.** The "never zoom a GIF" rule
above is about the page scaling *client-side*; it is not licence to post a
190x130 crop. One went out at that size and the owner reported seeing none of
the changes in it, because at that size there was nothing to see. The stills
he has been able to judge are 700-950 px across. Crop tight, then zoom so the
result is legible.

**One file per item, then** — and `post` refuses the alternative rather than
letting it through. `files` is the *frame sequence* field: several entries in
one item become a scrubbable strip, so a GIF and a still in the same item do
not render as an animation beside a picture, they become frame 0 of a
two-frame sequence and the motion is gone. Silently, and the card looks right
from the posting side, which is how it reached the owner once —
*"your card wasn't an animated gif, just two frames"*. Give an animation an
item of its own; `--gif` and `--image` do that for you.

**Autoplay earns its place at roughly ten frames or more.** A two-frame sequence
set playing is a strobe, not an animation: post those two states as an A/B and
let the owner compare them, or step the slider. Frames are for motion you need
to *scrub*; two instants are a comparison.

### Producing one, headlessly

`filmstrip` encodes a GIF with **no window and no GPU**, so this works from a
cloud session or over a plain shell:

```
cargo run --release --example filmstrip -- scene=fall gif=1 out=/tmp/fall.gif \
    start=100 every=6 count=20 zoom=1 crop=0,140,256,110
```

**`gif=1` is not optional when the output is named `.gif`.** Without it the
contact-sheet branch runs and `image::save_buffer` picks its encoder from the
extension, writing the whole sheet as a **single-frame GIF**: a valid file, an
animation's name, and no motion. Two cards shipped that way while the agents
reported posting animations. `filmstrip` now refuses the combination, and
`post --gif` refuses a one-frame GIF and tells you this is why.

`start` is the first frame sampled, `every` the interval between samples,
`count` how many. Frame delay follows `every` (`every*1000/60` ms, floored at
16). Drop `gif=1` and the same command writes a contact-sheet PNG instead.

**Never pass `zoom` to a GIF.** It multiplies the bytes and buys nothing: the
page scales client-side to exact integer multiples under
`image-rendering: pixelated`, and its zoom control reports the true factor. So
`zoom=2` ships 2.4x the data for a picture the owner could already have got by
pressing `z`. Crop instead — that removes pixels the question is not about.

Measured, `scene=fall`, `every=6`:

| settings | size |
|---|---|
| `count=40 zoom=2 crop=0,140,256,110` | 2353 KB |
| `count=40 zoom=1` (whole world) | 2080 KB |
| `count=40 zoom=1 crop=0,140,256,110` | 997 KB |
| `count=20 zoom=1 crop=0,140,256,110` | 558 KB |
| `count=20 zoom=1 crop=64,150,128,80` | 235 KB |

These ride in git on the `review-queue` branch that every clone fetches, so an
animation is not free the way a local file is. Twenty careless cards is 50 MB;
twenty cropped ones is 5 MB. Crop to the part the question is about, and prefer
`count=20` unless the motion genuinely needs longer.

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

## Framing: render wide, declare tight

**Render about twice the area you think you need, then declare the part you
actually want judged.** The page frames your declared region by default, and one
zoom step out reveals the margin you rendered around it.

This exists because the alternative costs a round trip. A crop that turns out too
tight cannot be recovered in the viewer — those pixels were never in the file —
so the owner has to ask for a re-render and the question waits. Margin captured
up front costs a few kilobytes and nothing else.

For a region of interest `x,y,w,h`, render the doubled box centred on it:

```
cargo run --release --example filmstrip -- scene=fall \
    crop=<x-w/2>,<y-h/2>,<2w>,<2h> out=/tmp/a.png
python3 scripts/review.py post --image /tmp/a.png --focus center …
```

`--focus center` is the middle half by area, which is exactly your region of
interest when you doubled it — so the usual case needs no arithmetic. Use
`--focus x,y,w,h` (in the *rendered image's* pixels, not world coordinates) when
the interesting part is off-centre. An out-of-bounds rect is rejected at post
time rather than reaching the owner as a blank pane.

The owner sees `Focus 6× · 128×80 of 256×160`, can press `z` to see the whole
frame, and can drag inside the margin without leaving focus mode.

Not a licence to render the whole world every time: doubling a crop roughly
quadruples the pixels, and these ride in git on `review-queue`. Double the
region of interest, not the world.

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

## Catching up a session that started before this existed

A session whose `CLAUDE.md` predates the queue needs two steps this file assumes
you already did — installing the tool and flushing anything queued locally.
Those live in `.claude/skills/review/CATCHUP.md`, which is also what to point
such a session at:

```
git fetch -q origin main && git show origin/main:.claude/skills/review/CATCHUP.md
```

Pointing at the file rather than pasting its contents keeps the instructions
current after the next change; a pasted copy went stale within a day.

## If you are not on the owner's machine

**Read this before your first card if you are a cloud or web session.**

The queue is a directory shared by every worktree of one clone. Your clone is in
your own container, on a disk the owner cannot read, so a card written there and
nothing else reaches nobody. This is not hypothetical: three cards were posted
that way and the owner's page said "Nothing queued here."

Cards therefore travel over the git remote, on an orphan branch `review-queue`:

```
python3 scripts/review.py sync     # exchange cards and verdicts with origin
```

`post` runs this for you, and `inbox` and `wait` run it before they read. You
normally do not have to call it. What you *do* have to do is **read what `post`
tells you**:

```json
"owner_can_see_it": true,
"sync": { "ok": true, "branch": "review-queue", "pushed": 2 }
```

If that says `false`, the card is on your local disk only and the owner will
never see it. The output carries the reason and a `warning`. Fix the cause or
re-run `sync`; do not report the card as posted.

**A printed URL is not proof of arrival.** `post` prints a `127.0.0.1` link by
formatting a string — it never contacts a server, and it cannot tell whether the
owner is running one. `owner_can_see_it` is the field that means something.

Use `--no-sync` (or `PIXEL_PHYSICS_REVIEW_NO_SYNC=1`) only when you deliberately
want a local-only queue. Sync failure is never fatal: the card is written to disk
first, so it survives and goes out on the next sync.

## Being told when the verdict lands

Posting is fire-and-forget, but you do not have to keep checking. Add `--notify`
and you will be pinged when the owner releases your verdicts:

```
python3 scripts/review.py post --notify …
```

That starts **one** background watcher per session — not one per card — which
delivers into this session and exits when none of your cards are open. The owner
presses a single button in the page to release everything they have answered, so
several verdicts for you arrive as **one** message, costing one wake-up turn
rather than one per card.

The ping is a digest: card title, choice, rating, first line of the comment.
`review.py inbox` remains the source of truth for the full comment and the pin
locations — read it when the ping arrives.

If nothing is watching, nothing is lost: the verdict sits in the queue and
`inbox` finds it exactly as it always did. `--notify` is an accelerator, never
the only path. It needs a Claude Code session (it writes to that session's own
inbox socket), and says so on stderr if there is none.

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

### From a phone

```
python3 scripts/review.py serve --lan
```

Prints a `http://<lan-ip>:<port>/?k=<key>` link (and copies it to the clipboard)
that any device on the same Wi-Fi can open; the page is laid out for a small
screen, and the full-screen viewer pans, pinches and pins by touch. Without the
flag the server stays on loopback exactly as before and no key is created —
`--lan` is the whole of the opt-in, and anything on that network holding the key
can answer cards, so it is the owner's call to make, not yours.

This changes nothing about how you post. It only means a card you queue may be
judged from the couch, which is one more reason the card has to stand on its own:
a title, a question, and the discrete event count in `meta`.

## Checking the tooling itself

```
python3 scripts/review_selftest.py
python3 scripts/review_mobile.py      # phone layout and touch, in a real browser
```

Covers what can actually break: concurrent posts from several worktrees, a post
killed mid-write, media outliving its worktree, a blind verdict resolving to the
real label, a `--wait` timeout leaving the card intact, and the cross-machine
transport — two real clones sharing only a remote, a verdict travelling back, a
concurrent push from both, and an offline post reporting itself as undelivered.
It also covers the LAN key: loopback exempt, every route including `/media/`
gated, a rebound hostname refused, and no key file written without `--lan`.

`review_mobile.py` is the half a static check cannot make. It drives Chromium at
390x844 with touch input and reviews a card using only taps, drags and pinches,
then renders the desktop layout from the current page and from git HEAD and
asserts every element they share is in the same place — a media query leaking
into the page the owner actually uses is the likeliest regression in that file.

Two clones, not two worktrees: worktrees share a queue directory, so a
worktree-only check passes while the case that actually broke still fails.

The suite forces `PIXEL_PHYSICS_REVIEW_NO_SYNC=1` for everything except its own
throwaway remote. Without that it pushed 108 fixture cards to the project's real
GitHub remote — a test harness must not be able to reach production.

The visual half — pixelated upscale, the scrubber, the blind reveal — still has
to be judged by eye in the page.
