#!/usr/bin/env python3
"""Watch the Google training VMs and the repo; exit when something changes, or when everything has been quiet long enough.

usage: watch.py LAUNCHER_DIR STATE_JSON [--poll 600] [--max 6600] [--quiet-min 60]
Prints one line per poll. Exit reasons (last line): CHANGE <what> | QUIET | TIMEOUT.
"""
import json, os, subprocess, sys, time, datetime
sys.path.insert(0, sys.argv[1])
import gcp_train_job as g  # noqa: E402
from google.cloud import compute_v1  # noqa: E402

STATE = sys.argv[2]
args = sys.argv[3:]
def opt(k, d):
    return int(args[args.index(k) + 1]) if k in args else d
POLL, MAX, QUIET = opt("--poll", 600), opt("--max", 6600), opt("--quiet-min", 60)
REPO = "repos/sgladstein/Pixel_Physics"

def now():
    return datetime.datetime.now(datetime.timezone.utc)

def vms():
    info, creds = g._creds()
    c = compute_v1.InstancesClient(credentials=creds)
    out = {}
    for zone, scoped in c.aggregated_list(project=info["project_id"]):
        for i in scoped.instances or []:
            out[i.name] = i.status
    return out

def runs():
    """newest object per run in the bucket: {run: (updated, has_DONE)}"""
    r = g.gcs().get(f"{g.GCS}/{g.BUCKET}/o", params={"prefix": "runs/", "maxResults": 1000}, timeout=60).json()
    out = {}
    for o in r.get("items", []):
        parts = o["name"].split("/")
        if len(parts) < 3:
            continue
        run, obj = parts[1], parts[2]
        u, d = out.get(run, ("", False))
        out[run] = (max(u, o["updated"]), d or obj == "DONE")
    return out

def gh(path):
    p = subprocess.run(["gh", "api", path], capture_output=True, text=True, timeout=60)
    return json.loads(p.stdout) if p.returncode == 0 else None

def repo():
    """the brain lane's refs only: its handoff branch, train/* and train-results/* branches, and the open PR heads"""
    prs = gh(f"{REPO}/pulls?state=open&per_page=100") or []
    p = subprocess.run(["git", "-C", "/home/user/pixel_physics", "ls-remote", "origin", "refs/heads/claude/project-thread-eewfjj",
                        "refs/heads/train/*", "refs/heads/train-results/*"], capture_output=True, text=True, timeout=120)
    refs = dict(reversed(l.split("\t")) for l in p.stdout.splitlines() if "\t" in l)
    # newest brain-lane commit: the handoff branch if it exists, else the handoff/brain-training files' last change on main
    dates = []
    c = gh(f"{REPO}/commits/claude/project-thread-eewfjj") or {}
    if c.get("commit"):
        dates.append(c["commit"]["committer"]["date"])
    for path in ("Reports/handoff/brain-lane-handoff.md", "Reports/brain-training", "training/starts"):
        m = gh(f"{REPO}/commits?sha=main&per_page=1&path={path}") or []
        if m:
            dates.append(m[0]["commit"]["committer"]["date"])
    c = {"commit": {"committer": {"date": max(dates) if dates else None}}}
    return {
        "refs": {k.replace("refs/heads/", ""): v[:9] for k, v in sorted(refs.items())},
        "lane_commit_at": c.get("commit", {}).get("committer", {}).get("date"),
        "prs": sorted(f"{p['number']}@{p['head']['sha'][:7]}" for p in prs),
    }

prev = json.load(open(STATE)) if os.path.exists(STATE) else None
t0 = time.time()
while True:
    try:
        v, r, rp = vms(), runs(), repo()
    except Exception as e:
        print(now().strftime("%H:%M"), "poll error", type(e).__name__, str(e)[:150], flush=True)
        time.sleep(POLL); continue
    # a run is "live" if its bucket copy was written in the last 15 minutes
    live = {k: u for k, (u, d) in r.items() if (now() - datetime.datetime.fromisoformat(u.replace("Z", "+00:00"))).total_seconds() < 900}
    done = sorted(k for k, (u, d) in r.items() if d)
    cur = {"vms": sorted(v), "done": done, "repo": rp}  # VM names only: a RUNNING->STOPPING step is not news
    print(now().strftime("%H:%M"), "vms", v, "| live runs", sorted(live), "| prs", rp["prs"], "| lane commit", rp["lane_commit_at"], "| refs", len(rp["refs"]), flush=True)
    if prev is not None and cur != prev:
        changed = [k for k in cur if cur[k] != prev.get(k)]
        json.dump(cur, open(STATE, "w"))
        print("CHANGE", changed, flush=True); sys.exit(0)
    json.dump(cur, open(STATE, "w")); prev = cur
    pushed = datetime.datetime.fromisoformat(rp["lane_commit_at"].replace("Z", "+00:00")) if rp["lane_commit_at"] else now()
    if not v and not live and (now() - pushed).total_seconds() > QUIET * 60:
        print("QUIET: no VMs, no live runs, last handoff commit", rp["lane_commit_at"], flush=True); sys.exit(0)
    if time.time() - t0 > MAX:
        print("TIMEOUT", flush=True); sys.exit(0)
    time.sleep(POLL)
