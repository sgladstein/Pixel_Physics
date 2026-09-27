#!/usr/bin/env python3
"""What each sub-agent actually spent, and on what -- read off its transcript.

Why this exists
---------------
`scripts/contextbudget.py` prices what an agent pays *before it starts* (the
always-loaded files) and `scripts/cacheprobe.py` says what the prompt cache did
across a whole session. Neither says, per agent, **where the rest went** -- and
the rest is most of it. Measured 2026-09-27 over 13 workflow and background
agents (`Reports/agent-efficiency-2026-09-27.md`):

* every agent began at **~78k tokens** of context before reading its task:
  system prompt and tool schemas ~34k, `CLAUDE.md` ~26k, the skill listing ~4k,
  the deferred-tool list ~1.5k -- against a task prompt under 1k. Every turn
  re-reads that prefix, so it was **41-71% of each agent's cost**;
* two tracers given related questions each re-read the **same ~30k tokens** of
  report, parser and source before their first data command, and carried them
  on every later turn.

Both are invisible from inside the agent and from its result. They are only
visible in the transcripts Claude Code already writes, one per sub-agent, at
`~/.claude/projects/<escaped-cwd>/<session>/subagents/[workflows/<run>/]agent-*.jsonl`.
This reads those. **No agent spend**; it must run on the machine that ran the
agents, like `cacheprobe.py`.

What it reports
---------------
Per agent: API calls; the context it started with; the context it ended with;
cost in **input-token equivalents** (uncached input 1x, cache write 1.25x at the
5-minute TTL or 2x at 1-hour, cache read 0.1x, output 5x); the share of that
cost which is the fixed starting context re-read on every call; and its tool
calls split three ways:

    ORIENT  reading the repo's own code, docs and scripts to learn how things work
    DATA    running analyses, harnesses and parsers over data
    WRITE   writing or editing files

Per run (a workflow's directory, or the loose background agents): the **files
two or more agents read** -- the material a shared brief should have carried.

Caveats, each measured rather than assumed
------------------------------------------
* **Output tokens are estimated.** The transcript's `usage.output_tokens` is the
  value at the start of the stream (2-16 on every call checked), not the final
  count. Output is estimated as content characters / 4 (text, thinking and tool
  inputs). Cache reads dominate the cost, so the estimate moves little.
* **The classes are a heuristic on the command text.** `sed -n` on a `.rs` file
  is ORIENT; `python3` on anything is DATA. Read the per-agent ORIENT list
  (`--reads`) before arguing from the split.
* **Costs are list-price ratios**, not dollars and not a bill.

Usage
-----
    python3 scripts/agentmeter.py                  # this repo's newest session with sub-agents
    python3 scripts/agentmeter.py --all            # every session of this repo
    python3 scripts/agentmeter.py PATH ...         # transcripts or directories
    python3 scripts/agentmeter.py --reads          # ...and each agent's ORIENT calls
    python3 scripts/agentmeter.py --json           # one JSON object per agent
    python3 scripts/agentmeter.py --selftest       # controls; no transcript needed
"""
import collections
import datetime
import glob
import json
import os
import pathlib
import re
import sys
import tempfile

PROJECTS = pathlib.Path.home() / ".claude" / "projects"
REPO = str(pathlib.Path(__file__).resolve().parent.parent)
# A path inside the repo named in a command or a Read. Scratch, data and build
# output are not the repo's knowledge, so they never count as orientation.
REPO_FILE = re.compile(r"(?:^|[\s'\"=(])((?:src|scripts|examples|tests|Reports|wiki|assets|\.claude)/[\w./-]+\.\w+|(?:CLAUDE|README|PLAN)\.md)")
DATA_HINT = re.compile(r"zcat|\.csv\b|\.log\b|\.gz\b|/runs/|target/release|cargo (run|build|test)")
ORIENT_CMD = re.compile(r"\b(grep|rg|sed -n|cat|head|tail|awk|ls|find|wc|git (show|log|grep|diff))\b")


def escaped(cwd):
    return re.sub(r"[^A-Za-z0-9]", "-", cwd)


