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

    try:
        fixtures(root, base / "art.png")
        a = serve(root, HERE / "review_page.html", 7471)
        b = serve(root, old_page, 7472)
        try:
            rc = subprocess.run(["node", str(HERE / "review_mobile.js"),
                                 "7471", "7472", str(out)]).returncode
        finally:
            for s in (a, b):
                s.shutdown()
                s.server_close()
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
