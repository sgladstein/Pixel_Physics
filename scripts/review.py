#!/usr/bin/env python3
"""Agent-facing CLI for the visual review queue.

    review.py serve                     start the page the owner judges from
    review.py serve --lan               ...and let a phone on the Wi-Fi reach it
    review.py post --json -             post a card (the general form)
    review.py post --title ... --image  post a one-image card
    review.py ab --a A.png --b B.png    post an A/B comparison
    review.py list / get / inbox        retrieve verdicts
    review.py wait <id>                 block on a card, when it truly blocks

Fire-and-forget is the standard: post, keep working, pick the verdict up with
`inbox` later or in a later session. `--wait` exists for the case where the
next stretch of work forks on the answer -- see `cmd_wait` for why blocking is
safe to offer here.

Run `review.py --help` or see .claude/skills/review/SKILL.md for the protocol.
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import time
import shutil
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import review_lib as rl  # noqa: E402


# --------------------------------------------------------------------------
# Self-install
# --------------------------------------------------------------------------

def refresh_bin(root: Path) -> None:
    """Keep a copy of the tooling inside the shared root.

    A worktree sitting on a branch that predates this feature has no
    scripts/review.py, but it still has the shared root -- so it can post via
    <root>/bin/review.py. Refreshing on every run means that copy tracks
    whatever the newest branch to run the tool has.
    """
    here = Path(__file__).resolve().parent
    for name in ("review.py", "review_lib.py", "review_server.py", "review_page.html"):
        src = here / name
        if not src.is_file():
            continue
        dest = root / "bin" / name
        try:
            if not dest.exists() or src.stat().st_mtime > dest.stat().st_mtime:
                shutil.copyfile(src, dest)
        except OSError:
            pass  # a read-only or racing copy must never fail a post


# --------------------------------------------------------------------------
# Card construction
# --------------------------------------------------------------------------

def build_card(root: Path, spec: dict) -> dict:
    """Normalise a user/agent-supplied spec into a stored card.

    The spec names artifacts by *source* path; the stored card names them by
    queue-relative path, because the source is in a worktree that may not
    outlive the question.
    """
    card_id = rl.new_id()
    kind = spec.get("kind") or "single"
    if kind not in rl.KINDS:
        raise SystemExit("unknown kind %r (expected one of %s)" % (kind, ", ".join(rl.KINDS)))

    title = (spec.get("title") or "").strip()
    if not title:
        raise SystemExit("a card needs a --title: it is what the owner scans the queue by")
    question = (spec.get("question") or "").strip()
    if not question:
        raise SystemExit(
            "a card needs a --question. A picture with no question asks the owner to "
            "guess what you are testing, which is the thing this tool exists to stop."
        )

    raw_items = spec.get("items") or []
    if not raw_items:
        raise SystemExit("a card needs at least one item with at least one file")

    items = []
    copied = 0
    for item in raw_items:
        files = item.get("files")
        if isinstance(files, str):
            files = [files]
        files = files or ([item["file"]] if item.get("file") else [])
        if not files:
            raise SystemExit("item %r has no files" % item.get("label"))
        # **An animation cannot share an item.** `files` is the frame-sequence
        # field: several entries in one item become a scrubbable strip, so a
        # GIF sitting beside a still does not render as an animation next to a
        # picture -- it becomes frame 0 of a two-frame sequence and the motion
        # is gone. Silently, and the card looks right from the posting side.
        # Refused rather than warned, and rather than quietly split: a card is
        # fire-and-forget, so a warning on stderr is read at exactly the moment
        # nobody is looking, and splitting would have to guess which item the
        # label, caption and meta belong to. This costs one re-post; the
        # alternative costs a round trip through the owner.
        animated = [f for f in files if str(f).lower().endswith(".gif")]
        if animated and len(files) > 1:
            raise SystemExit(
                "item %r puts %s in the same item as %d other file(s).\n"
                "`files` is the frame-sequence field, so they would render as a "
                "%d-frame strip and the animation would be lost.\n"
                "Give the gif an item of its own -- one file per item, as many items "
                "as you like -- or use --gif and --image, which do that for you."
                % (item.get("label") or "(unlabelled)", animated[0], len(files) - 1, len(files))
            )
        stored = []
        for src in files:
            stored.append(rl.copy_media(root, card_id, src, copied))
            copied += 1
        items.append({
            # Left empty for a lone unlabelled artifact: a pane header reading
            # "image" above the only image on the card is pure noise.
            "label": item.get("label")
                     or ("" if len(raw_items) == 1 else "item %d" % (len(items) + 1)),
            "files": stored,
            "caption": item.get("caption") or "",
            # Rendered as a table under the image. The repo's own rule: when a
            # change adds a discrete "this happened" event, the count must be
            # read next to the picture, because two very different mechanisms
            # look identical at the zoom a contact sheet is judged at.
            "meta": item.get("meta") or {},
            # Which part of this image the agent actually wants judged. The page
            # frames it by default and one zoom step out reveals the margin the
            # agent deliberately rendered around it -- so a too-tight crop costs
            # a scroll rather than a re-render round trip.
            "focus": item.get("focus"),
        })
        if items[-1]["focus"]:
            check_focus_fits(items[-1]["focus"], root, stored[0])
        check_animation(items[-1], item, root, stored)

    ask = spec.get("ask") or {}
    return {
        "id": card_id,
        "created": rl.utc_now(),
        "board": (spec.get("board") or "inbox").strip() or "inbox",
        "origin": rl.origin_info(),
        "title": title,
        "question": question,
        "context_md": spec.get("context_md") or spec.get("context") or "",
        "kind": kind,
        "blind": bool(spec.get("blind")),
        "blocking": bool(spec.get("blocking")),
        "items": items,
        "ask": {
            "choice": ask.get("choice", kind in ("ab", "before_after")),
            "rating": ask.get("rating", True),
            "comment": ask.get("comment", True),
            "annotations": ask.get("annotations", True),
        },
    }


def parse_focus(value: str):
    """`center` or `x,y,w,h`, in the rendered image's own pixel coordinates.

    `center` is the middle half by area, which is exactly the region of
    interest when the agent rendered at double it -- so the common case needs
    no arithmetic on their side, and no dimensions here. The page expands it
    against the image's natural size.
    """
    v = (value or "").strip().lower()
    if v in ("center", "centre"):
        return "center"
    parts = [p.strip() for p in v.split(",")]
    if len(parts) != 4:
        raise SystemExit("--focus takes 'center' or 'x,y,w,h' (got %r)" % value)
    try:
        rect = [int(p) for p in parts]
    except ValueError:
        raise SystemExit("--focus values must be whole pixels (got %r)" % value)
    if rect[2] <= 0 or rect[3] <= 0:
        raise SystemExit("--focus width and height must be positive (got %r)" % value)
    if rect[0] < 0 or rect[1] < 0:
        raise SystemExit("--focus x and y must not be negative (got %r)" % value)
    return rect


def check_focus_fits(focus, root: Path, stored_rel: str) -> None:
    """Reject a rect that falls outside the image, while it is still fixable.

    A focus rect the page cannot satisfy renders as a blank or clipped
    viewport, which reads as a broken tool rather than a bad argument -- so it
    fails here, where the agent is still holding the command that caused it.
    """
    if focus == "center" or not focus:
        return
    size = rl.image_size(rl.media_dir(root) / stored_rel)
    if size is None:
        return  # not a format we can measure; the page clamps instead
    w, h = size
    x, y, fw, fh = focus
    if x + fw > w or y + fh > h:
        raise SystemExit(
            "--focus %d,%d,%d,%d falls outside the %dx%d image. The focus rect is "
            "in the *rendered image's* pixels, not world coordinates."
            % (x, y, fw, fh, w, h))


def check_animation(stored_item: dict, spec_item: dict, root: Path, stored) -> None:
    """Record a GIF's frame count, and refuse a still posted as an animation.

    A one-frame GIF looks like an animation by name, size and header, and is
    exactly what `filmstrip out=x.gif` writes when `gif=1` is left off: the
    contact-sheet branch calls `image::save_buffer`, which picks its encoder
    from the extension and stores the whole sheet as a single frame. Nothing
    errors. The agent reports an animation, the owner sees a still, and neither
    can tell why -- which is precisely what happened.

    Refused for `--gif`, where the intent is explicit; a warning for `--image`,
    which stays the escape hatch for deliberately posting a still.
    """
    if len(stored) != 1 or not stored[0].lower().endswith(".gif"):
        return
    n = rl.gif_frames(rl.media_dir(root) / stored[0])
    if n is None:
        return
    stored_item["gif_frames"] = n
    if n > 1:
        return
    msg = ("%s is a GIF with a single frame, so it cannot animate. The usual "
           "cause is running filmstrip with out=*.gif but WITHOUT gif=1 -- that "
           "writes the contact sheet as a one-frame GIF. Re-run with gif=1."
           % (spec_item.get("files") or ["?"])[0])
    if spec_item.get("_gif_flag"):
        raise SystemExit(msg)
    print("review: " + msg, file=sys.stderr)


def parse_item_flag(value: str) -> dict:
    """`--item "Label:path.png:caption"` -- colon separated, caption optional.

    Windows paths contain a drive colon, so split from the left on exactly two
    colons and let the label take the first field.
    """
    parts = value.split(":", 1)
    if len(parts) == 1:
        return {"label": "item", "files": [parts[0]]}
    label, rest = parts
    # Re-join anything that looks like a drive letter back onto the path.
    if len(rest) > 1 and rest[0] in "\\/" and len(label) == 1:
        return {"label": "item", "files": [value]}
    file_part, _, caption = rest.partition("::")
    return {"label": label.strip(), "files": [file_part.strip()], "caption": caption.strip()}


# --------------------------------------------------------------------------
# Commands
# --------------------------------------------------------------------------

def emit(card: dict, root: Path, port: int, sync: dict = None) -> None:
    """Report what is actually true, not what is merely formatted.

    The old version printed a 127.0.0.1 URL unconditionally, by string
    formatting -- it never contacted a server. So a card posted from a cloud
    session into a queue on a disk the owner cannot read still printed a
    plausible, clickable link, and the agent reported success in good faith.
    The owner saw "Nothing queued here." Whether the card can actually reach a
    human is the one thing this output has to get right.
    """
    out = {
        "id": card["id"],
        "board": card["board"],
        "queue": str(root),
        "url": "http://127.0.0.1:%d/#%s" % (port, card["id"]),
        "url_note": "opens the owner's page if they are serving; not checked from here",
    }
    if sync is None:
        out["sync"] = "not attempted"
        out["owner_can_see_it"] = "only if you are on the owner's machine"
    elif sync.get("ok"):
        out["sync"] = {"ok": True, "branch": sync.get("branch"),
                       "pushed": len(sync.get("pushed") or []),
                       "pulled": len(sync.get("pulled") or [])}
        out["owner_can_see_it"] = True
    else:
        out["sync"] = {"ok": False,
                       "reason": sync.get("error") or sync.get("skipped")}
        out["owner_can_see_it"] = False
        out["warning"] = (
            "This card is on local disk only. If you are not running on the "
            "owner's machine they cannot see it. Re-run `review.py sync` once "
            "the remote is reachable."
        )
    print(json.dumps(out, indent=2))


def cmd_post(args) -> int:
    root = rl.review_root()
    refresh_bin(root)

    if args.json:
        text = sys.stdin.read() if args.json == "-" else Path(args.json).read_text(encoding="utf-8")
        try:
            spec = json.loads(text)
        except json.JSONDecodeError as exc:
            raise SystemExit("card spec is not valid JSON: %s" % exc)
    else:
        spec = {}

    # Flags override the JSON spec, so a shortcut can be layered onto a file.
    for flag, key in (("title", "title"), ("question", "question"), ("board", "board"),
                      ("kind", "kind")):
        val = getattr(args, flag, None)
        if val:
            spec[key] = val
    if args.context:
        spec["context_md"] = args.context
    if args.context_file:
        spec["context_md"] = Path(args.context_file).read_text(encoding="utf-8")
    if args.blind:
        spec["blind"] = True
    focus = parse_focus(args.focus) if args.focus else None
    if args.image:
        spec.setdefault("items", []).extend({"files": [p], "focus": focus}
                                            for p in args.image)
    if args.gif:
        # Mechanically identical to --image: the queue copies any file and the
        # page renders whatever it is. It exists because an agent decides what
        # to post by reading --help, and "gif" appeared nowhere in it -- so the
        # capability was invisible at the one moment it mattered, and agents
        # kept sending contact sheets for questions about motion.
        for path in args.gif:
            if not path.lower().endswith(".gif"):
                print("review: --gif was given %s, which is not a .gif. Posting it "
                      "anyway; use --image for stills." % path, file=sys.stderr)
            spec.setdefault("items", []).extend(
                [{"files": [path], "focus": focus, "_gif_flag": True}])
    if args.item:
        spec.setdefault("items", []).extend(
            dict(parse_item_flag(v), focus=focus) for v in args.item)
    if args.wait:
        spec["blocking"] = True

    card = build_card(root, spec)
    rl.save_card(root, card)
    if should_notify(args):
        note = ensure_watcher(root)
        if note:
            print("review: notify: %s" % note, file=sys.stderr)
    # The card is on disk before sync runs, so a transport failure costs
    # delivery time and never the question itself.
    emit(card, root, args.port, maybe_sync(args))

    if args.wait:
        return do_wait(root, card["id"], args.timeout)
    return 0


def cmd_ab(args) -> int:
    root = rl.review_root()
    refresh_bin(root)
    ab_focus = parse_focus(args.focus) if args.focus else None
    spec = {
        "title": args.title,
        "question": args.question,
        "board": args.board,
        "kind": "ab",
        "blind": args.blind,
        "blocking": args.wait,
        "context_md": args.context or "",
        "items": [
            {"label": args.a_label, "files": args.a, "focus": ab_focus},
            {"label": args.b_label, "files": args.b, "focus": ab_focus},
        ],
    }
    if args.context_file:
        spec["context_md"] = Path(args.context_file).read_text(encoding="utf-8")
    if len(args.a) > 1 or len(args.b) > 1:
        # Two frame sequences: the page gives them one shared scrubber, which
        # is the only way to compare motion at the same instant.
        spec["kind"] = "frames"
    card = build_card(root, spec)
    rl.save_card(root, card)
    if should_notify(args):
        note = ensure_watcher(root)
        if note:
            print("review: notify: %s" % note, file=sys.stderr)
    emit(card, root, args.port, maybe_sync(args))
    if args.wait:
        return do_wait(root, card["id"], args.timeout)
    return 0


def maybe_sync(args, root: Path = None) -> dict:
    """Best-effort transport. Returns None when the caller opted out."""
    if getattr(args, "no_sync", False):
        return None
    result = rl.sync_now(root or rl.review_root())
    if not result.get("ok") and result.get("error"):
        print("review: sync failed (%s) — card is queued locally and will go "
              "out on the next sync" % result["error"], file=sys.stderr)
    return result


def watcher_lock(root: Path) -> Path:
    return root / ("watch-%s.pid" % (rl.session_id() or "unknown"))


def should_notify(args) -> bool:
    """Explicit flag if given, otherwise on wherever it can actually work.

    Both variables are required to deliver -- the socket is where the ping is
    written, the session id is which outbox to read -- so keying the default on
    them means the automatic case never spawns a watcher that would only be
    able to report that it cannot work.
    """
    want = getattr(args, "notify", None)
    if want is not None:
        return want
    return bool(os.environ.get(rl.SOCKET_ENV) and rl.session_id())


def ensure_watcher(root: Path) -> str:
    """Start one deliverer per session, not one per card.

    The lock is keyed on the session, so ten cards posted by one agent share a
    single watcher -- which is the point: the owner releases a batch and the
    agent is woken once.
    """
    if not os.environ.get(rl.SOCKET_ENV):
        return "no %s — nothing to deliver into; verdicts stay in `inbox`" % rl.SOCKET_ENV
    if not rl.session_id():
        return "no %s — cannot address this session" % rl.SESSION_ENV
    lock = watcher_lock(root)
    existing = rl.read_json(lock)
    if existing and _pid_alive(existing.get("pid")):
        return ""
    proc = subprocess.Popen(
        [sys.executable, str(Path(__file__).resolve()), "watch"],
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        stdin=subprocess.DEVNULL, start_new_session=True)
    rl.write_json_atomic(lock, {"pid": proc.pid, "session_id": rl.session_id(),
                                "started": rl.utc_now()})
    return ""


def _pid_alive(pid) -> bool:
    if not isinstance(pid, int):
        return False
    try:
        os.kill(pid, 0)
        return True
    except (OSError, ProcessLookupError):
        return False


def compose_ping(entries: list) -> str:
    lines = ["Review verdicts are in for %d of your card%s:"
             % (len(entries), "" if len(entries) == 1 else "s")]
    for i, e in enumerate(entries, 1):
        lines.append("  %d. %s" % (i, e.get("digest") or e["card_id"]))
    lines.append("")
    lines.append("Run `review.py inbox` for the full comments and pin locations, "
                 "then act on them.")
    return "\n".join(lines)


def cmd_watch(args) -> int:
    """Deliver released verdicts into this session, batched.

    Runs detached in the agent's own environment, so the write comes from a
    process the session spawned -- which carries the child token and bypasses
    the inbound gate a stranger process would be held by.
    """
    root = rl.review_root()
    sid = rl.session_id()
    if not sid:
        print("watch: %s not set — cannot tell which session to deliver to"
              % rl.SESSION_ENV, file=sys.stderr)
        return 1
    if not os.environ.get(rl.SOCKET_ENV):
        print("watch: %s not set — no inbox to deliver into" % rl.SOCKET_ENV,
              file=sys.stderr)
        return 1

    deadline = time.monotonic() + args.timeout
    while time.monotonic() < deadline:
        if not args.no_sync:
            rl.sync_now(root)
        entries = rl.take_outbox(root, sid)
        if entries:
            err = rl.send_to_own_session(compose_ping(entries))
            if err:
                print("watch: %s" % err, file=sys.stderr)
                return 1
            # Marker first, then drop the outbox entry: a crash between them
            # re-delivers rather than losing the verdict.
            rl.mark_delivered(root, entries, sid)
        mine_open = [c for c in rl.load_cards(root)
                     if (c.get("origin") or {}).get("session_id") == sid
                     and c["notify"] in ("unanswered", "pending", "released")]
        if not mine_open:
            return 0
        time.sleep(args.interval)
    return 0


def cmd_notify(args) -> int:
    """The button's job, from the command line."""
    root = rl.review_root()
    result = rl.release_verdicts(root)
    if not args.no_sync:
        rl.sync_now(root)
    print(json.dumps(result, indent=2))
    return 0


