#!/usr/bin/env python3
"""Localhost HTTP server for the visual review queue.

Stdlib only, bound to 127.0.0.1 unless `serve --lan` says otherwise. Started
once from any worktree; because the queue root is shared per clone (see
review_lib.review_root), that one server sees every worktree's cards.

The server holds no queue state in memory -- every request re-reads the
directory. Several agents are writing into it concurrently, and a listdir is
far cheaper than any cache invalidation scheme that would actually be correct.

Write responsibilities are split so no two processes ever write one file: the
server owns responses/ and nothing else. Agents own cards/ and seen/.
"""

from __future__ import annotations

import hmac
import ipaddress
import json
import mimetypes
import posixpath
import sys
import threading
import time
import webbrowser
from http.cookies import SimpleCookie
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
    lan_token: str = ""        # non-empty only under `serve --lan`

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
        cookie = getattr(self, "_cookie", None)
        if cookie:
            self.send_header("Set-Cookie", cookie)
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

    # -- access control ---------------------------------------------------

    def _client_is_loopback(self) -> bool:
        addr = self.client_address[0]
        return addr in ("::1", "::ffff:127.0.0.1") or addr.startswith("127.")

    def _host_ok(self) -> bool:
        """Refuse a Host this server could not have been reached by.

        Without this a page on any website can point a name it controls at this
        machine's LAN address -- DNS rebinding -- and drive the queue from the
        browser of whoever visits it. SameSite=Lax still sends the cookie on a
        top-level navigation, so the key would ride along; only refusing the
        name stops it.
        """
        host = (self.headers.get("Host") or "").rsplit(":", 1)[0].strip("[]").lower()
        if not host:
            return False
        if host == "localhost" or host.endswith(".local"):
            return True
        try:
            ipaddress.ip_address(host)
            return True
        except ValueError:
            return False

    def _presented_key(self, url) -> str:
        wanted = parse_qs(url.query).get("k")
        if wanted and wanted[0]:
            return wanted[0]
        try:
            morsel = SimpleCookie(self.headers.get("Cookie") or "").get(rl.LAN_COOKIE)
        except Exception:
            return ""
        return morsel.value if morsel else ""

    def _gate(self, url) -> bool:
        """True if the request may proceed; otherwise it has been refused.

        Loopback stays exempt even with the LAN listener up. Loopback is the
        trust boundary this server already had, and exempting it keeps the
        desktop flow -- and any bookmark to http://127.0.0.1:PORT/ -- working
        exactly as before the flag existed.
        """
        if not self.lan_token or self._client_is_loopback():
            return True
        if not self._host_ok():
            return self._refuse("Reach this page by the address "
                                "<code>review.py serve --lan</code> printed, not by name.")
        presented = self._presented_key(url)
        if not presented or not hmac.compare_digest(presented, self.lan_token):
            # The only rate limit there is, and it is load-bearing: the key is
            # 40 bits, and a fifth of a second per attempt puts a guessing run
            # far past any sitting at this queue. Not a pointless sleep.
            time.sleep(0.2)
            return self._refuse("Open the link <code>review.py serve --lan</code> "
                                "printed &mdash; it carries the key.")
        if parse_qs(url.query).get("k"):
            # Set once, on the visit that carried the key. After that every
            # request has it, including <img src="/media/...">, which cannot
            # send a header -- which is why this is a cookie and not one.
            self._cookie = ("%s=%s; Path=/; Max-Age=2592000; SameSite=Lax"
                            % (rl.LAN_COOKIE, self.lan_token))
        return True

    def _refuse(self, why: str) -> bool:
        body = ("<!doctype html><meta charset=utf-8><title>Review queue</title>"
                "<body style=\"background:#14161a;color:#dde3ec;"
                "font:16px/1.6 ui-sans-serif,system-ui;padding:44px 24px\">"
                "<h1 style=\"font-size:19px\">Review queue &mdash; locked</h1>"
                "<p style=\"color:#8b95a5\">%s</p>" % why).encode("utf-8")
        self._send(401, body, "text/html; charset=utf-8")
        return False

    # -- routing ----------------------------------------------------------

    def do_GET(self):
        url = urlparse(self.path)
        path = url.path
        try:
            if not self._gate(url):
                return
            if path in ("/", "/index.html"):
                return self._send(200, self.page_path.read_bytes(), "text/html; charset=utf-8")
            if path == "/api/rev":
                # Sync state rides along on the poll the page already makes, so
                # a transport that has quietly stopped working becomes visible
                # without the page asking a second question.
                return self._json({"rev": rl.queue_revision(self.root),
                                   "sync": rl.sync_state(self.root),
                                   "pending_release": rl.pending_release_count(self.root)})
            if path == "/api/cards":
                return self._cards(parse_qs(url.query))
            if path.startswith("/media/"):
                return self._media(path[len("/media/"):])
            return self._error(404, "no route for %s" % path)
        except Exception as exc:  # a handler crash must not kill the server
            return self._error(500, "%s: %s" % (type(exc).__name__, exc))

    do_HEAD = do_GET

    def do_POST(self):
        url = urlparse(self.path)
        path = url.path
        try:
            if not self._gate(url):
                return
            if path == "/api/notify":
                # Release every pending verdict to the session that asked for
                # it. The server deliberately does not write to any socket: a
                # server on this machine cannot reach a cloud agent's container,
                # so delivery belongs to a watcher inside the agent's own
                # process tree. This only publishes; sync carries it.
                result = rl.release_verdicts(self.root)
                sync = rl.sync_now(self.root)
                result["synced"] = bool(sync.get("ok"))
                return self._json(result)
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
            "pending_release": rl.pending_release_count(self.root),
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
                          {"Cache-Control": "private, max-age=31536000, immutable"})


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
          sync_interval: float = 60.0, lan: bool = False) -> int:
    page = Path(__file__).resolve().parent / "review_page.html"
    if not page.is_file():
        page = root / "bin" / "review_page.html"
    if not page.is_file():
        raise SystemExit("review_page.html not found next to review_server.py")

    # No flag, no token file: a token is created the first time somebody asks
    # to be reachable, never as a side effect of the ordinary desktop serve.
    token = rl.lan_token(root) if lan else ""
    bind = "0.0.0.0" if lan else "127.0.0.1"

    handler = type("BoundReviewHandler", (ReviewHandler,),
                   {"root": root, "page_path": page, "lan_token": token})
    try:
        httpd = ThreadingHTTPServer((bind, port), handler)
    except OSError as exc:
        raise SystemExit(
            "cannot bind %s:%d (%s). Another review server is probably "
            "already running -- open http://127.0.0.1:%d/ , or pass --port."
            % (bind, port, exc, port)
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
    print("open %s" % url)
    if lan:
        _print_lan_banner(port, token)
    sys.stdout.flush()
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


def _print_lan_banner(port: int, token: str) -> None:
    """Say what was just exposed, and hand over a link that is tappable.

    No QR encoder: a stdlib one is Reed-Solomon and mask selection for a URL
    that gets pasted once a month. The token is short enough to retype, and
    the clipboard copy means usually nobody has to.
    """
    addr = rl.lan_address()
    if not addr:
        print("\n  --lan: this machine has no address on a local network "
              "(loopback or link-local only).\n         The listener is up, but "
              "no phone can reach it from here.")
        return
    url = "http://%s:%d/?k=%s" % (addr, port, token)
    print("\n  phone:  %s" % url)
    print("          anything on this Wi-Fi can reach the queue with that key.")
    print("          key lives in the queue root as %s; delete it to rotate."
          % rl.LAN_TOKEN_FILE)
    if rl.copy_to_clipboard(url):
        print("          (copied to the clipboard -- paste it to yourself and tap it)")



if __name__ == "__main__":
    import argparse
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--port", type=int, default=rl.DEFAULT_PORT)
    ap.add_argument("--open", action="store_true")
    ap.add_argument("--sync-interval", type=float, default=60.0)
    a = ap.parse_args()
    sys.exit(serve(rl.review_root(), a.port, a.open, a.sync_interval))
