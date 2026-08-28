#!/usr/bin/env python3
"""What the prompt cache actually did, read off a session transcript.

Why this exists
---------------
`Reports/agent-strategy.md` §6 set `subagentPromptCacheTtl: "1h"` on arithmetic
whose two arms used opposite premises, and the review that caught it
(`Reports/pr89-review.md` §1) closed by saying the remaining question needed an
instrumented fan-out costing ~100k tokens: *do Claude Code's subagents share a
cache namespace with the parent and with each other?*

**They do not need one.** Claude Code already writes every turn's `usage` to
`~/.claude/projects/<escaped-cwd>/<session-id>.jsonl`, including
`cache_read_input_tokens` and a `cache_creation` split **by TTL**
(`ephemeral_5m_input_tokens` / `ephemeral_1h_input_tokens`), and it tags
sub-agent turns `isSidechain: true`. The measurement is a post-hoc read of a
file any fan-out produces for free. This is CLAUDE.md's own rule -- grep
`instruments.md` before building a harness -- applied to a harness that had not
been built yet.

The three questions, which are NOT one question
-----------------------------------------------
Conflating them is the exact error §6 made, so this tool reports them apart:

1. **NAMESPACE** -- do sub-agents share a cached prefix at all? Read the FIRST
   turn of each sidechain agent: a non-zero `cache_read` covering the shared
   prefix means yes. Requires agents launched **sequentially**, or the race in
   (2) masks the answer.
2. **RACE** -- launched concurrently by `parallel([...])`, how many miss? Count
   first-turn writes across agents that started within seconds of each other.
   A shared namespace still gives every agent a miss if they all start together,
   which is why (1) cannot be read off a concurrent run.
3. **TTL** -- which TTL did the writes use, and did entries survive a phase
   boundary? Read the `cache_creation` split and the gap between turns.

Only (3) needs waiting. (1) and (2) fall out of any fan-out already planned.

**(1) alone can void the lever.** If sub-agents do not share a namespace, then
`subagentPromptCacheTtl` can only ever help one agent across its own multi-turn
life -- never across a fan-out -- and §6's 12-agent arithmetic is void at any
TTL. That is the cheapest possible falsification and it is the first row printed.

What it has already established
-------------------------------
Measured 2026-08-28 on this repo's own review session, 108 main-chain turns:

* the instrument is **not blind** -- 103 of 107 turns after the first show a
  non-zero `cache_read`, and turn 2 reads back exactly the 86,569 tokens turns
  0-1 wrote;
* **every write in the main conversation is `ephemeral_1h`** -- 976,431 tokens
  at 1h against **0** at 5m. §6 read the main session's 1-hour TTL off a schema;
  it is now measured.

The sidechain half is unmeasured here only because that session spawned no
sub-agents. Run any fan-out and point this at its transcript.

Usage
-----
    python3 scripts/cacheprobe.py                 # newest transcript for this repo
    python3 scripts/cacheprobe.py <file.jsonl>    # a specific one
    python3 scripts/cacheprobe.py --list          # what transcripts exist
    python3 scripts/cacheprobe.py --selftest      # controls; no transcript needed
"""

import json
import os
import pathlib
import sys

PROJECTS = pathlib.Path.home() / ".claude" / "projects"


def transcripts(root=None):
    """Transcript files for this repo, newest first."""
    base = pathlib.Path(root) if root else PROJECTS
    if not base.is_dir():
        return []
    cwd = str(pathlib.Path(__file__).resolve().parent.parent)
    slug = cwd.replace("/", "-").replace("_", "-")
    cands = [d for d in base.iterdir() if d.is_dir() and d.name in (slug, cwd.replace("/", "-"))]
    if not cands:
        cands = [d for d in base.iterdir() if d.is_dir()]
    files = [f for d in cands for f in d.glob("*.jsonl")]
    return sorted(files, key=lambda f: f.stat().st_mtime, reverse=True)


