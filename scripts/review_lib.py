"""Shared storage layer for the agent -> human visual review queue.

Design notes that are load-bearing; read before changing anything here.

*One file per card, one writer per file.* There is no index to rewrite and no
lock. Several agents, in several worktrees, post concurrently and cannot lose
each other's work because they never touch the same path. The three namespaces
below exist for the same reason -- so the posting agent, the server and the
retrieving agent each own their own file:

    cards/<id>.json      written once by the posting agent, never mutated
    responses/<id>.json  written only by the server, when the owner answers
    seen/<id>.json       touched by the posting agent once it has read a response
    media/<id>/...       artifact bytes, copied in at post time

Every write goes through `write_json_atomic` (tmp + os.replace), so a killed
process cannot leave a half-parsed card behind. A reader therefore never has to
defend against partial files -- if it is there, it is complete.
"""

from __future__ import annotations

import json
import os
import random
import shutil
import subprocess
import time
from datetime import datetime, timezone
from pathlib import Path

# Kept in one place because the CLI, the server and the skill doc all quote it.
DEFAULT_PORT = 7373
ROOT_ENV = "PIXEL_PHYSICS_REVIEW_DIR"
SOCKET_ENV = "CLAUDE_CODE_MESSAGING_SOCKET"
TOKEN_ENV = "CLAUDE_CODE_MESSAGING_TOKEN"
SESSION_ENV = "CLAUDE_CODE_SESSION_ID"

KINDS = ("single", "before_after", "ab", "gallery", "frames")


# --------------------------------------------------------------------------
# Where the queue lives
# --------------------------------------------------------------------------

def review_root(create: bool = True) -> Path:
    """The queue directory shared by every worktree of this clone.

    `git rev-parse --git-common-dir` resolves to the *main* clone's .git from
    inside any linked worktree, which is exactly the "one shared root, no
    configuration" property this needs. It is inside .git, so it is never
    committed and needs no .gitignore entry.

    --path-format=absolute is not optional: without it git happily returns the
    relative ".git", which silently gives every worktree its own private queue.
    That failure looks like "my card vanished" and is very hard to read back
    from the symptom, so it is ruled out here rather than guarded downstream.
    """
    env = os.environ.get(ROOT_ENV)
    if env:
        root = Path(env).expanduser().resolve()
    else:
        try:
            out = subprocess.run(
                ["git", "rev-parse", "--path-format=absolute", "--git-common-dir"],
                capture_output=True, text=True, check=True,
            ).stdout.strip()
        except (subprocess.CalledProcessError, FileNotFoundError) as exc:
            raise RuntimeError(
                "not inside a git repository, and %s is not set -- cannot locate "
                "the shared review queue" % ROOT_ENV
            ) from exc
        if not out:
            raise RuntimeError("git returned an empty --git-common-dir")
        root = Path(os.path.realpath(out)) / "pixel-physics-review"

    if create:
        for sub in ("cards", "responses", "seen", "media", "bin",
                    "outbox", "delivered"):
            (root / sub).mkdir(parents=True, exist_ok=True)
    return root


def cards_dir(root: Path) -> Path:
    return root / "cards"


def responses_dir(root: Path) -> Path:
    return root / "responses"


def seen_dir(root: Path) -> Path:
    return root / "seen"


def media_dir(root: Path) -> Path:
    return root / "media"


# --------------------------------------------------------------------------
# Atomic IO
# --------------------------------------------------------------------------

def write_json_atomic(path: Path, payload: dict) -> None:
    """Write JSON so that readers see either the old file or the whole new one.

    The temp file is created in the same directory as the target because
    os.replace is only atomic within a filesystem. On Windows os.replace is
    also atomic and overwrites, which plain os.rename does not.
    """
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.parent / ("." + path.name + ".tmp%d" % os.getpid())
    data = json.dumps(payload, indent=2, ensure_ascii=False)
    with open(tmp, "w", encoding="utf-8") as fh:
        fh.write(data)
        fh.flush()
        os.fsync(fh.fileno())
    os.replace(tmp, path)


