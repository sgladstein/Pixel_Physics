#!/usr/bin/env python3
"""Prove the review page works on a phone, by driving one.

The mobile work's whole failure mode is that it passes in a desktop browser and
fails in a hand, so this stands up a real queue, serves it twice -- the current
page and the one from git HEAD~ -- and hands both to `review_mobile.js`, which
runs Chromium at 390x844 with touch input and reviews a card using nothing but
taps, drags and pinches. The second server exists so the desktop layout can be
compared against the page as it was, which is the regression a media query is
most likely to cause.

    python3 scripts/review_mobile.py [--baseline <git-ref>] [--out DIR]

Needs Chromium and Playwright, which the container has; on a machine without
them it says so and exits 0 rather than failing a suite for a missing browser.
"""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
import tempfile
import zlib
import struct
from http.server import ThreadingHTTPServer
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import review_lib as rl  # noqa: E402

CHROME = "/opt/pw-browsers/chromium-1194/chrome-linux/chrome"
NODE_MODULES = "/opt/node22/lib/node_modules/playwright"


def png(path: Path, w: int, h: int, fill=(40, 60, 90)) -> Path:
    """A deterministic PNG, so nothing here depends on the engine building."""
    raw = b"".join(b"\x00" + bytes(fill) * w for _ in range(h))

    def chunk(tag, data):
        body = tag + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body))

    path.write_bytes(
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(raw))
        + chunk(b"IEND", b"")
    )
    return path


def serve(root: Path, page: Path, port: int):
    import review_server
    handler = type("T", (review_server.ReviewHandler,),
                   {"root": root, "page_path": page, "lan_token": ""})
    httpd = ThreadingHTTPServer(("127.0.0.1", port), handler)
    httpd.daemon_threads = True
    import threading
    threading.Thread(target=httpd.serve_forever, daemon=True).start()
    return httpd


def fixtures(root: Path, art: Path) -> None:
    """One A/B and one tall strip -- the two shapes a phone is worst at."""
    run = [sys.executable, str(HERE / "review.py")]
    env = dict(os.environ, **{rl.ROOT_ENV: str(root), rl.NO_SYNC_ENV: "1"})
    wide = png(art.parent / "wide.png", 1200, 90, (90, 60, 40))
    a = png(art.parent / "a.png", 256, 160, (40, 70, 110))
    b = png(art.parent / "b.png", 256, 160, (110, 70, 40))
    subprocess.run(run + ["ab", "--title", "Collapse debris spread",
                          "--question", "Which reads as real breakage?",
                          "--a", str(a), "--b", str(b), "--no-sync"],
                   env=env, check=True, capture_output=True)
    subprocess.run(run + ["post", "--title", "Seam row after the sweep reorder",
                          "--question", "Is the seam still visible?",
                          "--image", str(wide), "--no-sync"],
                   env=env, check=True, capture_output=True)


RAIL_BOARDS = 40   # headroom: the owner's real queue carries 14 today


def board_fixture(root: Path, n: int) -> None:
    """A queue whose only job is to make the sidebar long.

    Seeded well past the real board count rather than at it. A fixture pinned
    to today's number is a bar set from the current state, and the check that
    signed this button off used *two* boards -- which cannot produce an
    overflowing sidebar at any viewport, so it could never have failed.
    """
    for i in range(n):
        rl.save_card(root, {
            "id": "board%03d" % i, "board": "board-%02d" % i, "kind": "single",
            "title": "card on board-%02d" % i, "question": "?", "items": [],
            "created_at": rl.utc_now(), "origin": {"branch": "main"}})


# Undo the fix in CSS only: `#rail` back to one scroller, the actions back to
# flowing after the board list. This is what the sidebar was when the owner
# reported the button missing.
SABOTAGE = """
<style id="sabotage">
  #rail { display: block; overflow-y: auto; }
  #rail-scroll { overflow: visible; min-height: auto; }
  #rail-actions { border-top: 0; margin-top: 0; }
</style>
"""


