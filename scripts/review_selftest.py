#!/usr/bin/env python3
"""End-to-end checks for the review queue. Run: python3 scripts/review_selftest.py

These are not unit tests. What can actually break here is not a function's
return value but the claims the design makes about *concurrency and loss*:
several agents in several worktrees write to one directory, a killed post must
leave nothing half-written, and a verdict must survive the worktree that asked
for it. Each check below is one of those claims.

The browser half -- pixelated upscale, the frame scrubber, blind reveal -- has
to be judged by eye and is not covered here. See .claude/skills/review/SKILL.md.
"""

from __future__ import annotations

import json
import os
import shutil
import struct
import subprocess
import sys
import tempfile
import threading
import time
import zlib
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import review_lib as rl  # noqa: E402

FAILURES = []


def check(ok: bool, label: str, detail: str = "") -> None:
    print(("  PASS  " if ok else "  FAIL  ") + label + (" -- " + detail if detail else ""))
    if not ok:
        FAILURES.append(label)


def run(*args, root=None, expect=0, stdin=None):
    env = dict(os.environ)
    if root:
        env[rl.ROOT_ENV] = str(root)
    proc = subprocess.run([sys.executable, str(HERE / "review.py"), *args],
                          capture_output=True, text=True, env=env, input=stdin)
    if expect is not None and proc.returncode != expect:
        raise AssertionError("review.py %s exited %d\n%s" % (args, proc.returncode, proc.stderr))
    return proc


def png(path: Path, w=24, h=16, fill=(120, 90, 60)) -> Path:
    raw = bytearray()
    for _ in range(h):
        raw.append(0)
        raw.extend(bytes(fill) * w)

    def chunk(tag, data):
        return (struct.pack(">I", len(data)) + tag + data
                + struct.pack(">I", zlib.crc32(tag + data) & 0xffffffff))
    path.write_bytes(b"\x89PNG\r\n\x1a\n"
                     + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0))
                     + chunk(b"IDAT", zlib.compress(bytes(raw), 1))
                     + chunk(b"IEND", b""))
    return path


# --------------------------------------------------------------------------

def test_concurrent_posts(root: Path, art: Path) -> None:
    """The claim that makes 'nothing gets lost' true: no index, no lock."""
    print("\nconcurrent posts from many agents")
    procs = [subprocess.Popen(
        [sys.executable, str(HERE / "review.py"), "post", "--board", "burst",
         "--title", "burst %d" % i, "--question", "q", "--image", str(art)],
        env={**os.environ, rl.ROOT_ENV: str(root)},
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL) for i in range(24)]
    for p in procs:
        p.wait()
    cards = rl.load_cards(root)
    check(len(cards) == 24, "all 24 cards survive", "got %d" % len(cards))
    check(len({c["id"] for c in cards}) == 24, "no id collisions")
    check(len({c["title"] for c in cards}) == 24, "no card overwrote another")
    check(all((rl.media_dir(root) / c["items"][0]["files"][0]).is_file() for c in cards),
          "every card's media landed")