def cmd_sync(args) -> int:
    root = rl.review_root()
    result = rl.sync_now(root)
    print(json.dumps(result, indent=2))
    return 0 if result.get("ok") else 1


def cmd_list(args) -> int:
    root = rl.review_root()
    cards = rl.load_cards(root)
    if args.board:
        cards = [c for c in cards if c.get("board") == args.board]
    if args.status:
        cards = [c for c in cards if c["status"] == args.status]
    if args.mine:
        me = rl.origin_info()
        cards = [c for c in cards if _is_mine(c, me)]
    print(json.dumps([_summary(c) for c in cards], indent=2))
    return 0


def cmd_get(args) -> int:
    root = rl.review_root()
    card = rl.load_card(root, args.id)
    if card is None:
        print("no such card: %s" % args.id, file=sys.stderr)
        return 1
    print(json.dumps(card, indent=2))
    if args.mark_seen and card.get("response"):
        rl.mark_seen(root, card["id"])
    return 0


def cmd_inbox(args) -> int:
    """Answered cards this agent posted and has not yet read.

    This is the retrieval half of the loop. Without it an agent returning in a
    later session would have to re-read the whole queue to discover that one of
    its own questions came back.
    """
    root = rl.review_root()
    # Pull first: the verdict was given on the owner's machine, so without this
    # `inbox` reports "nothing answered" while the answer sits on the remote.
    maybe_sync(args, root)
    me = rl.origin_info()
    cards = [
        c for c in rl.load_cards(root)
        if c.get("response") and c["response"].get("answered_at") and not c["seen"]
    ]
    if not args.all and _identifiable(me):
        cards = [c for c in cards if _is_mine(c, me)]
    elif not args.all:
        # Run from outside a git worktree with no explicit agent name, "mine"
        # matches nothing -- and an empty inbox reads exactly like "no answers
        # yet", which is the failure this whole tool exists to prevent. Widen
        # rather than silently hide, and say so.
        print("review: cannot identify this agent (not in a worktree, and "
              "PIXEL_PHYSICS_REVIEW_AGENT is unset) -- showing every unseen "
              "answer, not just yours.", file=sys.stderr)
    print(json.dumps([_answered(c) for c in cards], indent=2))
    if args.mark_seen:
        for card in cards:
            rl.mark_seen(root, card["id"])
    return 0