def read_json(path: Path):
    try:
        with open(path, "r", encoding="utf-8") as fh:
            return json.load(fh)
    except FileNotFoundError:
        return None
    except (json.JSONDecodeError, OSError):
        # Cannot happen for a completed write (see write_json_atomic), so this
        # means a foreign file landed in the directory. Skipping beats taking
        # the whole queue down over one bad entry.
        return None


# --------------------------------------------------------------------------
# Identity
# --------------------------------------------------------------------------

def new_id() -> str:
    """Sortable, collision-free without coordination between agents.

    Milliseconds are in the stamp, not just seconds: the queue is sorted by id
    to get "newest first", and two cards posted in the same second would
    otherwise order by their random suffix -- which reads as the queue randomly
    reshuffling a burst of related cards.
    """
    stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S.%f")[:-3]
    return "%sZ-%06x" % (stamp.replace(".", ""), random.getrandbits(24))


def utc_now() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def origin_info() -> dict:
    """Which worktree/branch/commit a card came from.

    Best-effort: a card posted from a detached or non-git cwd is still a valid
    card, so every probe here degrades to None rather than failing the post.
    """
    def git(*args):
        try:
            return subprocess.run(
                ["git", *args], capture_output=True, text=True, check=True
            ).stdout.strip() or None
        except (subprocess.CalledProcessError, FileNotFoundError):
            return None

    return {
        "worktree": git("rev-parse", "--show-toplevel"),
        "branch": git("rev-parse", "--abbrev-ref", "HEAD"),
        "commit": git("rev-parse", "--short", "HEAD"),
        # Only an *explicitly* set agent name counts as identity. Falling back
        # to $USER looked equivalent and is not: every agent on one machine runs
        # as the same user, so `inbox --mine` would have matched every card in
        # the queue and quietly stopped being a filter at all.
        "agent": os.environ.get("PIXEL_PHYSICS_REVIEW_AGENT") or None,
        # Which Claude Code session asked. This is the address a verdict is
        # released to; without it a card is answerable but not notifiable.
        "session_id": os.environ.get(SESSION_ENV) or None,
        "user": os.environ.get("USER") or os.environ.get("USERNAME"),
        "cwd": str(Path.cwd()),
    }


# --------------------------------------------------------------------------
# Media
# --------------------------------------------------------------------------

def copy_media(root: Path, card_id: str, src: str, index: int) -> str:
    """Copy an artifact into the queue and return its queue-relative path.

    Copying rather than referencing is the whole point: the producing worktree
    is disposable, and a card whose image lives in a deleted `target/` is a
    card nobody can judge. Cost is a few hundred KB per card.
    """
    src_path = Path(src).expanduser()
    if not src_path.is_file():
        raise FileNotFoundError("artifact not found: %s" % src_path)

    dest_dir = media_dir(root) / card_id
    dest_dir.mkdir(parents=True, exist_ok=True)
    # Prefix with an index so two artifacts with the same basename (a.png from
    # two different worktrees) cannot clobber each other.
    dest_name = "%03d-%s" % (index, src_path.name)
    dest = dest_dir / dest_name
    shutil.copyfile(src_path, dest)
    return "%s/%s" % (card_id, dest_name)


def image_size(path: Path):
    """(width, height) from a PNG or GIF header, or None if it is neither.

    Header bytes only -- no decode, no dependency. Enough to bounds-check a
    declared focus rectangle at post time, where the agent can still fix it,
    rather than letting a bad rect reach the page as a blank viewport.
    """
    try:
        with open(path, "rb") as fh:
            head = fh.read(32)
    except OSError:
        return None
    if head[:8] == b"\x89PNG\r\n\x1a\n" and head[12:16] == b"IHDR":
        return int.from_bytes(head[16:20], "big"), int.from_bytes(head[20:24], "big")
    if head[:6] in (b"GIF87a", b"GIF89a"):
        return int.from_bytes(head[6:8], "little"), int.from_bytes(head[8:10], "little")
    return None