def test_crash_safety(root: Path) -> None:
    """A card is visible only once it is complete -- tmp + os.replace."""
    print("\nposts killed mid-write")
    big = png(root.parent / "big.png", 700, 500)
    for i in range(14):
        p = subprocess.Popen(
            [sys.executable, str(HERE / "review.py"), "post", "--title", "crash %d" % i,
             "--question", "q", "--image", str(big)],
            env={**os.environ, rl.ROOT_ENV: str(root)},
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        time.sleep(0.03 + (i % 7) * 0.035)
        p.kill()
        p.wait()

    unreadable = [f.name for f in rl.cards_dir(root).glob("*.json")
                  if rl.read_json(f) is None]
    check(not unreadable, "no half-written card is readable", str(unreadable))
    stray = [f.name for f in rl.cards_dir(root).iterdir() if f.name.startswith(".")]
    check(not stray, "no temp files left in cards/", str(stray))

    size = big.stat().st_size
    truncated = [rel for c in rl.load_cards(root) for it in c["items"] for rel in it["files"]
                 if not (rl.media_dir(root) / rel).is_file()
                 or (rl.media_dir(root) / rel).stat().st_size != size]
    # Media is copied before the card is written, so a card can never reference
    # an artifact that did not finish copying.
    check(not truncated, "no card points at truncated media", str(truncated[:3]))

    # Construct the case rather than hoping a kill produced one: this run may
    # legitimately have zero orphans, and a check that cannot fail proves
    # nothing about the code it is named for.
    orphan = rl.media_dir(root) / "20200101T000000000Z-deadbe"
    orphan.mkdir(parents=True, exist_ok=True)
    (orphan / "000-x.png").write_bytes(b"x")
    kept = [rel for c in rl.load_cards(root) for it in c["items"] for rel in it["files"]]
    pruned = rl.prune_orphan_media(root)
    check(orphan.name in pruned and not orphan.exists(), "gc removes media with no card")
    check(all((rl.media_dir(root) / rel).is_file() for rel in kept),
          "gc leaves every carded artifact alone", "%d kept" % len(kept))


def test_media_outlives_source(root: Path) -> None:
    """The producing worktree is disposable; the card is not."""
    print("\nmedia survives its source")
    tmp = Path(tempfile.mkdtemp())
    art = png(tmp / "doomed.png")
    out = json.loads(run("post", "--title", "from a doomed tree", "--question", "q",
                         "--image", str(art), root=root).stdout)
    shutil.rmtree(tmp)
    card = rl.load_card(root, out["id"])
    stored = rl.media_dir(root) / card["items"][0]["files"][0]
    check(stored.is_file() and stored.stat().st_size > 0,
          "artifact still readable after its source directory is deleted")


def test_blind_resolves_to_real_label(root: Path, art_a: Path, art_b: Path) -> None:
    """Blinding must cost the agent nothing: the stored verdict names the real
    option, not the shuffled slot the owner clicked."""
    print("\nblind A/B")
    out = json.loads(run("ab", "--title", "blind", "--question", "which?",
                         "--a", str(art_a), "--b", str(art_b),
                         "--a-label", "uniform", "--b-label", "graded",
                         "--blind", root=root).stdout)
    card = rl.load_card(root, out["id"])
    check(card["blind"] is True, "card is flagged blind")
    # The server resolves index -> label; mimic that write here.
    rl.save_response(root, card["id"], {
        "card_id": card["id"], "answered_at": rl.utc_now(), "choice": 1,
        "choice_label": card["items"][1]["label"], "rating": 4,
        "comment": "grit reads right", "annotations": [], "archived": False})
    got = rl.load_card(root, card["id"])
    check(got["status"] == "answered", "status derives from the response file")
    check(got["response"]["choice_label"] == "graded", "verdict names the real option")


def test_wait_degrades(root: Path, art: Path) -> None:
    """Blocking must never be load-bearing: a timeout leaves the same disk
    state as a plain post."""
    print("\nblocking wait")
    proc = run("post", "--title", "blocker", "--question", "q", "--image", str(art),
               "--wait", "--timeout", "2", root=root, expect=2)
    card_id = json.loads(proc.stdout)["id"]
    card = rl.load_card(root, card_id)
    check(card is not None and card["status"] == "open", "card still queued after a timeout")
    check(card["blocking"] is True, "card is marked as blocking")
    check("inbox" in proc.stderr, "timeout tells the agent how to retrieve later")

    def answer():
        time.sleep(1.0)
        rl.save_response(root, card_id, {
            "card_id": card_id, "answered_at": rl.utc_now(), "choice": 0,
            "choice_label": "x", "rating": 5, "comment": "", "annotations": [],
            "archived": False})
    threading.Thread(target=answer, daemon=True).start()
    started = time.monotonic()
    run("wait", card_id, "--timeout", "20", root=root)
    check(time.monotonic() - started < 15, "wait returns as soon as the answer lands")


def test_inbox_is_never_silently_empty(root: Path, art: Path) -> None:
    """An empty inbox must mean 'no answers', never 'the filter matched
    nothing because it could not tell who I am'."""
    print("\ninbox identity")
    outside = Path(tempfile.mkdtemp())
    env = {**os.environ, rl.ROOT_ENV: str(root)}
    env.pop("PIXEL_PHYSICS_REVIEW_AGENT", None)
    proc = subprocess.run([sys.executable, str(HERE / "review.py"), "inbox"],
                          capture_output=True, text=True, env=env, cwd=str(outside))
    shutil.rmtree(outside, ignore_errors=True)
    check("cannot identify this agent" in proc.stderr,
          "an unidentifiable caller is warned, not silently filtered to nothing")
    check(len(json.loads(proc.stdout)) > 0, "and still sees the answers that exist")


def _clone_run(clone: Path, *args, expect=0):
    """Run the CLI inside a clone, letting the root resolve from that clone.

    Deliberately does not set PIXEL_PHYSICS_REVIEW_DIR: the point of these
    checks is that two clones resolve to two *different* roots and still
    exchange cards, which an env override would paper over.
    """
    # Sync is re-enabled only here, where the clones point at a bare repo this
    # suite created in its own temp dir and nowhere else.
    env = {k: v for k, v in os.environ.items()
           if k not in (rl.ROOT_ENV, rl.NO_SYNC_ENV)}
    proc = subprocess.run([sys.executable, str(HERE / "review.py"), *args],
                          capture_output=True, text=True, env=env, cwd=str(clone))
    if expect is not None and proc.returncode != expect:
        raise AssertionError("review.py %s in %s exited %d\n%s"
                             % (args, clone.name, proc.returncode, proc.stderr[:400]))
    return proc


def test_cross_machine_transport(base: Path, art: Path) -> None:
    """Two clones sharing only a remote -- the case a worktree test cannot see.

    The original design shared a *directory*, which every worktree of one clone
    can reach and no other machine can. A cloud session posted three cards and
    the owner's page said "Nothing queued here". These checks use two real
    clones because two worktrees would pass while that case still failed.
    """
    print("\ncross-machine transport")
    origin = base / "origin.git"
    subprocess.run(["git", "init", "-q", "--bare", str(origin)], check=True,
                   capture_output=True)
    a, b = base / "cloneA", base / "cloneB"
    for c in (a, b):
        r = subprocess.run(["git", "clone", "-q", str(origin), str(c)],
                           capture_output=True, text=True)
        if r.returncode != 0:
            return check(False, "clone the test remote", r.stderr.strip()[:120])

    roots = [_clone_run(c, "root").stdout.strip() for c in (a, b)]
    check(roots[0] != roots[1], "the two clones really do have separate queues")

    out = json.loads(_clone_run(a, "post", "--board", "t", "--title", "from clone A",
                                "--question", "does this cross?", "--image", str(art)).stdout)
    check(out["owner_can_see_it"] is True,
          "post reports the card actually reached the remote")
    card_id = out["id"]

    _clone_run(b, "sync")
    got = json.loads(_clone_run(b, "get", card_id).stdout)
    check(got and got["id"] == card_id, "the card arrives in the other clone")
    stored = Path(roots[1]) / "media" / got["items"][0]["files"][0]
    check(stored.is_file() and stored.read_bytes() == art.read_bytes(),
          "and its artifact arrives byte-identical")

    # The verdict has to travel back, or a cloud agent can never learn anything.
    rl.save_response(Path(roots[1]), card_id, {
        "card_id": card_id, "answered_at": rl.utc_now(), "choice": 0,
        "choice_label": "x", "rating": 4, "comment": "it crossed",
        "annotations": [{"item": 0, "x": 0.5, "y": 0.5, "note": "here"}],
        "archived": False})
    _clone_run(b, "sync")
    inbox = json.loads(_clone_run(a, "inbox").stdout)
    match = [c for c in inbox if c["id"] == card_id]
    check(bool(match), "the verdict comes back to the clone that asked")
    if match:
        r = match[0]["response"]
        check(r["rating"] == 4 and r["comment"] == "it crossed" and r["annotations"],
              "with its rating, comment and pins intact")


def test_offline_is_reported_as_offline(base: Path, art: Path) -> None:
    """Regression guard for a bug this change set introduced and fixed.

    `git fetch` fails identically for "no such branch" and "no such remote", so
    sync took the first-ever-sync path against a dead remote, rebuilt a tree
    identical to the last commit, saw nothing to push and reported ok. A card
    from an offline agent was reported as delivered -- the same misleading
    readout the honest-URL change exists to prevent, one layer down.
    """
    print("\noffline is reported as offline")
    origin = base / "origin.git"
    c = base / "cloneC"
    r = subprocess.run(["git", "clone", "-q", str(origin), str(c)],
                       capture_output=True, text=True)
    if r.returncode != 0:
        return check(False, "clone for the offline check", r.stderr.strip()[:120])
    subprocess.run(["git", "remote", "set-url", "origin", str(base / "nope.git")],
                   cwd=str(c), check=True, capture_output=True)

    proc = _clone_run(c, "post", "--title", "offline", "--question", "q",
                      "--image", str(art))
    out = json.loads(proc.stdout)
    check(proc.returncode == 0, "an offline post still exits 0 -- the card is on disk")
    check(out["owner_can_see_it"] is False,
          "and does NOT claim the owner can see it")
    check("warning" in out and "sync" in proc.stderr,
          "and says so on both stdout and stderr")
    root = Path(_clone_run(c, "root").stdout.strip())
    check((root / "cards" / (out["id"] + ".json")).is_file(),
          "the card is queued locally regardless")

    # Reconnecting must flush it, or "it'll go out later" is a lie.
    subprocess.run(["git", "remote", "set-url", "origin", str(origin)],
                   cwd=str(c), check=True, capture_output=True)
    res = json.loads(_clone_run(c, "sync").stdout)
    check(res["ok"] and res["pushed"], "reconnecting flushes the parked card")


def test_concurrent_push(base: Path, art: Path) -> None:
    """Disjoint paths mean concurrent pushes converge -- tested, not asserted."""
    print("\nconcurrent pushes from two clones")
    a, b = base / "cloneA", base / "cloneB"
    if not (a / ".git").is_dir():
        return check(False, "clones from the transport check are present")
    procs = []
    for i in range(3):
        for clone, tag in ((a, "A"), (b, "B")):
            env = {k: v for k, v in os.environ.items() if k != rl.ROOT_ENV}
            procs.append(subprocess.Popen(
                [sys.executable, str(HERE / "review.py"), "post", "--title",
                 "%s burst %d" % (tag, i), "--question", "q", "--image", str(art)],
                cwd=str(clone), env=env,
                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL))
    for pr in procs:
        pr.wait()
    for _ in range(2):
        for clone in (a, b):
            _clone_run(clone, "sync", expect=None)
    titles = set()
    counts = []
    for clone in (a, b):
        cards = json.loads(_clone_run(clone, "list").stdout)
        counts.append(len(cards))
        titles |= {c["title"] for c in cards}
    burst = {t for t in titles if "burst" in t}
    check(len(burst) == 6, "all 6 concurrently-posted cards survive", "got %d" % len(burst))
    check(counts[0] == counts[1], "both clones converge on the same queue",
          "%d vs %d" % tuple(counts))


def gif(path: Path, frames: int, w=8, h=8) -> Path:
    """Minimal GIF89a. Literal-only LZW -- inefficient, and valid, which is all
    a fixture needs. `frames=1` reproduces the file this guard exists for."""
    def lzw(indices, mcs):
        out, bits, nbits, width = bytearray(), 0, 0, mcs + 1
        def emit(code):
            nonlocal bits, nbits
            bits |= code << nbits; nbits += width
            while nbits >= 8:
                out.append(bits & 0xFF); bits >>= 8; nbits -= 8
        emit(1 << mcs)
        for i in indices: emit(i)
        emit((1 << mcs) + 1)
        if nbits: out.append(bits & 0xFF)
        return bytes(out)
    b = bytearray(b"GIF89a")
    b += struct.pack("<HHBBB", w, h, 0xF0 | 2, 0, 0)
    for c in [(0, 0, 0), (255, 255, 255)] + [(0, 0, 0)] * 6:
        b += bytes(c)
    b += b"\x21\xFF\x0BNETSCAPE2.0\x03\x01\x00\x00\x00"
    for f in range(frames):
        b += b"\x21\xF9\x04\x04" + struct.pack("<H", 8) + b"\x00\x00"
        b += b"\x2C" + struct.pack("<HHHHB", 0, 0, w, h, 0) + bytes([3])
        data = lzw([(x + f) % 2 for x in range(w * h)], 3)
        for i in range(0, len(data), 255):
            chunk = data[i:i + 255]
            b += bytes([len(chunk)]) + chunk
        b += b"\x00"
    b += b"\x3B"
    path.write_bytes(bytes(b))
    return path


def test_single_frame_gif_is_refused(root: Path, base: Path) -> None:
    """A still with an animation's name must not reach the owner silently.

    `filmstrip out=x.gif` without `gif=1` writes the contact sheet as a
    one-frame GIF -- `image::save_buffer` picks its encoder from the extension.
    Valid file, plausible name, cannot move. Two cards shipped that way while
    the agents reported posting animations, and nothing anywhere said otherwise.
    """
    print("\nsingle-frame GIFs")
    still, moving = gif(base / "still.gif", 1), gif(base / "moving.gif", 4)
    check(rl.gif_frames(still) == 1, "frame counter reads a one-frame GIF", "got %s" % rl.gif_frames(still))
    check(rl.gif_frames(moving) == 4, "and a real animation", "got %s" % rl.gif_frames(moving))
    check(rl.gif_frames(base / "a.png") is None, "and declines a PNG")

    proc = run("post", "--title", "t", "--question", "q", "--gif", str(still),
               root=root, expect=None)
    check(proc.returncode != 0, "--gif refuses a one-frame GIF")
    check("gif=1" in (proc.stdout + proc.stderr),
          "and names the cause (filmstrip without gif=1)")

    out = json.loads(run("post", "--title", "t", "--question", "q",
                         "--gif", str(moving), root=root).stdout)
    card = rl.load_card(root, out["id"])
    check(card["items"][0].get("gif_frames") == 4,
          "a real animation posts, with its frame count recorded for the page")

    proc = run("post", "--title", "t", "--question", "q", "--image", str(still),
               root=root, expect=0)
    check("single frame" in proc.stderr,
          "--image stays an escape hatch, but warns")


def test_focus_is_validated_at_post_time(root: Path, art: Path) -> None:
    """A focus rect the page cannot satisfy must fail where it is still fixable.

    The rect is in the rendered image's pixels, which is easy to confuse with
    world coordinates -- and a wrong one renders as a blank or clipped viewport,
    reading as a broken tool rather than a bad argument.
    """
    print("\nfocus validation")
    size = rl.image_size(art)
    check(size is not None, "image_size reads a PNG header", str(size))
    w, h = size

    out = json.loads(run("post", "--title", "t", "--question", "q", "--image", str(art),
                         "--focus", "center", root=root).stdout)
    card = rl.load_card(root, out["id"])
    check(card["items"][0]["focus"] == "center", "center shorthand is stored verbatim")

    rect = [w // 4, h // 4, w // 2, h // 2]
    out = json.loads(run("post", "--title", "t", "--question", "q", "--image", str(art),
                         "--focus", ",".join(str(n) for n in rect), root=root).stdout)
    check(rl.load_card(root, out["id"])["items"][0]["focus"] == rect,
          "an explicit rect round-trips")

    for bad, why in ((f"{w-2},{h-2},40,40", "outside the image"),
                     ("1,2,3", "wrong arity"),
                     ("1,2,0,4", "zero width"),
                     ("-5,0,10,10", "negative origin")):
        proc = run("post", "--title", "t", "--question", "q", "--image", str(art),
                   "--focus", bad, root=root, expect=None)
        check(proc.returncode != 0, "rejects %s (%s)" % (bad, why))

    out = json.loads(run("post", "--title", "t", "--question", "q",
                         "--image", str(art), root=root).stdout)
    check(rl.load_card(root, out["id"])["items"][0]["focus"] is None,
          "a card with no --focus stores none, so its zoom cycle is unchanged")


def test_verdicts_batch_per_agent(root: Path, art: Path) -> None:
    """Six verdicts for one agent must cost one wake-up turn, not six.

    This is the whole reason the release is a button rather than a per-card
    ping: the wake replaces the turn the owner would spend saying "I reviewed
    your card", so batched it is roughly free and un-batched it multiplies with
    every card answered in a sitting. A per-card design would pass every other
    check in this suite, so the batching is asserted directly.
    """
    print("\nbatched verdict release")
    sids = {"agent-alpha": 6, "agent-beta": 2, "agent-gamma": 1}
    ids = {}
    for sid, n in sids.items():
        for i in range(n):
            env = dict(os.environ, **{rl.SESSION_ENV: sid})
            proc = subprocess.run(
                [sys.executable, str(HERE / "review.py"), "post", "--title",
                 "%s card %d" % (sid, i), "--question", "q", "--image", str(art)],
                capture_output=True, text=True,
                env=dict(env, **{rl.ROOT_ENV: str(root)}))
            ids.setdefault(sid, []).append(json.loads(proc.stdout)["id"])

    for sid, cards in ids.items():
        for cid in cards:
            rl.save_response(root, cid, {
                "card_id": cid, "answered_at": rl.utc_now(), "choice": 0,
                "choice_label": "A", "rating": 4, "comment": "looks right",
                "annotations": [], "archived": False})

    pend = rl.pending_release_count(root)
    check(pend == {"agents": 3, "verdicts": 9, "orphans": 0},
          "the button counts 9 verdicts across 3 agents", str(pend))

    res = rl.release_verdicts(root)
    check(res["agents"] == 3 and res["verdicts"] == 9, "one press releases all of them")
    for sid, n in sids.items():
        got = rl.take_outbox(root, sid)
        check(len(got) == n, "%s gets exactly its own %d" % (sid, n), "got %d" % len(got))

    # The assertion the feature exists for: ONE message, not one per card.
    entries = rl.take_outbox(root, "agent-alpha")
    ping = compose_ping_via_cli(entries)
    check(ping.count("\n  ") == 6, "all 6 of one agent's verdicts go in ONE message",
          "found %d lines" % ping.count("\n  "))
    for sid in ("agent-beta", "agent-gamma"):
        check(sid not in ping, "and it carries nothing belonging to %s" % sid)

    again = rl.release_verdicts(root)
    check(again["verdicts"] == 0, "a second press releases nothing", str(again))

    rl.mark_delivered(root, entries, "agent-alpha")
    check(rl.take_outbox(root, "agent-alpha") == [], "delivery empties that agent's outbox")
    card = rl.load_card(root, ids["agent-alpha"][0])
    check(rl.notify_state(root, card) == "delivered", "and the card reads as delivered")
    check(rl.notify_state(root, rl.load_card(root, ids["agent-beta"][0])) == "released",
          "while an undelivered agent's card stays released")


def compose_ping_via_cli(entries):
    import importlib.util
    spec = importlib.util.spec_from_file_location("review_cli", HERE / "review.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod.compose_ping(entries)


def test_unnotifiable_cards_are_reported(root: Path, art: Path) -> None:
    """A card with no session id must be visible, not silently skipped."""
    print("\nun-notifiable verdicts")
    env = {k: v for k, v in os.environ.items() if k != rl.SESSION_ENV}
    proc = subprocess.run(
        [sys.executable, str(HERE / "review.py"), "post", "--title", "no session",
         "--question", "q", "--image", str(art)],
        capture_output=True, text=True, env=dict(env, **{rl.ROOT_ENV: str(root)}))
    cid = json.loads(proc.stdout)["id"]
    check(rl.load_card(root, cid)["origin"].get("session_id") in (None, ""),
          "a card posted with no session id records none")
    rl.save_response(root, cid, {"card_id": cid, "answered_at": rl.utc_now(),
                                 "choice": 0, "choice_label": "A", "rating": 3,
                                 "comment": "", "annotations": [], "archived": False})
    res = rl.release_verdicts(root)
    check(any(o["id"] == cid for o in res["orphans"]),
          "and is reported as un-notifiable rather than dropped")
    check(rl.pending_release_count(root)["orphans"] >= 1,
          "the button surfaces the orphan count too")


def test_socket_frame_shape() -> None:
    """Guard the exact frame. `{"type":"message","text":…}` has a valid string
    type, so it passes the reader's type check and falls through to an
    unhandled-type branch that logs at debug level and does nothing. That shape
    failed silently twice before the protocol was read properly."""
    print("\nsocket frame")
    src = (HERE / "review_lib.py").read_text(encoding="utf-8")
    body = src[src.index("def send_to_own_session"):]
    body = body[:body.index("\ndef ")]
    check('"type": "user"' in body, 'the message frame is type "user"')
    check('"content": text' in body, "and the text lives at message.content")
    check('"type": "auth"' in body, "an auth frame is sent first")
    check('"type": "message"' not in body,
          'the silently-ignored {"type":"message"} shape is not used')
    check('"priority": "next"' in body,
          'priority is "next" so it lands between turns, not mid-work')


def test_every_page_button_traces_to_a_card() -> None:
    """A control the click handler cannot resolve is silently dead.

    The play button sat inside `<div class="scrub" data-card=...>` but carried
    no `data-card` of its own, and the handler read the attribute only from the
    button -- so it did nothing, with no error, from the day it was written. The
    browser check that was supposed to cover it exercised the *slider* instead,
    which resolves its card by a different path and passed.

    This is a static lint rather than a browser test on purpose: it costs
    nothing, runs everywhere, and catches the whole class -- any future control
    that forgets the attribute.
    """
    print("\npage controls")
    import re
    src = (HERE / "review_page.html").read_text(encoding="utf-8")

    handler_ok = "btn.closest(\"[data-card]\")" in src
    check(handler_ok, "the click handler falls back to an enclosing [data-card]")

    # The full-screen viewer is browser-only, so these are structural rather
    # than behavioural: enough to catch it being deleted or unwired, not a
    # substitute for driving it. A quarter of posted media is wider than the
    # card pane at 1:1, so losing this silently would matter.
    for needle, why in (
            ('id="lightbox"', "the lightbox element exists"),
            ("openLightbox(card, Number(img.dataset.item))",
             "clicking a card image opens it full screen"),
            ("if (lb.card) {", "the lightbox takes the keyboard while open"),
            ("function lbClamp()", "panning is clamped so the image cannot leave the screen"),
            ("lbClamp();", "and the clamp is actually applied"),
            ("function lbDefaultFactor()",
             "the opening zoom adapts to extreme aspect ratios")):
        check(needle in src, why)

    orphans = []
    for m in re.finditer(r"<button\s+([^>]*?)>", src, re.S):
        attrs = m.group(1)
        if "data-act" not in attrs:
            continue
        act = re.search(r'data-act="([^"]*)"', attrs)
        if "data-card" not in attrs:
            orphans.append(act.group(1) if act else "?")
    check(not orphans, "every button[data-act] carries data-card", str(orphans))


def test_root_is_shared_across_worktrees() -> None:
    """The one hard requirement: every worktree of a clone resolves to one queue."""
    print("\ncross-worktree root")
    repo = HERE.parent
    try:
        main_root = subprocess.run(
            [sys.executable, str(HERE / "review.py"), "root", "--no-create"],
            capture_output=True, text=True, cwd=str(repo),
            env={k: v for k, v in os.environ.items() if k != rl.ROOT_ENV}).stdout.strip()
    except Exception as exc:
        return check(False, "resolve root in the main checkout", str(exc))

    wt = Path(tempfile.mkdtemp()) / "wt"
    made = subprocess.run(["git", "worktree", "add", "-q", "--detach", str(wt), "HEAD"],
                          cwd=str(repo), capture_output=True, text=True)
    if made.returncode != 0:
        return check(False, "create a linked worktree", made.stderr.strip()[:120])
    try:
        wt_root = subprocess.run(
            [sys.executable, str(HERE / "review.py"), "root", "--no-create"],
            capture_output=True, text=True, cwd=str(wt),
            env={k: v for k, v in os.environ.items() if k != rl.ROOT_ENV}).stdout.strip()
        check(wt_root == main_root and main_root.endswith("pixel-physics-review"),
              "a linked worktree resolves to the same queue as the main checkout",
              "%s vs %s" % (wt_root, main_root))
        check(Path(main_root).is_absolute(),
              "the root is absolute (a relative one gives every worktree its own queue)")
    finally:
        subprocess.run(["git", "worktree", "remove", "--force", str(wt)],
                       cwd=str(repo), capture_output=True)
        shutil.rmtree(wt.parent, ignore_errors=True)


# --------------------------------------------------------------------------


def _serve_for_test(root: Path, port: int, lan: bool):
    """Start a real server in-process and return (thread, httpd, token)."""
    sys.path.insert(0, str(HERE))
    import review_server
    import threading as _t
    page = HERE / "review_page.html"
    token = rl.lan_token(root) if lan else ""
    handler = type("T", (review_server.ReviewHandler,),
                   {"root": root, "page_path": page, "lan_token": token})
    from http.server import ThreadingHTTPServer
    httpd = ThreadingHTTPServer(("0.0.0.0" if lan else "127.0.0.1", port), handler)
    httpd.daemon_threads = True
    th = _t.Thread(target=httpd.serve_forever, daemon=True)
    th.start()
    return httpd, token


def _req(host: str, port: int, path: str, key: str = "", cookie: str = "",
         host_header: str = "", method: str = "GET"):
    """One raw request, so the Host header and the client address are ours."""
    import http.client
    conn = http.client.HTTPConnection(host, port, timeout=10)
    headers = {}
    if cookie:
        headers["Cookie"] = "%s=%s" % (rl.LAN_COOKIE, cookie)
    if host_header:
        headers["Host"] = host_header
    conn.request(method, path + ("?k=" + key if key else ""), headers=headers)
    resp = conn.getresponse()
    resp.read()
    out = (resp.status, resp.getheader("Set-Cookie") or "")
    conn.close()
    return out


def test_lan_access_is_gated(root: Path, art: Path) -> None:
    """The key is the only thing between the queue and the rest of the Wi-Fi.

    Every route has to be behind it -- `/media/` above all, since that is where
    the renders are and an <img> cannot send a header, which is why the key
    becomes a cookie at all.
    """
    print("\nLAN access")
    root.mkdir(parents=True, exist_ok=True)
    os.environ[rl.ROOT_ENV] = str(root)
    lan = rl.lan_address()
    if not lan:
        return check(True, "skipped: this machine has no LAN address to bind")

    httpd, token = _serve_for_test(root, 7466, lan=True)
    try:
        check(_req("127.0.0.1", 7466, "/api/rev")[0] == 200,
              "loopback still needs no key -- the desktop flow is untouched")
        check(_req(lan, 7466, "/api/rev")[0] == 401, "off-box with no key is refused")
        check(_req(lan, 7466, "/api/rev", key="WRONG123")[0] == 401,
              "off-box with the wrong key is refused")
        code, setc = _req(lan, 7466, "/", key=token)
        check(code == 200, "off-box with the key is served")
        check(rl.LAN_COOKIE in setc and token in setc,
              "...and the key comes back as a cookie", setc)
        check(_req(lan, 7466, "/api/rev", cookie=token)[0] == 200,
              "the cookie alone is enough afterwards")
        # The one an <img> depends on, and the easiest to forget.
        check(_req(lan, 7466, "/media/nope.png")[0] == 401,
              "/media/ is gated too -- 401 before 404")
        check(_req(lan, 7466, "/media/nope.png", cookie=token)[0] == 404,
              "...and reachable with the key")
        check(_req(lan, 7466, "/api/notify", cookie="", method="POST")[0] == 401,
              "POST is gated, not just GET")
        check(_req(lan, 7466, "/api/rev", cookie=token,
                   host_header="evil.example.com")[0] == 401,
              "a rebound hostname is refused even holding a valid key")
        check(_req(lan, 7466, "/media/../../etc/passwd", cookie=token)[0] in (400, 403, 404),
              "the media containment check was not relaxed to make LAN work")
    finally:
        httpd.shutdown()
        httpd.server_close()

    mode = oct(os.stat(root / rl.LAN_TOKEN_FILE).st_mode & 0o777)
    check(mode == "0o600", "the key file is readable only by its owner", mode)


def test_plain_serve_creates_no_key(root: Path) -> None:
    """No flag, no exposure and no key file. The default must not drift."""
    print("\nplain serve")
    root.mkdir(parents=True, exist_ok=True)
    httpd, token = _serve_for_test(root, 7467, lan=False)
    try:
        check(token == "", "no --lan means no key is even loaded")
        check(not (root / rl.LAN_TOKEN_FILE).exists(),
              "...and none is written to the queue")
        check(httpd.server_address[0] == "127.0.0.1",
              "the listener stays on loopback", str(httpd.server_address))
        check(_req("127.0.0.1", 7467, "/api/rev")[0] == 200,
              "and the desktop page still answers with no key")
    finally:
        httpd.shutdown()
        httpd.server_close()


def test_page_works_on_a_phone() -> None:
    """Static half of the mobile check; the browser drive is review_mobile.py.

    Structural, because the failure this guards is deletion or a rename, not
    behaviour: how it *looks* at 390px is a review-queue card, and whether the
    gestures work is a Playwright run. What is cheap to assert here is that the
    pieces are still wired to each other.
    """
    print("\nphone layout")
    import re
    src = (HERE / "review_page.html").read_text(encoding="utf-8")

    check("@media (max-width: 720px)" in src,
          "the page has mobile rules at all (it had none, and was unusable)")
    check("min-width: 0" in src.split("@media")[1],
          "the 280px pane floor is lifted, or a phone scrolls sideways")
    check("font-size: 16px" in src.split("@media")[1],
          "inputs are >=16px, or iOS zooms the page in on focus and stays there")
    check("touch-action: none" in src,
          "the lightbox claims its gestures from the browser")

    for needle, why in (
            ('addEventListener("pointerdown"', "the lightbox listens for pointers, not mice"),
            ("document.elementFromPoint", "a captured tap resolves its target by point, not ev.target"),
            ("lbPinch", "pinch-to-zoom exists"),
            ("lbToggleZoom", "double-tap toggles fit and 1:1"),
            ("lbTapTimer", "a first tap waits out a possible second before opening prompt()")):
        check(needle in src, why)

    check('addEventListener("mousedown"' not in src and 'addEventListener("mousemove"' not in src,
          "no mouse-only pan survives beside the pointer path")

    # The generalisation of the bug that left Close dead for a commit: every id
    # the script wires a listener to has to exist in the markup.
    ids = set(re.findall(r'id="([^"]+)"', src))
    wired = set(re.findall(r'getElementById\("([^"]+)"\)', src))
    missing = sorted(w for w in wired if w not in ids)
    check(not missing, "every getElementById the script wires actually exists",
          str(missing))


def main() -> int:
    # Isolate from any real remote, globally, before a single check runs.
    #
    # This is not belt-and-braces: without it the local-only checks inherit the
    # cwd's `origin` and push their fixtures to it. Run once from a real
    # checkout, the suite created a `review-queue` branch on the project's
    # GitHub remote holding 108 cards named "burst 3" and "crash 7". The checks
    # then failed too -- "all 24 cards survive -- got 105" -- because each run
    # pulled every previous run's fixtures back down. A test harness must not
    # be able to reach production, and here that is one environment variable.
    os.environ[rl.NO_SYNC_ENV] = "1"

    base = Path(tempfile.mkdtemp(prefix="review-selftest-"))
    art_a, art_b = png(base / "a.png"), png(base / "b.png", fill=(60, 90, 120))
    try:
        test_concurrent_posts(base / "q1", art_a)
        test_crash_safety(base / "q2")
        test_media_outlives_source(base / "q3")
        test_blind_resolves_to_real_label(base / "q4", art_a, art_b)
        test_wait_degrades(base / "q5", art_a)
        test_inbox_is_never_silently_empty(base / "q4", art_a)
        test_focus_is_validated_at_post_time(base / "q6", art_a)
        test_single_frame_gif_is_refused(base / "q7", base)
        test_verdicts_batch_per_agent(base / "q8", art_a)
        test_unnotifiable_cards_are_reported(base / "q9", art_a)
        test_socket_frame_shape()
        test_every_page_button_traces_to_a_card()
        test_page_works_on_a_phone()
        test_lan_access_is_gated(base / "q10", art_a)
        test_plain_serve_creates_no_key(base / "q11")
        test_root_is_shared_across_worktrees()
        transport = Path(tempfile.mkdtemp(prefix="review-transport-"))
        try:
            test_cross_machine_transport(transport, art_a)
            test_concurrent_push(transport, art_a)
            test_offline_is_reported_as_offline(transport, art_a)
        finally:
            shutil.rmtree(transport, ignore_errors=True)
    finally:
        shutil.rmtree(base, ignore_errors=True)

    print("\n%d checks failed" % len(FAILURES) if FAILURES else "\nall checks passed")
    return 1 if FAILURES else 0


if __name__ == "__main__":
    sys.exit(main())
