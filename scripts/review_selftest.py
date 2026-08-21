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
        test_every_page_button_traces_to_a_card()
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
