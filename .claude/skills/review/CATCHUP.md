# Catching up a session that started before the tooling changed

**You were pointed here because the `CLAUDE.md` you read at startup is stale.**
This file lives in the repo alongside the code it describes, so it is current
whatever changed since your session began — unlike a prompt pasted into chat,
which went stale within a day the one time it was tried.

Everything below is safe to run twice. If you already caught up earlier, this
loses nothing and creates no duplicates.

## What exists now

A review queue: you render an artifact — an image, a before/after pair, a blind
A/B, or a frame sequence — post it with a written question, the owner judges it
in a browser, and you collect the verdict later.

Cards travel over the **git remote**, on an orphan branch `review-queue`. That
matters most if you are a cloud or web session: your clone is in your own
container, on a disk the owner cannot read, so a card written only to local disk
reaches nobody.

## 1. Get the tool

If your worktree has `scripts/review.py` **and** you have merged `main`, use it
directly and skip to step 2.

Otherwise install it from `main` — including if you bootstrapped it before, as
any copy older than the transport fix cannot deliver anything:

```
git fetch -q origin main
D="$(git rev-parse --path-format=absolute --git-common-dir)/pixel-physics-review/bin"
mkdir -p "$D"
for f in review.py review_lib.py review_server.py review_page.html; do
  git show origin/main:scripts/$f > "$D/$f"
done
```

Then use `python3 "$D/review.py" …` wherever the docs say `scripts/review.py`.
This writes only inside `.git/`, so it cannot dirty your working tree or disturb
a commit you are part-way through.

## 2. Flush anything already queued

```
python3 "$D/review.py" sync
```

If you posted cards earlier that never reached the owner, they are still on your
local disk and this pushes them. **Do not post them again** — that gives the
owner duplicates of the same questions.

## 3. The field that matters

`post` output includes:

```json
"owner_can_see_it": true
```

If that is `false`, the card is on local disk only and the owner cannot see it.
The reason is in the same output. Say so — do not report the card as posted.

**A printed URL is not proof of arrival.** `post` prints a `127.0.0.1` link by
formatting a string; it never contacts a server and cannot tell whether the
owner is running one. This is not hypothetical: it is exactly how three cards
were once reported as delivered into a container the owner could not read.

`post` syncs for you, and `inbox` and `wait` sync before they read, so you
rarely call `sync` by hand after this. Sync failure is never fatal — the card is
written to disk first and goes out on the next attempt.

## 4. How the owner wants it used

The primary way to get the owner's feedback, meant to be used **constantly**.
Everything this project optimises for is judged by eye, so when a change is
visible, **post it rather than describe it** — "this looks better" is precisely
the claim the owner has to check, and a sentence is not checkable.

Post when:

- a change alters anything on screen, including one you are confident about;
- you are about to claim something looks, moves or feels better;
- a complaint could mean two things — render both readings and ask which;
- a step is "judge by eye" — post *before* declaring it done;
- you are choosing between approaches and the difference is visual: post a
  blind A/B (`ab --blind`) rather than arguing it out;
- the question is whether something *moves* right — post an animation, not a
  contact sheet. `filmstrip` encodes a GIF headlessly (`gif=1 out=x.gif`) and
  `post --gif` attaches it.

Crop generously and declare the part that matters: render roughly double the
region of interest and pass `--focus center`. The owner sees your framing by
default and can zoom out into the margin, so a too-tight crop no longer costs a
re-render.

Two house rules, both from failures already paid for here:

- **Put the discrete event count in the card's `meta`.** The page renders it
  under the image. A collapse here once read as "chunks are working" from a
  picture whose body count was zero for the whole run.
- **Prefer a paired comparison** over one run against a remembered impression.

Posting is fire-and-forget: post, carry on, collect with `review.py inbox` later
or in a later session. Add `--notify` to be pinged when the owner releases your
verdicts — one background watcher per session, and several verdicts arrive as a
single message. `--wait` is for one case only — a wrong guess would waste
the work you are about to do.

## 5. Then

Run `review.py inbox` to pick up anything already answered, carry on with what
you were doing, and post a card for the next visible thing you change.

When you next merge or rebase, take `main` so you get `scripts/` and the current
`CLAUDE.md` properly.

Full protocol, including the JSON card spec:

```
git show origin/main:.claude/skills/review/SKILL.md
```