def gif_frames(path: Path):
    """Number of frames in a GIF, or None if it is not one.

    Exists because a single-frame GIF is indistinguishable from an animation by
    name, size or header, and is exactly what you get from
    `filmstrip out=x.gif` *without* `gif=1`: the contact-sheet branch calls
    `image::save_buffer`, which picks its encoder from the file extension and
    writes the whole sheet as one still frame. No error, a plausible .gif, and
    an agent reporting that it posted an animation.
    """
    try:
        d = path.read_bytes()
    except OSError:
        return None
    if d[:6] not in (b"GIF87a", b"GIF89a"):
        return None
    gct = 3 * (2 ** ((d[10] & 7) + 1)) if d[10] & 0x80 else 0
    i, n = 13 + gct, 0
    while i < len(d):
        b = d[i]
        if b == 0x3B:
            break
        if b == 0x21:                       # extension block
            i += 2
            while i < len(d) and d[i]:
                i += d[i] + 1
            i += 1
        elif b == 0x2C:                     # image descriptor -- one per frame
            n += 1
            lf = d[i + 9]
            i += 10 + (3 * (2 ** ((lf & 7) + 1)) if lf & 0x80 else 0) + 1
            while i < len(d) and d[i]:
                i += d[i] + 1
            i += 1
        else:
            break
    return n


# --------------------------------------------------------------------------
# Queue reads
# --------------------------------------------------------------------------

def load_cards(root: Path) -> list:
    """Every card, newest first, each with its response attached if answered.

    Deliberately re-reads the directory on every call rather than caching:
    other processes are writing into it continuously, and at this scale a
    listdir is cheaper than any correct invalidation scheme would be.
    """
    out = []
    cdir = cards_dir(root)
    if not cdir.is_dir():
        return out
    for path in cdir.glob("*.json"):
        card = read_json(path)
        if not isinstance(card, dict) or "id" not in card:
            continue
        card["response"] = read_json(responses_dir(root) / path.name)
        card["seen"] = (seen_dir(root) / path.name).exists()
        card["status"] = card_status(card)
        card["notify"] = notify_state(root, card)
        out.append(card)
    out.sort(key=lambda c: c.get("id", ""), reverse=True)
    return out


def load_card(root: Path, card_id: str):
    card = read_json(cards_dir(root) / ("%s.json" % card_id))
    if card is None:
        return None
    card["response"] = read_json(responses_dir(root) / ("%s.json" % card_id))
    card["seen"] = (seen_dir(root) / ("%s.json" % card_id)).exists()
    card["status"] = card_status(card)
    return card


def card_status(card: dict) -> str:
    """Status is derived, never stored.

    Storing it would mean two processes mutating one file; deriving it from the
    presence of a response (plus an archive flag the server owns) keeps the
    single-writer rule intact.
    """
    resp = card.get("response")
    if resp and resp.get("archived"):
        return "archived"
    if resp and resp.get("answered_at"):
        return "answered"
    return "open"


def load_boards(cards: list) -> list:
    counts = {}
    for card in cards:
        board = card.get("board") or "inbox"
        entry = counts.setdefault(board, {"board": board, "open": 0, "total": 0})
        entry["total"] += 1
        if card["status"] == "open":
            entry["open"] += 1
    return sorted(counts.values(), key=lambda b: (-b["open"], b["board"]))


def queue_revision(root: Path) -> str:
    """Cheap change token: newest mtime plus file count across both namespaces.

    mtime alone is not enough -- deleting a file can leave the max unchanged --
    and hashing every card would make the 2s poll cost real work.
    """
    newest = 0.0
    count = 0
    for sub in ("cards", "responses"):
        d = root / sub
        if not d.is_dir():
            continue
        for path in d.glob("*.json"):
            try:
                newest = max(newest, path.stat().st_mtime)
            except OSError:
                continue
            count += 1
    return "%.6f-%d" % (newest, count)


# --------------------------------------------------------------------------
# Queue writes
# --------------------------------------------------------------------------

def save_card(root: Path, card: dict) -> Path:
    path = cards_dir(root) / ("%s.json" % card["id"])
    write_json_atomic(path, card)
    return path


def save_response(root: Path, card_id: str, response: dict) -> Path:
    path = responses_dir(root) / ("%s.json" % card_id)
    write_json_atomic(path, response)
    return path


def mark_seen(root: Path, card_id: str) -> None:
    write_json_atomic(
        seen_dir(root) / ("%s.json" % card_id),
        {"seen_at": utc_now(), "by": origin_info().get("agent")},
    )


