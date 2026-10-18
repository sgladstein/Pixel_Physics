#!/usr/bin/env python3
"""Turn pushed `.github/train-jobs/*.job` files (or workflow_dispatch inputs)
into the matrix `.github/workflows/train.yml` runs. Writes `matrix=<json>` to
$GITHUB_OUTPUT. Refuses, loudly, anything that could not run on a hosted
runner: a path under /mnt (the project-files mount is not there), an `out=`
(the workflow owns it), or a name that is not a plain token.

    EVENT=push python3 scripts/train_plan.py        # head commit's .job files
    python3 scripts/train_plan.py --selftest        # the refusals, positive control
"""
import json, os, re, subprocess, sys

NAME = re.compile(r"^[A-Za-z0-9._-]{1,60}$")
REF = re.compile(r"^[A-Za-z0-9._/-]{1,100}$")


def parse_job(name, text):
    """The single arguments line of a job file, or raise ValueError."""
    if not NAME.match(name):
        raise ValueError(f"job name {name!r} is not letters, digits, . _ -")
    lines = [l.strip() for l in text.splitlines() if l.strip() and not l.strip().startswith("#")]
    if len(lines) != 1:
        raise ValueError(f"{name}: need exactly one arguments line, found {len(lines)}")
    ref = ""
    keep = []
    for tok in lines[0].split():
        if tok.startswith("ref="):
            ref = tok[4:]
            if not REF.match(ref) or ".." in ref:
                raise ValueError(f"{name}: ref={ref!r} is not a branch name or commit")
        else:
            keep.append(tok)
    args = " ".join(keep)
    for tok in args.split():
        if "=" not in tok:
            raise ValueError(f"{name}: argument {tok!r} is not key=value")
        k, v = tok.split("=", 1)
        if k == "every" and v != "1":
            raise ValueError(f"{name}: every={v} -- curves and hold-out are wanted every generation (every=1, the default); remove it")
        if k == "out":
            raise ValueError(f"{name}: out= is set by the workflow, remove it")
        if v.startswith("/") or ".." in v:
            raise ValueError(f"{name}: {tok!r} is an absolute or parent path; use a path inside the repo (the /mnt project files are not on the runner)")
    return {"name": name, "args": args, "ref": ref}


def from_push():
    files = subprocess.run(
        ["git", "diff-tree", "--no-commit-id", "--name-only", "-r", "--diff-filter=AM", "HEAD"],
        check=True, capture_output=True, text=True).stdout.split()
    jobs = []
    for f in files:
        if f.startswith(".github/train-jobs/") and f.endswith(".job"):
            jobs.append(parse_job(os.path.basename(f)[:-4], open(f).read()))
    return jobs


def main():
    if os.environ.get("EVENT") == "workflow_dispatch":
        jobs = [parse_job(os.environ["IN_NAME"], os.environ["IN_ARGS"] + (" ref=" + os.environ["IN_REF"] if os.environ.get("IN_REF") else ""))]
    else:
        jobs = from_push()
    names = [j["name"] for j in jobs]
    if len(set(names)) != len(names):
        sys.exit("duplicate job names")
    print(f"{len(jobs)} job(s): {names}")
    with open(os.environ["GITHUB_OUTPUT"], "a") as out:
        out.write("matrix=" + json.dumps(jobs) + "\n")


def selftest():
    ok = parse_job("a-1", "# c\nbrain=mind gens=2 start=training/x.txt\n")
    assert ok == {"name": "a-1", "args": "brain=mind gens=2 start=training/x.txt", "ref": ""}, ok
    withref = parse_job("a", "ref=claude/foo-1 gens=2")
    assert withref == {"name": "a", "args": "gens=2", "ref": "claude/foo-1"}, withref
    for bad_name, bad in [("a b", "x=1"), ("a", "x=1\ny=2"), ("a", "out=foo"),
                          ("a", "start=/mnt/project-files/x"), ("a", "start=../x"), ("a", "gens"), ("a", "# only comment"), ("a", "every=5"), ("a", "ref=../etc gens=1")]:
        try:
            parse_job(bad_name, bad)
        except ValueError:
            continue
        sys.exit(f"selftest: {bad_name!r} {bad!r} was accepted")
    print("selftest ok: 2 accepted, 9 refused")


if __name__ == "__main__":
    selftest() if "--selftest" in sys.argv else main()