def turns(path):
    """(is_sidechain, ts, read, write_5m, write_1h, agent) per assistant turn."""
    out = []
    with open(path, encoding="utf-8") as fh:
        for line in fh:
            try:
                e = json.loads(line)
            except Exception:
                continue
            if e.get("type") != "assistant":
                continue
            u = (e.get("message") or {}).get("usage") or {}
            cc = u.get("cache_creation") or {}
            out.append({
                "side": bool(e.get("isSidechain")),
                "ts": e.get("timestamp") or "",
                "read": u.get("cache_read_input_tokens") or 0,
                "w5": cc.get("ephemeral_5m_input_tokens") or 0,
                "w1": cc.get("ephemeral_1h_input_tokens") or 0,
                # Sub-agent turns are grouped by their parent tool_use id when
                # present; fall back to the leaf uuid chain so a transcript
                # without it still separates agents rather than pooling them.
                "agent": e.get("parentUuid") if e.get("isSidechain") else None,
            })
    return out


def control(rows):
    """Is this transcript able to show caching at all? Print the verdict.

    A run where nothing was cached and a run where the FIELD is dead look
    identical in every summary below, so this is read first. CLAUDE.md: before
    citing an instrument, construct the case whose answer you know is non-zero.
    """
    main = [r for r in rows if not r["side"]]
    if len(main) < 3:
        print("  CONTROL INCONCLUSIVE -- fewer than 3 main-chain turns to check")
        return False
    hits = sum(1 for r in main[1:] if r["read"] > 0)
    if hits == 0:
        print(f"  CONTROL FAILED -- 0 of {len(main) - 1} main-chain turns after the")
        print("    first show any cache read. Either nothing cached, or the field is")
        print("    dead. Do NOT read the sidechain rows below as evidence of anything.")
        return False
    print(f"  control passes -- {hits}/{len(main) - 1} main-chain turns after the first")
    print("    show a cache read, so the field moves and the rows below mean something")
    return True


def report(path):
    rows = turns(path)
    print(f"cacheprobe: {path}")
    print(f"  {len(rows)} assistant turns "
          f"({sum(1 for r in rows if r['side'])} sidechain)\n")

    ok = control(rows)
    print()

    main = [r for r in rows if not r["side"]]
    side = [r for r in rows if r["side"]]

    for label, group in (("main conversation", main), ("sub-agents", side)):
        if not group:
            print(f"  {label}: no turns")
            continue
        w5 = sum(r["w5"] for r in group)
        w1 = sum(r["w1"] for r in group)
        rd = sum(r["read"] for r in group)
        print(f"  {label}: writes 5m={w5:,}  1h={w1:,}  reads={rd:,}")
        if w5 or w1:
            ttl = "1h" if w1 > w5 else "5m"
            share = 100 * max(w1, w5) / (w1 + w5)
            print(f"    -> writes are {share:.0f}% {ttl}. This is the TTL actually in use,")
            print("       not the one a settings schema describes.")
    print()

    if not side:
        print("  (3) TTL: answered for the main conversation above.")
        print("  (1) NAMESPACE and (2) RACE: NOT ANSWERABLE -- this session spawned no")
        print("      sub-agents. Run a fan-out and re-point this at its transcript;")
        print("      no extra agent spend is needed, the transcript is a byproduct.")
        return 0 if ok else 1

    # (1) namespace: first turn of each distinct sidechain agent.
    firsts, seen = [], set()
    for r in side:
        key = r["agent"]
        if key not in seen:
            seen.add(key)
            firsts.append(r)
    warm = sum(1 for r in firsts if r["read"] > 0)
    print(f"  (1) NAMESPACE: {warm}/{len(firsts)} sub-agents read a cached prefix on")
    print("      their first turn.")
    if warm == 0:
        print("      -> NO SHARING. subagentPromptCacheTtl cannot help a fan-out at any")
        print("         value; agent-strategy.md §6's 12-agent arithmetic is void.")
    else:
        print("      -> they share. The TTL question is live; read (2) before acting,")
        print("         because a concurrent launch misses even in a shared namespace.")

    # (2) race: agents whose first turn started within 60s of the earliest.
    stamps = [r["ts"] for r in firsts if r["ts"]]
    if stamps:
        t0 = min(stamps)
        burst = [r for r in firsts if r["ts"] and r["ts"][:19] <= _plus60(t0)]
        missed = sum(1 for r in burst if r["read"] == 0)
        print(f"  (2) RACE: {len(burst)} sub-agents started within ~60s of each other;")
        print(f"      {missed} of them missed the cache and wrote their own copy.")
        if missed > 1:
            print("      -> concurrent launch races. A 1h TTL makes each of those writes")
            print("         cost 2x instead of 1.25x, so raising the TTL COSTS money here.")
            print("         The fix is to warm the prefix before fanning out.")
    return 0 if ok else 1