def prune_orphan_media(root: Path) -> list:
    """Drop media directories with no card.

    A post killed between copying its artifacts and writing its card leaves
    one of these behind. It is invisible in the queue -- nothing references it
    -- so this is housekeeping, not repair, and it deliberately never touches
    media belonging to an answered or archived card: a verdict months old is
    only re-readable if its picture is still there.
    """
    removed = []
    mdir = media_dir(root)
    if not mdir.is_dir():
        return removed
    for entry in mdir.iterdir():
        if not entry.is_dir():
            continue
        if (cards_dir(root) / ("%s.json" % entry.name)).exists():
            continue
        shutil.rmtree(entry, ignore_errors=True)
        removed.append(entry.name)
    return removed


def wait_for_response(root: Path, card_id: str, timeout: float, interval: float = 1.0):
    """Block until the owner answers, or give up.

    Polls the file directly rather than talking to the server, so a wait
    survives the server being restarted underneath it -- and works even if the
    owner has not started the server yet when the card is posted.
    """
    deadline = time.monotonic() + timeout
    path = responses_dir(root) / ("%s.json" % card_id)
    while True:
        resp = read_json(path)
        if resp and resp.get("answered_at"):
            return resp
        if time.monotonic() >= deadline:
            return None
        time.sleep(min(interval, max(0.05, deadline - time.monotonic())))


# --------------------------------------------------------------------------
# Cross-machine transport
# --------------------------------------------------------------------------
#
# The queue above is a directory, which makes it shared by every worktree of a
# clone -- and invisible to a session running on a different machine. A Claude
# Code web session has its own clone in its own container, so its cards were
# written to a disk the owner could never read. That was not a bug in the
# storage; it was the storage being the wrong shape for where agents actually
# run.
#
# The one thing every clone already shares is the git remote, so that is the
# transport. An orphan branch `review-queue` carries the same cards/, media/
# and responses/ layout, sharing no history with main -- so it never shows up
# in a log, diff or merge of the project, and can be squashed or reset when it
# grows without touching project history.
#
# The property that made the local queue loss-proof does the work again here:
# one file per card, one writer per file. Two agents pushing different cards
# touch disjoint paths, so a concurrent push is a fast-forward or a trivially
# auto-resolving merge. No lock, no coordination.

SYNC_BRANCH = "review-queue"
NO_SYNC_ENV = "PIXEL_PHYSICS_REVIEW_NO_SYNC"


class SyncError(RuntimeError):
    """Transport failed. Never fatal to a post -- the card is already on disk."""


def sync_disabled() -> bool:
    return os.environ.get(NO_SYNC_ENV, "").strip() not in ("", "0", "false", "no")


def _git(args, cwd, check=True, timeout=120):
    proc = subprocess.run(["git", *args], cwd=str(cwd), capture_output=True,
                          text=True, timeout=timeout)
    if check and proc.returncode != 0:
        raise SyncError("git %s failed: %s" % (" ".join(args),
                                               (proc.stderr or proc.stdout).strip()[:300]))
    return proc


def remote_url(cwd=None):
    """The push URL of the clone this agent is working in, or None."""
    try:
        proc = subprocess.run(["git", "remote", "get-url", "origin"],
                              cwd=str(cwd) if cwd else None,
                              capture_output=True, text=True, timeout=30)
    except (OSError, subprocess.SubprocessError):
        return None
    return proc.stdout.strip() if proc.returncode == 0 and proc.stdout.strip() else None


def sync_dir(root: Path) -> Path:
    return root / "sync"


def sync_state(root: Path) -> dict:
    return read_json(root / "sync-state.json") or {"last_ok": None, "last_error": None,
                                                   "last_attempt": None, "skipped": None}


def _record_sync(root: Path, ok: bool, detail: dict) -> None:
    state = sync_state(root)
    state["last_attempt"] = utc_now()
    if ok:
        state["last_ok"] = utc_now()
        state["last_error"] = None
    else:
        state["last_error"] = detail.get("error")
    state.update({k: v for k, v in detail.items() if k != "error"})
    write_json_atomic(root / "sync-state.json", state)


