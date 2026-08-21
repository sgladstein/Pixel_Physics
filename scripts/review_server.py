#!/usr/bin/env python3
"""Localhost HTTP server for the visual review queue.

Stdlib only, bound to 127.0.0.1. Started once from any worktree; because the
queue root is shared per clone (see review_lib.review_root), that one server
sees every worktree's cards.

The server holds no queue state in memory -- every request re-reads the
directory. Several agents are writing into it concurrently, and a listdir is
far cheaper than any cache invalidation scheme that would actually be correct.

Write responsibilities are split so no two processes ever write one file: the
server owns responses/ and nothing else. Agents own cards/ and seen/.
"""

from __future__ import annotations

import json
import mimetypes
import posixpath
import sys
import threading
import webbrowser
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import urlparse, parse_qs

sys.path.insert(0, str(Path(__file__).resolve().parent))

import review_lib as rl  # noqa: E402

MAX_BODY = 4 * 1024 * 1024  # a response carries text and pins, never media


class ReviewHandler(BaseHTTPRequestHandler):
    server_version = "PixelPhysicsReview/1.0"
    root: Path = None          # set on the server instance
    page_path: Path = None

    # -- plumbing ---------------------------------------------------------

    def log_message(self, fmt, *args):
        # The default logs every request to stderr, which drowns the one line
        # the owner actually needs (the URL). Keep errors only.
        pass

    def _send(self, code: int, body: bytes, ctype: str, extra=None):
        self.send_response(code)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(body)))
        # The page is regenerated from disk constantly; caching it hides edits.
        self.send_header("Cache-Control", "no-store")
        for key, val in (extra or {}).items():
            self.send_header(key, val)
        self.end_headers()
        if self.command != "HEAD":
            self.wfile.write(body)

    def _json(self, payload, code: int = 200):
        self._send(code, json.dumps(payload).encode("utf-8"), "application/json; charset=utf-8")

    def _error(self, code: int, message: str):
        self._json({"error": message}, code)

    def _body(self):
        length = int(self.headers.get("Content-Length") or 0)
        if length <= 0:
            return {}
        if length > MAX_BODY:
            raise ValueError("request body too large")
        return json.loads(self.rfile.read(length).decode("utf-8"))

    # -- routing ----------------------------------------------------------

    def do_GET(self):
        url = urlparse(self.path)
        path = url.path
        try:
            if path in ("/", "/index.html"):
                return self._send(200, self.page_path.read_bytes(), "text/html; charset=utf-8")
            if path == "/api/rev":
                # Sync state rides along on the poll the page already makes, so
                # a transport that has quietly stopped working becomes visible
                # without the page asking a second question.
                return self._json({"rev": rl.queue_revision(self.root),
                                   "sync": rl.sync_state(self.root)})
            if path == "/api/cards":
                return self._cards(parse_qs(url.query))
            if path.startswith("/media/"):
                return self._media(path[len("/media/"):])
            return self._error(404, "no route for %s" % path)
        except Exception as exc:  # a handler crash must not kill the server
            return self._error(500, "%s: %s" % (type(exc).__name__, exc))

    do_HEAD = do_GET

    def do_POST(self):
        path = urlparse(self.path).path
        try:
            parts = path.strip("/").split("/")
            if len(parts) == 4 and parts[0] == "api" and parts[1] == "cards":
                card_id, action = parts[2], parts[3]
                if not _safe_id(card_id):
                    return self._error(400, "bad card id")
                if action == "response":
                    return self._respond(card_id, self._body())
                if action == "status":
                    return self._status(card_id, self._body())
            return self._error(404, "no route for %s" % path)
        except ValueError as exc:
            return self._error(400, str(exc))
        except Exception as exc:
            return self._error(500, "%s: %s" % (type(exc).__name__, exc))

    # -- handlers ---------------------------------------------------------

    def _cards(self, query):
        cards = rl.load_cards(self.root)
        board = (query.get("board") or [None])[0]
        status = (query.get("status") or [None])[0]
        filtered = [
            c for c in cards
            if (not board or c.get("board") == board) and (not status or c["status"] == status)
        ]
        return self._json({
            "rev": rl.queue_revision(self.root),
            "root": str(self.root),
            "sync": rl.sync_state(self.root),
            "boards": rl.load_boards(cards),
            "cards": filtered,
            "counts": {
                "open": sum(1 for c in cards if c["status"] == "open"),
                "answered": sum(1 for c in cards if c["status"] == "answered"),
                "archived": sum(1 for c in cards if c["status"] == "archived"),
            },
        })

    def _respond(self, card_id: str, body: dict):
        card = rl.load_card(self.root, card_id)
        if card is None:
            return self._error(404, "no such card")

        # Resolve the chosen index back to its real label. Under `blind` the
        # page shuffled the panes, so the index the owner clicked is not the
        # index in the card -- the page sends both and the label is what any
        # agent reading this back actually needs.
        choice = body.get("choice")
        choice_label = None
        if isinstance(choice, int) and 0 <= choice < len(card["items"]):
            choice_label = card["items"][choice]["label"]

        existing = card.get("response") or {}
        response = {
            "card_id": card_id,
            "answered_at": rl.utc_now(),
            "choice": choice,
            "choice_label": choice_label,
            "rating": body.get("rating"),
            "comment": (body.get("comment") or "").strip(),
            "annotations": body.get("annotations") or [],
            "archived": existing.get("archived", False),
            "blind_was": body.get("blind_order"),
        }
        rl.save_response(self.root, card_id, response)
        return self._json({"ok": True, "response": response})

    def _status(self, card_id: str, body: dict):
        card = rl.load_card(self.root, card_id)
        if card is None:
            return self._error(404, "no such card")
        want = body.get("status")
        if want not in ("archived", "open"):
            return self._error(400, "status must be 'archived' or 'open'")

        # Archiving is stored on the response because that is the file the
        # server owns. An unanswered card being archived gets a response
        # carrying only the flag -- deliberately no answered_at, so it does not
        # show up in an agent's inbox as if the owner had judged it.
        response = card.get("response") or {"card_id": card_id, "annotations": []}
        response["archived"] = (want == "archived")
        rl.save_response(self.root, card_id, response)
        return self._json({"ok": True, "status": rl.card_status(rl.load_card(self.root, card_id))})

    def _media(self, rel: str):
        """Serve artifact bytes, refusing anything outside media/.

        The server binds to loopback, but the queue holds paths written by
        other processes, so the containment check is done against the resolved
        real path rather than trusted from the URL.
        """
        rel = posixpath.normpath(rel).lstrip("/")
        if rel.startswith("..") or "\\" in rel:
            return self._error(400, "bad media path")
        base = rl.media_dir(self.root).resolve()
        target = (base / rel).resolve()
        if base not in target.parents and target != base:
            return self._error(403, "outside media root")
        if not target.is_file():
            return self._error(404, "no such artifact")
        ctype = mimetypes.guess_type(target.name)[0] or "application/octet-stream"
        # Media never changes once written (ids are unique), so it is the one
        # thing worth letting the browser cache -- a frame scrubber requests
        # hundreds of images.
        return self._send(200, target.read_bytes(), ctype,
                          {"Cache-Control": "public, max-age=31536000, immutable"})