def cmd_wait(args) -> int:
    root = rl.review_root()
    maybe_sync(args, root)
    return do_wait(root, args.id, args.timeout, sync=not args.no_sync)


def do_wait(root: Path, card_id: str, timeout: float, sync: bool = True) -> int:
    """Block until answered; exit 2 on timeout.

    Blocking is never load-bearing: a wait that times out leaves exactly the
    disk state a fire-and-forget post would have, so the card is still queued,
    still answerable and still reachable via `inbox`. Waiting only changes
    *when* the agent learns the answer -- never whether it can.

    The card is written before the wait starts, so an agent killed while parked
    here still leaves its question behind.
    """
    # Poll the remote as well as the disk, or a cloud agent waits forever on an
    # answer that was given on a machine it cannot see.
    deadline = time.monotonic() + timeout
    resp = None
    while resp is None:
        slice_s = min(30.0, max(1.0, deadline - time.monotonic()))
        resp = rl.wait_for_response(root, card_id, slice_s)
        if resp is not None or time.monotonic() >= deadline:
            break
        if sync:
            rl.sync_now(root)
    if resp is None:
        print(json.dumps({"id": card_id, "status": "timeout",
                          "note": "still queued; retrieve later with `review.py inbox`"}),
              file=sys.stderr)
        return 2
    print(json.dumps(resp, indent=2))
    rl.mark_seen(root, card_id)
    return 0