def _ensure_sync_repo(root: Path, url: str) -> Path:
    """A standalone one-branch repo used only as a staging area.

    Deliberately *not* a `git worktree` of the session's clone: a worktree
    would appear in `git worktree list`, and this directory lives inside
    `.git/`, which is no place to register one. A standalone repo also cannot
    touch the session's real working tree no matter what it does -- which
    matters, because sync runs on a timer in the background.
    """
    d = sync_dir(root)
    if not (d / ".git").is_dir():
        d.mkdir(parents=True, exist_ok=True)
        _git(["init", "-q"], d)
        _git(["config", "user.email", "review-queue@localhost"], d)
        _git(["config", "user.name", "Pixel Physics review queue"], d)
        # Nothing here is ever hand-edited, so line-ending rewriting could only
        # corrupt a PNG. Off explicitly rather than trusting the global config.
        _git(["config", "core.autocrlf", "false"], d)
    existing = _git(["remote"], d).stdout.split()
    if "origin" in existing:
        _git(["remote", "set-url", "origin", url], d)
    else:
        _git(["remote", "add", "origin", url], d)
    return d


def _remote_branch_head(d: Path):
    """Prove the remote is reachable, and say whether the branch exists yet.

    These are two different things and a plain `git fetch` conflates them: it
    fails identically for "no such branch" and "no such remote". Conflating
    them made sync claim success against a remote that did not exist -- it took
    the first-sync path, rebuilt a tree identical to the last commit, saw
    nothing to push and reported ok. A card posted from an offline agent was
    then reported as delivered, which is the exact failure this whole change
    set out to fix, reappearing one layer down.

    `ls-remote` separates them: a non-zero exit is an unreachable remote, empty
    output is a reachable remote with no such branch.
    """
    proc = _git(["ls-remote", "--heads", "origin", SYNC_BRANCH], d, check=False)
    if proc.returncode != 0:
        raise SyncError("remote unreachable: %s"
                        % (proc.stderr or proc.stdout).strip()[:200])
    out = proc.stdout.strip()
    return out.split()[0] if out else None


def _reset_to_remote(d: Path) -> bool:
    """Point the staging tree at the branch's current remote tip.

    Returns False when the branch does not exist yet (first ever sync), in
    which case the tree is emptied and the first commit creates it. Raises if
    the remote cannot be reached at all -- never silently treats that as a
    first sync.
    """
    if _remote_branch_head(d) is None:
        _git(["checkout", "-q", "--orphan", SYNC_BRANCH], d, check=False)
        _git(["rm", "-rq", "--cached", "."], d, check=False)
        for entry in d.iterdir():
            if entry.name != ".git":
                shutil.rmtree(entry, ignore_errors=True) if entry.is_dir() else entry.unlink()
        return False
    _git(["fetch", "-q", "origin", SYNC_BRANCH], d)
    _git(["checkout", "-q", "-B", SYNC_BRANCH, "FETCH_HEAD"], d)
    _git(["reset", "-q", "--hard", "FETCH_HEAD"], d)
    return True


def _newer_response(a: dict, b: dict) -> dict:
    """Later `answered_at` wins when both sides hold a response for one card.

    Reopen-and-reanswer is the only way two responses exist for one id, and the
    later answer is by definition the owner's current view.
    """
    if not a:
        return b
    if not b:
        return a
    return a if (a.get("answered_at") or "") >= (b.get("answered_at") or "") else b


def _merge_dirs(src: Path, dst: Path, kind: str) -> list:
    """Copy what the destination is missing. Returns the names copied.

    Cards and media are immutable once written -- ids are unique and nothing
    rewrites them -- so "copy if absent" is exactly right and costs one stat
    per file. Responses are the one mutable thing, and go through
    `_newer_response`.
    """
    moved = []
    if not src.is_dir():
        return moved
    dst.mkdir(parents=True, exist_ok=True)
    for entry in sorted(src.iterdir()):
        target = dst / entry.name
        if kind == "responses":
            incoming = read_json(entry)
            current = read_json(target)
            winner = _newer_response(incoming, current)
            if winner is not incoming or current is None:
                if current is None or winner != current:
                    write_json_atomic(target, winner)
                    moved.append(entry.name)
            continue
        if target.exists():
            continue
        if entry.is_dir():
            shutil.copytree(entry, target)
        else:
            shutil.copyfile(entry, target)
        moved.append(entry.name)
    return moved