def classify(name, inp):
    """ORIENT / DATA / WRITE / OTHER for one tool call."""
    if name == "Read":
        p = inp.get("file_path", "")
        if p.endswith((".png", ".gif", ".jpg")) or "/scratchpad/" in p or "/tmp/" in p:
            return "DATA"
        return "ORIENT"
    if name in ("Grep", "Glob"):
        return "ORIENT"
    if name in ("Write", "Edit", "NotebookEdit"):
        return "WRITE"
    if name == "Bash":
        c = inp.get("command", "")
        if re.search(r"\bpython3?\b", c) or DATA_HINT.search(c):
            return "DATA"
        if ORIENT_CMD.search(c):
            return "ORIENT"
        return "OTHER"
    return "OTHER"


def repo_paths(name, inp):
    """Repo-relative files an ORIENT call read."""
    if name == "Read":
        p = inp.get("file_path", "")
        return [os.path.relpath(p, REPO)] if p.startswith(REPO + "/") else [p]
    text = inp.get("command", "") or inp.get("path", "") or inp.get("pattern", "")
    return sorted(set(m.group(1) for m in REPO_FILE.finditer(" " + text)))


def when(ts):
    return datetime.datetime.fromisoformat(ts.replace("Z", "+00:00"))


def meter(path):
    """One agent's numbers from its transcript."""
    rows = [json.loads(l) for l in open(path) if l.strip()]
    meta = {}
    mp = path[:-len(".jsonl")] + ".meta.json"
    if os.path.exists(mp):
        meta = json.load(open(mp))
    usage, out_chars, order, pending, calls = {}, collections.Counter(), [], {}, []
    for r in rows:
        if r.get("type") == "assistant":
            m = r["message"]
            mid = m.get("id") or r.get("uuid")
            if mid not in usage:
                order.append(mid)
            usage[mid] = m.get("usage", {})
            for b in m.get("content", []):
                t = b.get("type")
                if t == "text":
                    out_chars[mid] += len(b.get("text", ""))
                elif t == "thinking":
                    out_chars[mid] += len(b.get("thinking", ""))
                elif t == "tool_use":
                    out_chars[mid] += len(json.dumps(b.get("input", {})))
                    pending[b["id"]] = (b["name"], b.get("input", {}))
        elif r.get("type") == "user":
            c = r["message"].get("content")
            if isinstance(c, list):
                for b in c:
                    if b.get("type") == "tool_result" and b.get("tool_use_id") in pending:
                        name, inp = pending.pop(b["tool_use_id"])
                        s = b.get("content")
                        s = s if isinstance(s, str) else json.dumps(s)
                        calls.append((classify(name, inp), name, inp, len(s)))
    us = [usage[m] for m in order]
    ctx = lambda u: u.get("input_tokens", 0) + u.get("cache_creation_input_tokens", 0) + u.get("cache_read_input_tokens", 0)

    def write_cost(u):
        cc = u.get("cache_creation") or {}
        h1 = cc.get("ephemeral_1h_input_tokens", 0)
        return 1.25 * (u.get("cache_creation_input_tokens", 0) - h1) + 2.0 * h1

    out = sum(out_chars[m] for m in order) / 4.0
    cost = sum(u.get("input_tokens", 0) + write_cost(u) + 0.1 * u.get("cache_read_input_tokens", 0) for u in us) + 5 * out
    start = ctx(us[0]) if us else 0
    # The fixed prefix: paid in full on the first call, then re-read on every
    # later one. Bounded by each call's own context so it can never exceed it.
    fixed = (us[0].get("input_tokens", 0) + write_cost(us[0]) + 0.1 * us[0].get("cache_read_input_tokens", 0)) if us else 0
    fixed += sum(0.1 * min(start, ctx(u)) for u in us[1:])
    ts = [r["timestamp"] for r in rows if r.get("timestamp")]
    by = collections.Counter(c for c, _, _, _ in calls)
    chars = collections.Counter()
    for c, _, _, n in calls:
        chars[c] += n
    return {
        "agent": os.path.basename(path)[len("agent-"):-len(".jsonl")],
        "run": os.path.basename(os.path.dirname(path)),
        "task": meta.get("description", ""),
        "calls": len(us),
        "start": start,
        "end": ctx(us[-1]) if us else 0,
        "out_est": out,
        "cost": cost,
        "fixed_share": fixed / cost if cost else 0.0,
        "orient": by["ORIENT"], "data": by["DATA"], "write": by["WRITE"], "other": by["OTHER"],
        "orient_chars": chars["ORIENT"], "data_chars": chars["DATA"],
        "minutes": (when(ts[-1]) - when(ts[0])).total_seconds() / 60 if len(ts) > 1 else 0.0,
        "reads": [(name, inp, n, repo_paths(name, inp)) for c, name, inp, n in calls if c == "ORIENT"],
    }