def cmd_serve(args) -> int:
    root = rl.review_root()
    refresh_bin(root)
    import review_server
    return review_server.serve(root, args.port, open_browser=args.open,
                               sync_interval=args.sync_interval, lan=args.lan)


def cmd_gc(args) -> int:
    root = rl.review_root()
    removed = rl.prune_orphan_media(root)
    print(json.dumps({"removed_media_dirs": removed}, indent=2))
    return 0


def cmd_root(args) -> int:
    print(rl.review_root(create=not args.no_create))
    return 0


# --------------------------------------------------------------------------
# Helpers
# --------------------------------------------------------------------------

def _identifiable(me: dict) -> bool:
    return bool(me.get("branch") or me.get("worktree") or me.get("agent"))


def _is_mine(card: dict, me: dict) -> bool:
    """Match on branch or worktree, not both.

    An agent may post from a worktree and read back from the main checkout
    after merging, or the reverse -- so either matching is enough.
    """
    origin = card.get("origin") or {}
    return (
        (origin.get("branch") and origin["branch"] == me.get("branch"))
        or (origin.get("worktree") and origin["worktree"] == me.get("worktree"))
        or (me.get("agent") and origin.get("agent") == me["agent"])
    )


def _summary(card: dict) -> dict:
    return {
        "id": card["id"], "board": card.get("board"), "title": card.get("title"),
        "kind": card.get("kind"), "status": card["status"],
        "blocking": card.get("blocking", False), "created": card.get("created"),
        "branch": (card.get("origin") or {}).get("branch"),
    }