def sabotage_page(src: Path, dst: Path) -> Path:
    """A copy of the current page with the rail fix disabled.

    The "can this check fail" assertion used to render `git HEAD`'s page, which
    works exactly until the fix lands -- then the baseline IS the fix, the
    assertion goes green for the wrong reason, and one commit later nobody
    remembers why. Breaking the mechanism on purpose keeps the question
    answerable forever, and it is the repo's own rule: break the replacement
    and confirm the guard notices.
    """
    dst.write_text(src.read_text(encoding="utf-8") + SABOTAGE, encoding="utf-8")
    return dst


def real_board_count() -> int:
    """What the owner's queue actually holds, for headroom reporting only.

    Read before the fixture root is installed in the environment, or this
    reports on the fixture and always says the headroom is fine.
    """
    try:
        return len(rl.load_boards(rl.load_cards(rl.review_root(create=False))))
    except Exception:
        return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--baseline", default="HEAD",
                    help="git ref to render the desktop comparison against "
                         "(default HEAD -- the page before uncommitted edits)")
    ap.add_argument("--out", metavar="DIR",
                    help="write the screenshots here and keep them "
                         "(default: a temp dir, deleted on the way out)")
    args = ap.parse_args()

    if not Path(CHROME).exists() or not Path(NODE_MODULES).exists():
        print("skipping: no Chromium/Playwright here (%s)" % CHROME)
        return 0

    os.environ[rl.NO_SYNC_ENV] = "1"
    # Same reason as the selftest: notify defaults on, and the fixtures
    # would each leave a detached watcher polling a deleted temp queue.
    os.environ.pop(rl.SOCKET_ENV, None)
    real = real_board_count()
    base = Path(tempfile.mkdtemp(prefix="review-mobile-"))
    out = Path(args.out) if args.out else base / "shots"
    out.mkdir(parents=True, exist_ok=True)
    root = base / "queue"
    root.mkdir()
    os.environ[rl.ROOT_ENV] = str(root)

    old_page = base / "baseline.html"
    got = subprocess.run(["git", "show", "%s:scripts/review_page.html" % args.baseline],
                         cwd=str(HERE.parent), capture_output=True)
    if got.returncode != 0:
        print("cannot read %s:scripts/review_page.html -- skipping the desktop "
              "comparison" % args.baseline)
        old_page.write_bytes((HERE / "review_page.html").read_bytes())
    else:
        old_page.write_bytes(got.stdout)

    rail = base / "rail"
    rail.mkdir()

    try:
        fixtures(root, base / "art.png")
        board_fixture(rail, RAIL_BOARDS)
        print("rail fixture: %d boards (the live queue has %s)"
              % (RAIL_BOARDS, real or "unknown"))
        if real >= RAIL_BOARDS:
            print("  raise RAIL_BOARDS -- the fixture no longer has headroom "
                  "over the real queue")
        servers = [serve(root, HERE / "review_page.html", 7471),
                   serve(root, old_page, 7472),
                   serve(rail, HERE / "review_page.html", 7473),
                   serve(rail, sabotage_page(HERE / "review_page.html",
                                             base / "sabotaged.html"), 7474)]
        try:
            rc = subprocess.run(["node", str(HERE / "review_mobile.js"),
                                 "7471", "7472", "7473", "7474", str(out)]).returncode
        finally:
            for srv in servers:
                srv.shutdown()
                srv.server_close()
        answered = len(list(rl.responses_dir(root).glob("*.json")))
        print("\nverdicts written by the phone run: %d" % answered)
        if answered < 1:
            print("  FAIL  the touch submit never reached responses/")
            rc = rc or 1
        if args.out:
            print("screenshots: %s" % out)
        return rc
    finally:
        shutil.rmtree(base, ignore_errors=True)


if __name__ == "__main__":
    sys.exit(main())