def shared_reads(agents):
    """Repo files read by two or more agents of one run: path -> (agents, chars)."""
    who, size = collections.defaultdict(set), collections.Counter()
    for a in agents:
        for _, _, n, paths in a["reads"]:
            for p in paths:
                who[p].add(a["agent"])
                size[p] += n / max(1, len(paths))
    return {p: (sorted(w), size[p]) for p, w in who.items() if len(w) >= 2}


def run_dups(agents):
    """shared_reads within each run only: two agents of different runs never share a brief."""
    runs = collections.defaultdict(list)
    for a in agents:
        runs[a["run"]].append(a)
    return {run: shared_reads(group) for run, group in runs.items()}


def transcripts(args):
    if args:
        out = []
        for a in args:
            out += sorted(glob.glob(os.path.join(a, "**", "agent-*.jsonl"), recursive=True)) if os.path.isdir(a) else [a]
        return out
    root = PROJECTS / escaped(REPO)
    sessions = sorted((d for d in root.glob("*/subagents") if d.is_dir()), key=lambda d: d.stat().st_mtime)
    if "--all" not in sys.argv:
        sessions = sessions[-1:]
    return [p for d in sessions for p in sorted(glob.glob(str(d / "**" / "agent-*.jsonl"), recursive=True))]


def report(paths, show_reads=False, as_json=False):
    agents = [meter(p) for p in paths]
    if as_json:
        for a in agents:
            print(json.dumps({k: v for k, v in a.items() if k != "reads"}))
        return agents
    runs = collections.defaultdict(list)
    for a in agents:
        runs[a["run"]].append(a)
    k = lambda x: f"{x / 1000:.0f}k"
    for run, group in runs.items():
        print(f"== {run}: {len(group)} agent(s)")
        print(f"   {'agent':9} {'task':32} {'calls':>5} {'start':>6} {'end':>6} {'cost':>7} {'fixed':>6} {'orient':>6} {'data':>5} {'write':>5} {'orient chars':>12} {'data chars':>10} {'min':>5}")
        for a in group:
            print(f"   {a['agent'][:9]:9} {a['task'][:32]:32} {a['calls']:>5} {k(a['start']):>6} {k(a['end']):>6} {k(a['cost']):>7} "
                  f"{a['fixed_share']:>6.0%} {a['orient']:>6} {a['data']:>5} {a['write']:>5} {k(a['orient_chars']):>12} {k(a['data_chars']):>10} {a['minutes']:>5.1f}")
            if show_reads:
                for name, inp, n, ps in a["reads"]:
                    what = re.sub(r"\s+", " ", inp.get("file_path") or inp.get("command") or inp.get("pattern", ""))
                    print(f"        {name[:5]:5} {n:>7} ch  {what[:110]}")
        tot = sum(a["cost"] for a in group)
        print(f"   run total {k(tot)} input-token equivalents, {k(tot / len(group))} per agent; "
              f"tool calls orient {sum(a['orient'] for a in group)} / data {sum(a['data'] for a in group)} / write {sum(a['write'] for a in group)}")
        dup = run_dups(group)[run]
        if dup:
            print("   read by 2+ agents of this run (what a shared brief should carry):")
            for p, (w, n) in sorted(dup.items(), key=lambda kv: -kv[1][1])[:12]:
                print(f"      {len(w)} agents  {k(n):>5} chars  {p}")
    return agents