def _answered(card: dict) -> dict:
    resp = card.get("response") or {}
    out = _summary(card)
    out["question"] = card.get("question")
    out["response"] = {
        "answered_at": resp.get("answered_at"),
        "choice": resp.get("choice"),
        "choice_label": resp.get("choice_label"),
        "rating": resp.get("rating"),
        "comment": resp.get("comment"),
        "annotations": resp.get("annotations") or [],
    }
    return out


# --------------------------------------------------------------------------

def main(argv=None) -> int:
    p = argparse.ArgumentParser(prog="review.py", description=__doc__,
                                formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = p.add_subparsers(dest="cmd", required=True)

    def add_notify(sp):
        # Tri-state: None means "decide from the environment". Opt-in was the
        # original design and it failed in the obvious way -- it depended on
        # every agent remembering a flag, and none did, so the owner pressed a
        # button that woke nobody. The watcher is one process per session that
        # exits when none of your cards are open; that is cheaper than the
        # feature not happening.
        g = sp.add_mutually_exclusive_group()
        g.add_argument("--notify", dest="notify", action="store_true", default=None,
                       help="be pinged when the owner releases the verdict; on by "
                            "default inside a Claude Code session (one background "
                            "watcher per session, not per card)")
        g.add_argument("--no-notify", dest="notify", action="store_false",
                       help="post without starting a watcher; the verdict still "
                            "arrives, you just have to read it with `inbox`")

    def add_no_sync(sp):
        sp.add_argument("--no-sync", action="store_true",
                        help="skip the git transport (offline, or a local-only queue)")

    def add_port(sp):
        # Attached per-subcommand rather than to the top-level parser: a
        # top-level --port only parses *before* the subcommand, and
        # `review.py serve --port 8080` -- the order anyone actually types --
        # was rejected.
        sp.add_argument("--port", type=int, default=rl.DEFAULT_PORT,
                        help="review page port (default %d)" % rl.DEFAULT_PORT)

    sp = sub.add_parser("serve", help="serve the review page")
    sp.add_argument("--open", action="store_true", help="open a browser at the page")
    sp.add_argument("--lan", action="store_true",
                    help="also listen on the local network, key-gated, so the "
                         "queue can be reviewed from a phone on the same Wi-Fi")
    sp.add_argument("--sync-interval", type=float, default=60.0,
                    help="seconds between remote syncs; 0 disables (default 60)")
    add_port(sp)
    sp.set_defaults(func=cmd_serve)

    sp = sub.add_parser("post", help="post a card")
    sp.add_argument("--json", metavar="PATH", help="card spec as JSON; '-' for stdin")
    sp.add_argument("--title")
    sp.add_argument("--question")
    sp.add_argument("--board")
    sp.add_argument("--kind", choices=rl.KINDS)
    sp.add_argument("--context", help="markdown context shown above the images")
    sp.add_argument("--context-file")
    sp.add_argument("--image", action="append", metavar="PATH",
                    help="still artifact file; repeatable")
    sp.add_argument("--gif", action="append", metavar="PATH",
                    help="animated GIF; repeatable. Produce one headlessly with "
                         "`cargo run --release --example filmstrip -- gif=1 out=x.gif`")
    sp.add_argument("--item", action="append", metavar="LABEL:PATH[::CAPTION]",
                    help="labelled artifact; repeatable")
    sp.add_argument("--focus", metavar="SPEC",
                    help='"center" (the middle half — what you want when you rendered at double the region of interest) or x,y,w,h in the rendered image\'s pixels')
    sp.add_argument("--blind", action="store_true",
                    help="hide labels until after the owner chooses")
    sp.add_argument("--wait", action="store_true",
                    help="block until answered (only when the answer truly blocks you)")
    sp.add_argument("--timeout", type=float, default=1800)
    add_port(sp)
    add_no_sync(sp)
    add_notify(sp)
    sp.set_defaults(func=cmd_post)

    sp = sub.add_parser("ab", help="post an A/B comparison")
    sp.add_argument("--title", required=True)
    sp.add_argument("--question", required=True)
    sp.add_argument("--a", nargs="+", required=True, metavar="PATH")
    sp.add_argument("--b", nargs="+", required=True, metavar="PATH")
    sp.add_argument("--a-label", default="A")
    sp.add_argument("--b-label", default="B")
    sp.add_argument("--board", default="inbox")
    sp.add_argument("--context")
    sp.add_argument("--context-file")
    sp.add_argument("--focus", metavar="SPEC", help='"center" (the middle half — what you want when you rendered at double the region of interest) or x,y,w,h in the rendered image\'s pixels')
    sp.add_argument("--blind", action="store_true")
    sp.add_argument("--wait", action="store_true")
    sp.add_argument("--timeout", type=float, default=1800)
    add_port(sp)
    add_no_sync(sp)
    add_notify(sp)
    sp.set_defaults(func=cmd_ab)

    sp = sub.add_parser("list", help="list cards")
    sp.add_argument("--board")
    sp.add_argument("--status", choices=("open", "answered", "archived"))
    sp.add_argument("--mine", action="store_true")
    sp.set_defaults(func=cmd_list)

    sp = sub.add_parser("get", help="print one card and its response")
    sp.add_argument("id")
    sp.add_argument("--mark-seen", action="store_true")
    sp.set_defaults(func=cmd_get)

    sp = sub.add_parser("inbox", help="answered cards I posted and have not read")
    sp.add_argument("--all", action="store_true", help="not just mine")
    sp.add_argument("--mark-seen", action="store_true")
    add_no_sync(sp)
    sp.set_defaults(func=cmd_inbox)

    sp = sub.add_parser("wait", help="block until a card is answered")
    sp.add_argument("id")
    sp.add_argument("--timeout", type=float, default=1800)
    add_no_sync(sp)
    sp.set_defaults(func=cmd_wait)

    sp = sub.add_parser("watch", help="deliver released verdicts into this session")
    sp.add_argument("--interval", type=float, default=60.0)
    sp.add_argument("--timeout", type=float, default=6 * 3600)
    add_no_sync(sp)
    sp.set_defaults(func=cmd_watch)

    sp = sub.add_parser("notify",
                        help="release every answered verdict to the agent that asked")
    add_no_sync(sp)
    sp.set_defaults(func=cmd_notify)

    sp = sub.add_parser("sync", help="exchange cards and verdicts with the remote")
    sp.set_defaults(func=cmd_sync)

    sp = sub.add_parser("gc", help="remove media left behind by a killed post")
    sp.set_defaults(func=cmd_gc)

    sp = sub.add_parser("root", help="print the shared queue directory")
    sp.add_argument("--no-create", action="store_true")
    sp.set_defaults(func=cmd_root)

    args = p.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