def sync_now(root: Path, cwd=None, attempts: int = 4) -> dict:
    """Exchange cards and verdicts with the remote. Best effort, never fatal.

    Each attempt re-applies our local files on top of the branch's *current*
    tip rather than trying to merge diverged trees. With one file per card that
    converges: whatever another agent pushed in the meantime is imported first,
    ours is layered on, and the push is a fast-forward.
    """
    if sync_disabled():
        result = {"ok": False, "skipped": "disabled via %s" % NO_SYNC_ENV,
                  "pulled": [], "pushed": []}
        _record_sync(root, False, dict(result, error=None))
        return result
    url = remote_url(cwd)
    if not url:
        result = {"ok": False,
                  "skipped": "no git remote 'origin' in %s — local-only queue"
                             % (cwd or Path.cwd()),
                  "pulled": [], "pushed": []}
        _record_sync(root, False, dict(result, error=None))
        return result

    last_error = None
    for attempt in range(attempts):
        try:
            d = _ensure_sync_repo(root, url)
            _reset_to_remote(d)

            # Remote -> local. This is the half that makes a cloud agent's card
            # show up in the owner's page.
            pulled = []
            for kind in ("cards", "media", "responses"):
                pulled += _merge_dirs(d / kind, root / kind, kind)

            # Local -> remote.
            pushed = []
            for kind in ("cards", "media", "responses"):
                pushed += _merge_dirs(root / kind, d / kind, kind)

            _git(["add", "-A"], d)
            if not _git(["status", "--porcelain"], d).stdout.strip():
                result = {"ok": True, "pulled": pulled, "pushed": [], "branch": SYNC_BRANCH}
                _record_sync(root, True, dict(result))
                return result

            _git(["commit", "-q", "-m",
                  "queue sync: %d card/response files" % len(pushed)], d)
            proc = _git(["push", "-q", "origin", "%s:%s" % (SYNC_BRANCH, SYNC_BRANCH)],
                        d, check=False)
            if proc.returncode == 0:
                result = {"ok": True, "pulled": pulled, "pushed": pushed,
                          "branch": SYNC_BRANCH}
                _record_sync(root, True, dict(result))
                return result
            # Someone else pushed between our fetch and our push. Loop: their
            # cards get imported on the next pass and ours re-applied on top.
            last_error = (proc.stderr or proc.stdout).strip()[:300]
        except (SyncError, OSError, subprocess.SubprocessError) as exc:
            last_error = str(exc)[:300]

    result = {"ok": False, "error": last_error or "sync failed", "pulled": [], "pushed": []}
    _record_sync(root, False, dict(result))
    return result


# --------------------------------------------------------------------------
# Telling the agent its verdict is in
# --------------------------------------------------------------------------
#
# Answering a card told nobody, so the owner had to visit each agent by hand.
#
# The server never writes to a session socket. It only *releases* verdicts into
# `outbox/<session_id>/`; the agent's own watcher delivers them. That split is
# deliberate on two counts. A server on the owner's machine cannot reach a cloud
# agent's container at all, so a server-writes-directly design would work for
# local agents and silently not for the rest of the fleet -- releasing through
# the queue, which already syncs over git, makes both identical. And a process
# the agent itself spawned carries the *child* token, which bypasses the inbound
# gate; a stranger process is subject to `crossSessionInbound`, whose default is
# mode-parity and would be *held* against a session running in bypassPermissions.
#
# Releases are batched by the owner pressing a button, not fired per card: six
# verdicts for one agent must cost one wake-up turn, not six. That wake replaces
# the turn the owner would otherwise spend saying "I reviewed your card", so
# batched it is roughly free and un-batched it multiplies with every card.

MAX_FRAME = 1024 * 1024          # the reader destroys the connection past 1 MiB


def outbox_dir(root: Path) -> Path:
    return root / "outbox"


def delivered_dir(root: Path) -> Path:
    return root / "delivered"


def session_id() -> str:
    return os.environ.get(SESSION_ENV) or ""


def notify_state(root: Path, card: dict) -> str:
    """pending -> released -> delivered. Derived, never stored on the card."""
    if not (card.get("response") or {}).get("answered_at"):
        return "unanswered"
    cid = card["id"]
    if (delivered_dir(root) / ("%s.json" % cid)).exists():
        return "delivered"
    sid = (card.get("origin") or {}).get("session_id")
    if sid and (outbox_dir(root) / sid / ("%s.json" % cid)).exists():
        return "released"
    return "pending"