def selftest():
    """Controls for every number this prints, including a blind-classifier row
    that must turn the orientation check red: a check that cannot fail is not
    one (CLAUDE.md, put the fault back)."""
    tmp = tempfile.mkdtemp(prefix="agentmeter-")
    fails = []

    def agent(run, name, calls):
        d = os.path.join(tmp, run)
        os.makedirs(d, exist_ok=True)
        rows, t = [], 0
        for i, (usage, tool, inp, result) in enumerate(calls):
            t += 60
            ts = f"2026-09-27T12:{t // 60:02d}:00Z"
            rows.append({"type": "assistant", "timestamp": ts, "message": {"id": f"m{i}", "usage": usage, "content": [
                {"type": "text", "text": "x" * 400}, {"type": "tool_use", "id": f"t{i}", "name": tool, "input": inp}]}})
            rows.append({"type": "user", "timestamp": ts, "message": {"content": [
                {"type": "tool_result", "tool_use_id": f"t{i}", "content": "r" * result}]}})
        with open(os.path.join(d, f"agent-{name}.jsonl"), "w") as fh:
            fh.write("\n".join(json.dumps(r) for r in rows))
        json.dump({"description": name}, open(os.path.join(d, f"agent-{name}.meta.json"), "w"))
        return os.path.join(d, f"agent-{name}.jsonl")

    first = {"input_tokens": 2, "cache_creation_input_tokens": 40000, "cache_read_input_tokens": 30000,
             "cache_creation": {"ephemeral_5m_input_tokens": 40000, "ephemeral_1h_input_tokens": 0}}
    later = {"input_tokens": 1, "cache_creation_input_tokens": 1000, "cache_read_input_tokens": 74000,
             "cache_creation": {"ephemeral_5m_input_tokens": 1000, "ephemeral_1h_input_tokens": 0}}
    read = ("Read", {"file_path": f"{REPO}/scripts/antloop.py"}, 4000)
    sed = ("Bash", {"command": f"cd {REPO}; sed -n 1,80p Reports/how-the-ant-works.md"}, 2000)
    py = ("Bash", {"command": "python3 /tmp/x/analyse.py /home/user/runs/csv/a.csv"}, 1000)
    a = agent("wfA", "alpha", [(first,) + read, (later,) + py])
    b = agent("wfA", "beta", [(first,) + read, (later,) + sed])
    c = agent("wfB", "gamma", [(first,) + sed])
    ma, mb = meter(a), meter(b)

    def check(name, ok, detail=""):
        print(f"  {'ok  ' if ok else 'FAIL'} {name}{'  ' + detail if detail else ''}")
        if not ok:
            fails.append(name)

    # Cost: 2 + 1.25*40000 + 0.1*30000 = 53002 for call 1; 1 + 1.25*1000 + 0.1*74000 = 8651 for call 2;
    # output (400 chars of text + the tool input's JSON per call) / 4 * 5.
    out = (400 + len(json.dumps(read[1])) + 400 + len(json.dumps(py[1]))) / 4.0
    want = 53002 + 8651 + 5 * out
    check("cost arithmetic", abs(ma["cost"] - want) < 1e-6, f"{ma['cost']:.1f} vs {want:.1f}")
    check("starting context", ma["start"] == 70002, str(ma["start"]))
    fixed = 53002 + 0.1 * 70002
    check("fixed share", abs(ma["fixed_share"] - fixed / want) < 1e-9, f"{ma['fixed_share']:.4f}")
    check("classes: Read of repo source is ORIENT, python on data is DATA", (ma["orient"], ma["data"]) == (1, 1))
    check("classes: sed -n on a report is ORIENT", (mb["orient"], mb["data"]) == (2, 0))
    dup = shared_reads([ma, mb])
    check("positive control: the file both agents read is found", list(dup) == ["scripts/antloop.py"], str(dup))
    # beta (run wfA) and gamma (run wfB) both read how-the-ant-works.md: across runs that is not a
    # shared read, and the same pair put in one run must be one (so the negative is not vacuous)
    mc = meter(c)
    check("negative control: agents of different runs are never paired", all(not d for d in run_dups([mb, mc]).values()),
          str(run_dups([mb, mc])))
    check("...and the same two in one run are", "Reports/how-the-ant-works.md" in shared_reads([mb, mc]))
    # Blind row: a classifier that calls everything DATA must turn the class check red.
    global classify
    real = classify
    classify = lambda name, inp: "DATA"
    try:
        blind = meter(a)
        check("blind classifier is caught (sensitivity)", (blind["orient"], blind["data"]) != (1, 1))
    finally:
        classify = real
    print(f"agentmeter selftest: {'PASS' if not fails else 'FAIL: ' + ', '.join(fails)}")
    return 1 if fails else 0


def main():
    argv = [a for a in sys.argv[1:] if not a.startswith("--")]
    if "--selftest" in sys.argv:
        return selftest()
    paths = transcripts(argv)
    if not paths:
        print(f"agentmeter: no sub-agent transcripts under {PROJECTS / escaped(REPO)}/*/subagents/")
        print("  It must run on the machine that ran the agents. --selftest works anywhere.")
        return 1
    report(paths, show_reads="--reads" in sys.argv, as_json="--json" in sys.argv)
    return 0


if __name__ == "__main__":
    sys.exit(main())