def _plus60(ts):
    """ts + 60s, string-compared. Crude on purpose: the burst/no-burst gap in a
    fan-out is seconds against minutes, so second-level precision is ample and a
    datetime dependency is not worth it."""
    try:
        from datetime import datetime, timedelta
        return (datetime.fromisoformat(ts.replace("Z", "+00:00"))
                + timedelta(seconds=60)).isoformat()[:19]
    except Exception:
        return ts[:19]


def selftest():
    """Controls over the parser and over the blindness check, in milliseconds.

    Drives the real functions on synthetic transcripts -- it does not
    re-implement the predicates. `scripts/lanecheck.py` shipped a selftest that
    asserted `x > y` inline, proved Python's `>` works, and passed with the
    check it guarded gutted to `return []`. That is the mistake this avoids.
    """
    import tempfile
    bad = 0

    def write(tmp, entries):
        p = pathlib.Path(tmp) / "t.jsonl"
        p.write_text("\n".join(json.dumps(e) for e in entries), encoding="utf-8")
        return p

    def turn(read=0, w5=0, w1=0, side=False):
        return {"type": "assistant", "isSidechain": side, "timestamp": "2026-08-28T00:00:00Z",
                "message": {"usage": {"cache_read_input_tokens": read,
                                      "cache_creation": {"ephemeral_5m_input_tokens": w5,
                                                         "ephemeral_1h_input_tokens": w1}}}}

    with tempfile.TemporaryDirectory() as d:
        # parser: counts turns, splits chains, sums by TTL
        p = write(d, [turn(w1=100), turn(read=100), turn(read=100, side=True), {"type": "user"}])
        rows = turns(p)
        if len(rows) == 3 and sum(r["side"] for r in rows) == 1 \
                and sum(r["w1"] for r in rows) == 100 and sum(r["read"] for r in rows) == 200:
            print("cacheprobe: parser control -- turns, chain split and TTL sums correct")
        else:
            print(f"cacheprobe: PARSER CONTROL FAILED -- {rows}"); bad = 1

        # blindness check: must FAIL on an all-cold transcript, PASS on a warm one
        cold = turns(write(d, [turn(w1=10), turn(w1=10), turn(w1=10)]))
        warm = turns(write(d, [turn(w1=10), turn(read=10), turn(read=10)]))
        import io, contextlib
        with contextlib.redirect_stdout(io.StringIO()):
            cold_ok, warm_ok = control(cold), control(warm)
        if cold_ok is False and warm_ok is True:
            print("cacheprobe: blindness control -- an all-cold transcript is refused,")
            print("            a warm one is accepted")
        else:
            print(f"cacheprobe: BLINDNESS CONTROL FAILED -- cold={cold_ok} warm={warm_ok}")
            bad = 1

    live = transcripts()
    print(f"cacheprobe: {len(live)} live transcript(s) for this repo"
          f"{' -- run without --selftest to read the newest' if live else ''}")
    return bad


def main():
    args = sys.argv[1:]
    if "--selftest" in args:
        return selftest()
    if "--list" in args:
        for f in transcripts():
            print(f"  {f}  ({f.stat().st_size:,} B)")
        return 0
    paths = [a for a in args if not a.startswith("-")]
    if paths:
        return report(paths[0])
    live = transcripts()
    if not live:
        print("cacheprobe: no transcript found under ~/.claude/projects/")
        print("  Claude Code writes one per session; this needs to run on the machine")
        print("  that ran the agents. --selftest works anywhere.")
        return 1
    return report(live[0])


if __name__ == "__main__":
    sys.exit(main())