def verdict_digest(card: dict) -> str:
    """One line per card. The ping only has to make the agent look; `inbox`
    remains the source of truth for pins and the full comment."""
    r = card.get("response") or {}
    bits = []
    if r.get("choice_label"):
        bits.append("chose %s" % r["choice_label"])
    elif r.get("choice") == -1:
        bits.append("chose neither")
    if r.get("rating") is not None:
        bits.append("rating %s" % r["rating"])
    comment = (r.get("comment") or "").strip().splitlines()
    if comment:
        bits.append('"%s"' % comment[0][:160])
    if r.get("annotations"):
        bits.append("%d pin(s)" % len(r["annotations"]))
    return "%s — %s" % (card.get("title", "(untitled)"), "; ".join(bits) or "answered")


def release_verdicts(root: Path) -> dict:
    """Group every pending verdict by the session that asked, and release it.

    One file per card under the asking session's directory: one writer, one
    file, so a second press cannot duplicate an entry and two presses racing
    cannot lose one -- the same property the rest of the queue relies on.
    """
    by_session, orphans = {}, []
    for card in load_cards(root):
        if notify_state(root, card) != "pending":
            continue
        sid = (card.get("origin") or {}).get("session_id")
        if not sid:
            # Visibly un-notifiable rather than silently skipped: the owner is
            # told which branches these came from so they can chase by hand.
            orphans.append({"id": card["id"], "title": card.get("title"),
                            "branch": (card.get("origin") or {}).get("branch")})
            continue
        by_session.setdefault(sid, []).append(card)

    released = {}
    for sid, cards in by_session.items():
        for card in cards:
            write_json_atomic(outbox_dir(root) / sid / ("%s.json" % card["id"]),
                              {"card_id": card["id"], "session_id": sid,
                               "released_at": utc_now(), "digest": verdict_digest(card)})
        released[sid] = [c["id"] for c in cards]
    return {"released": released, "orphans": orphans,
            "agents": len(released), "verdicts": sum(len(v) for v in released.values())}


def pending_release_count(root: Path) -> dict:
    """What the button should say before it is pressed."""
    agents, verdicts, orphans = set(), 0, 0
    for card in load_cards(root):
        if notify_state(root, card) != "pending":
            continue
        sid = (card.get("origin") or {}).get("session_id")
        if sid:
            agents.add(sid); verdicts += 1
        else:
            orphans += 1
    return {"agents": len(agents), "verdicts": verdicts, "orphans": orphans}


def take_outbox(root: Path, sid: str) -> list:
    d = outbox_dir(root) / sid
    if not d.is_dir():
        return []
    out = [read_json(f) for f in sorted(d.glob("*.json"))]
    return [e for e in out if e]


def mark_delivered(root: Path, entries: list, sid: str) -> None:
    """Delivered marker first, then drop the outbox entry.

    This order matters: a crash between the two re-delivers (harmless, the agent
    sees the digest twice) rather than losing the verdict entirely.
    """
    for e in entries:
        write_json_atomic(delivered_dir(root) / ("%s.json" % e["card_id"]),
                          {"card_id": e["card_id"], "delivered_at": utc_now(), "session_id": sid})
    for e in entries:
        try:
            (outbox_dir(root) / sid / ("%s.json" % e["card_id"])).unlink()
        except OSError:
            pass