def _safe_id(card_id: str) -> bool:
    return bool(card_id) and all(ch.isalnum() or ch in "-_" for ch in card_id)


def _sync_loop(root: Path, interval: float, stop: threading.Event) -> None:
    """Pull cloud agents' cards in and push the owner's verdicts back out.

    On a timer because the owner should not have to run anything: a card posted
    from a Claude Code web session lands on the remote, and appears in the page
    on its own within one interval. Failures are recorded rather than raised --
    `sync_now` never throws -- and the page surfaces them, because a transport
    that silently stopped looks exactly like nobody having posted anything.
    """
    while not stop.is_set():
        try:
            rl.sync_now(root)
        except Exception:
            pass  # sync_now records its own failure; the loop must not die
        stop.wait(interval)


def serve(root: Path, port: int, open_browser: bool = False,
          sync_interval: float = 60.0) -> int:
    page = Path(__file__).resolve().parent / "review_page.html"
    if not page.is_file():
        page = root / "bin" / "review_page.html"
    if not page.is_file():
        raise SystemExit("review_page.html not found next to review_server.py")

    handler = type("BoundReviewHandler", (ReviewHandler,), {"root": root, "page_path": page})
    try:
        httpd = ThreadingHTTPServer(("127.0.0.1", port), handler)
    except OSError as exc:
        raise SystemExit(
            "cannot bind 127.0.0.1:%d (%s). Another review server is probably "
            "already running -- open http://127.0.0.1:%d/ , or pass --port."
            % (port, exc, port)
        )
    httpd.daemon_threads = True

    stop = threading.Event()
    if sync_interval > 0 and not rl.sync_disabled():
        threading.Thread(target=_sync_loop, args=(root, sync_interval, stop),
                         daemon=True).start()

    url = "http://127.0.0.1:%d/" % port
    print("review queue: %s" % root)
    if rl.sync_disabled():
        print("NOT syncing: %s is set. Cards from other machines will not appear."
              % rl.NO_SYNC_ENV)
    elif sync_interval <= 0:
        print("NOT syncing: --sync-interval 0. Cards from other machines will not appear.")
    elif not rl.remote_url():
        print("NOT syncing: no git remote 'origin' here. Cards posted from a cloud "
              "session will not appear. Start this from inside the repo checkout.")
    else:
        print("syncing with origin/%s every %ds" % (rl.SYNC_BRANCH, int(sync_interval)))
    print("open %s" % url, flush=True)
    if open_browser:
        threading.Timer(0.5, lambda: webbrowser.open(url)).start()
    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        print("\nstopped")
    finally:
        stop.set()
        httpd.server_close()
    return 0


if __name__ == "__main__":
    import argparse
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--port", type=int, default=rl.DEFAULT_PORT)
    ap.add_argument("--open", action="store_true")
    ap.add_argument("--sync-interval", type=float, default=60.0)
    a = ap.parse_args()
    sys.exit(serve(rl.review_root(), a.port, a.open, a.sync_interval))
