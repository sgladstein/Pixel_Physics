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
        for sub in ("cards", "responses", "seen", "media", "bin"):
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
                                                   "last_attempt": None}


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
        return {"ok": False, "skipped": "disabled via %s" % NO_SYNC_ENV,
                "pulled": [], "pushed": []}
    url = remote_url(cwd)
    if not url:
        return {"ok": False, "skipped": "no git remote 'origin' — local-only queue",
                "pulled": [], "pushed": []}

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