def send_to_own_session(text: str) -> str:
    """Write one message into this session's inbox socket. Returns "" on success.

    The wire format is newline-delimited JSON: an auth frame first (optional on
    macOS/Linux, required on Windows, and it sets the sender class either way),
    then the message. `type` must be "user" and the text must live at
    `message.content` -- a frame like {"type":"message","text":...} has a valid
    string type, so it passes the type check and falls through to an
    unhandled-type branch that logs at debug level and does nothing. That shape
    failed silently twice before the protocol was read properly; the selftest
    asserts it still delivers nothing.

    priority "next" rather than "now": the verdict lands between turns instead
    of interrupting mid-work and making the agent re-establish where it was.
    """
    sock_path = os.environ.get(SOCKET_ENV)
    if not sock_path:
        return "%s is not set — not running inside a Claude Code session" % SOCKET_ENV
    frames = []
    token = os.environ.get(TOKEN_ENV)
    if token:
        frames.append({"type": "auth", "token": token})
    frames.append({"type": "user", "priority": "next",
                   "message": {"role": "user", "content": text}})
    payload = b"".join((json.dumps(f) + "\n").encode("utf-8") for f in frames)
    if max(len(json.dumps(f)) for f in frames) >= MAX_FRAME:
        return "message too large for one frame (%d byte cap)" % MAX_FRAME
    try:
        import socket as _socket
        s = _socket.socket(_socket.AF_UNIX, _socket.SOCK_STREAM)
        s.settimeout(10)
        s.connect(sock_path)
        s.sendall(payload)
        s.close()
    except OSError as exc:
        return "could not write to %s: %s" % (sock_path, exc)
    return ""


def live_sessions() -> list:
    """Every running Claude Code session, from a plain subprocess.

    Not needed to deliver -- the watcher writes to its own socket -- but it is
    how a card's recorded session id is checked against reality, and how the
    watcher knows its session has ended.
    """
    try:
        proc = subprocess.run(["claude", "agents", "--json"],
                              capture_output=True, text=True, timeout=60)
        return json.loads(proc.stdout) if proc.returncode == 0 else []
    except (OSError, ValueError, subprocess.SubprocessError):
        return []


# --------------------------------------------------------------------------
# Reaching the queue from a phone
#
# The server binds loopback by default and always will. `serve --lan` is the
# opt-in that puts it on the Wi-Fi, and the token below is the whole of what
# stands between the queue and everything else on that network.
# --------------------------------------------------------------------------

LAN_TOKEN_FILE = "lan-token"
LAN_COOKIE = "review_key"

# Crockford base32 minus I/L/O/U: no character pair anyone can confuse while
# typing it off a terminal into a phone, and no accidental words.
_TOKEN_ALPHABET = "0123456789ABCDEFGHJKMNPQRSTVWXYZ"
TOKEN_LEN = 8


def lan_token(root: Path, create: bool = True) -> str:
    """The shared key for non-loopback access, created once per queue.

    Eight characters is 40 bits. The threat model is somebody already on your
    Wi-Fi guessing it against a server that sleeps on every failure -- not an
    internet-facing brute force -- and a 64-hex token you cannot retype off a
    terminal is worse here, not better: it would be pasted somewhere insecure
    or the feature would go unused.
    """
    import secrets
    path = root / LAN_TOKEN_FILE
    try:
        existing = path.read_text(encoding="utf-8").strip()
        if existing:
            return existing
    except OSError:
        pass
    if not create:
        return ""
    token = "".join(secrets.choice(_TOKEN_ALPHABET) for _ in range(TOKEN_LEN))
    # Write restricted from the start: creating it 0644 and chmod-ing after
    # leaves a window in which any local user can read it.
    fd = os.open(str(path), os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
    try:
        os.write(fd, (token + "\n").encode("utf-8"))
    finally:
        os.close(fd)
    return token


def lan_address() -> str:
    """This machine's address on the local network, or "" if it has none.

    Connecting a UDP socket sends no packet -- it only asks the routing table
    which interface would be used to reach that address -- so this works with
    no network traffic and no name resolution. A loopback or link-local answer
    means there is no usable LAN address, and saying so beats printing a URL
    the phone cannot open.
    """
    import socket
    try:
        s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        try:
            s.connect(("8.8.8.8", 80))
            addr = s.getsockname()[0]
        finally:
            s.close()
    except OSError:
        return ""
    if addr.startswith("127.") or addr.startswith("169.254.") or addr == "0.0.0.0":
        return ""
    return addr


def copy_to_clipboard(text: str) -> bool:
    """Best effort, so the URL can be pasted into Messages and tapped."""
    for cmd in (["pbcopy"], ["wl-copy"], ["xclip", "-selection", "clipboard"]):
        try:
            proc = subprocess.run(cmd, input=text, text=True,
                                  capture_output=True, timeout=5)
            if proc.returncode == 0:
                return True
        except (OSError, subprocess.SubprocessError):
            continue
    return False
