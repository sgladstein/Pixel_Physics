#!/usr/bin/env python3
"""Colony-selection, in-run and emergence batches on GitHub's hosted runners (`.github/workflows/batch.yml`).

A batch file `.github/train-jobs/<batch>.batch` holds the same job lines the Google VM runs
(`scratch/cloud/inrun_cloud.py`), so one jobs file runs in either place:

    # comments and blank lines are ignored
    ref=<branch or commit>            optional: build the code from this ref (resolved to a sha once, at plan time)
    resume=1                          optional: carry on batch-results/<batch> from an earlier run (failed lines retry)
    leg_minutes=N                     optional: a shorter leg than the 320-minute default (a quick look, or a test)
    use=scratch/inrun/jobs/elite2.txt [only=e3,e13]   every line of a VM jobs file in the repo (or the named OUTs)
    OUT colsel P MODE NCOL ROUNDS FRAMES GENOME [inrun args...]     one colsel.py driver (P becomes the runner's cores)
    OUT SEED ROUNDS FRAMES SEL GENOME [inrun args...]               one rounds.sh loop
    OUT emerge ARM N GENS FRAMES FOUNDER [opts and inrun args...]   one scratch/pheromones/emerge.py arm
    OUT KEY=VALUE ... <one of the above>                            environment for that line (e.g. PP_GATE=off)

Each colsel or emerge line gets a runner of its own; rounds.sh lines are packed PACK to a runner (each is one core).
A runner stops at 6 h, so a batch runs in legs: each leg carries on what the last one published.

    python3 scripts/batch.py plan                 # in the workflow: the pushed head commit's .batch files -> matrix
    python3 scripts/batch.py todo ENTRY_JSON DIR  # in a leg: which lines of this entry still need running
    python3 scripts/batch.py collect DIR BATCH LEG  # after a leg: write STATUS.md, print how many entries remain
    python3 scripts/batch.py --selftest           # the refusals and the parse, positive control
"""
import json, os, re, subprocess, sys

NAME = re.compile(r"^[A-Za-z0-9._-]{1,60}$")
REF = re.compile(r"^[A-Za-z0-9._/-]{1,100}$")
ENVKEY = re.compile(r"^[A-Z][A-Z0-9_]*$")
# a line may not steer the runner itself: these are set by the leg (BIN, P, DEADLINE) or would break it
ENV_REFUSED = {"BIN", "P", "DEADLINE", "PATH", "HOME", "LD_PRELOAD", "LD_LIBRARY_PATH", "GH_TOKEN", "GITHUB_TOKEN"}
PACK = 4  # rounds.sh lines per runner: each uses one core, and a hosted runner has 4 vCPUs
MAX_ENTRIES = 256  # GitHub's matrix limit
STATES = ("done", "paused", "failed", "new")


def split_line(line):
    """(out, env, kind, words) for one job line, or raise ValueError. `words` is what follows the env tokens."""
    toks = line.split()
    if len(toks) < 2:
        raise ValueError(f"{line!r}: a job line is OUT followed by its arguments")
    out, rest = toks[0], toks[1:]
    if not NAME.match(out):
        raise ValueError(f"{line!r}: OUT {out!r} is not a bare name (letters, digits, . _ -)")
    env = {}
    while rest and "=" in rest[0] and ENVKEY.match(rest[0].split("=", 1)[0]):
        k, v = rest.pop(0).split("=", 1)
        if k in ENV_REFUSED or k.startswith(("GITHUB_", "RUNNER_", "ACTIONS_")):
            raise ValueError(f"{line!r}: {k}= is set by the runner, remove it")
        env[k] = v
    if not rest:
        raise ValueError(f"{line!r}: nothing to run after the environment")
    for t in rest + [f"{k}={v}" for k, v in env.items()]:
        for v in re.split(r"[=,:@]", t):
            if v.startswith("/") or ".." in v.split("/"):
                raise ValueError(f"{line!r}: {t!r} is an absolute or parent path; use a path inside the repo "
                                 "(the /mnt project files are not on the runner)")
        if t.startswith("out="):
            raise ValueError(f"{line!r}: out= is set by the driver, remove it")
    kind = rest[0] if rest[0] in ("colsel", "emerge") else "rounds"
    need = {"colsel": 8, "emerge": 6, "rounds": 5}[kind]
    if len(rest) < need:
        raise ValueError(f"{line!r}: a {kind} line needs at least {need} words after OUT, found {len(rest)}")
    if kind == "rounds" and not all(x.isdigit() for x in rest[:3]):
        raise ValueError(f"{line!r}: not colsel, emerge, or a rounds.sh line (SEED ROUNDS FRAMES must be numbers)")
    if kind == "colsel" and not all(x.isdigit() for x in (rest[1], rest[3], rest[4], rest[5])):
        raise ValueError(f"{line!r}: colsel's P NCOL ROUNDS FRAMES must be numbers")
    return out, env, kind, rest


def parse_batch(name, text, read=lambda p: open(p).read()):
    """{batch, ref, resume, lines: [(out, kind, line)]} for one batch file, or raise ValueError."""
    if not NAME.match(name):
        raise ValueError(f"batch name {name!r} is not letters, digits, . _ -")
    ref, resume, leg, lines = "", False, 320, []
    for raw in text.splitlines():
        l = raw.strip()
        if not l or l.startswith("#"):
            continue
        if l.startswith("ref="):
            ref = l[4:]
            if not REF.match(ref) or ".." in ref:
                raise ValueError(f"{name}: ref={ref!r} is not a branch name or commit")
        elif l.startswith("resume="):
            v = l[7:]
            if v not in ("0", "1", "true", "false", "on", "off"):
                raise ValueError(f"{name}: resume={v!r} is not 1 or 0")
            resume = v in ("1", "true", "on")
        elif l.startswith("leg_minutes="):
            v = l[12:]
            if not v.isdigit() or not 5 <= int(v) <= 320:
                raise ValueError(f"{name}: leg_minutes={v!r} is not a number of minutes from 5 to 320 (a runner stops at 360)")
            leg = int(v)
        elif l.startswith("use="):
            toks = l.split()
            path = toks[0][4:]
            only = None
            for t in toks[1:]:
                if t.startswith("only="):
                    only = set(t[5:].split(","))
                else:
                    raise ValueError(f"{name}: {t!r} after use= (only only=a,b may follow)")
            if path.startswith("/") or ".." in path.split("/"):
                raise ValueError(f"{name}: use={path!r} must be a path inside the repo")
            got = [x.strip() for x in read(path).splitlines() if x.strip() and not x.strip().startswith("#")]
            if only is not None:
                missing = only - {x.split()[0] for x in got}
                if missing:
                    raise ValueError(f"{name}: only= names {sorted(missing)}, not in {path}")
                got = [x for x in got if x.split()[0] in only]
            for x in got:
                lines.append(x)
        else:
            lines.append(l)
    if not lines:
        raise ValueError(f"{name}: no job lines")
    parsed = [split_line(l) for l in lines]
    outs = [p[0] for p in parsed]
    dup = sorted({o for o in outs if outs.count(o) > 1})
    if dup:
        raise ValueError(f"{name}: OUT names used twice: {dup}")
    return {"batch": name, "ref": ref, "resume": resume, "leg_minutes": leg,
            "lines": [(p[0], p[2], " ".join(l.split())) for p, l in zip(parsed, lines)]}


def entries(b):
    """The matrix entries of one parsed batch: a colsel or emerge line alone, rounds.sh lines PACK at a time."""
    one = [(o, [l]) for o, k, l in b["lines"] if k != "rounds"]
    rounds = [l for o, k, l in b["lines"] if k == "rounds"]
    packs = [rounds[i:i + PACK] for i in range(0, len(rounds), PACK)]
    out = one + [(f"pack{i + 1:03}", p) for i, p in enumerate(packs)]
    if len(out) > MAX_ENTRIES:
        raise ValueError(f"{b['batch']}: {len(out)} runners wanted, GitHub's matrix holds {MAX_ENTRIES}")
    return [{"entry": e, "lines": "\n".join(ls)} for e, ls in out]


def resolve(ref):
    """A full sha for ref (a sha already, or a branch on origin), so every leg builds the same code."""
    if re.fullmatch(r"[0-9a-f]{40}", ref):
        return ref
    r = subprocess.run(["git", "ls-remote", "origin", ref], capture_output=True, text=True, check=True).stdout.split()
    if not r:
        raise ValueError(f"ref={ref!r}: no such branch on origin (a short sha cannot be resolved; give all 40 digits)")
    return r[0]


def plan():
    if os.environ.get("EVENT") == "workflow_dispatch":
        name = os.environ["IN_NAME"]
        if not NAME.match(name):
            raise ValueError(f"batch name {name!r} is not letters, digits, . _ -")
        files = [f".github/train-jobs/{name}.batch"]
    else:
        files = subprocess.run(
            ["git", "diff-tree", "--no-commit-id", "--name-only", "-r", "--diff-filter=AM", "HEAD"],
            check=True, capture_output=True, text=True).stdout.split()
        files = [f for f in files if f.startswith(".github/train-jobs/") and f.endswith(".batch")]
    if len(files) > 1:
        # legs of different batches would wait on each other's slowest entry, and the results branch is per batch
        sys.exit(f"one .batch file per push, found {len(files)}: {files}")
    out = {"batch": "", "ref": "", "resume": "false", "leg_minutes": "320", "matrix": "[]", "n": "0"}
    if files:
        b = parse_batch(os.path.basename(files[0])[:-6], open(files[0]).read())
        es = entries(b)
        sha = resolve(b["ref"]) if b["ref"] else subprocess.run(["git", "rev-parse", "HEAD"], capture_output=True, text=True, check=True).stdout.strip()
        kinds = {}
        for _, k, _ in b["lines"]:
            kinds[k] = kinds.get(k, 0) + 1
        print(f"batch {b['batch']}: {len(b['lines'])} lines {kinds} on {len(es)} runners, code at {sha}, resume {b['resume']}, legs of {b['leg_minutes']} min")
        out = {"batch": b["batch"], "ref": sha, "resume": str(b["resume"]).lower(), "leg_minutes": str(b["leg_minutes"]),
               "matrix": json.dumps(es), "n": str(len(es))}
    else:
        print("no .batch file in the pushed commit")
    with open(os.environ["GITHUB_OUTPUT"], "a") as f:
        for k, v in out.items():
            f.write(f"{k}={v}\n")


def read_status(path):
    """{out: (state, note)} from an entry's status file (`OUT STATE note...` lines); {} if there is none."""
    st = {}
    if os.path.exists(path):
        for l in open(path):
            p = l.rstrip("\n").split(" ", 2)
            if len(p) >= 2 and p[1] in STATES:
                st[p[0]] = (p[1], p[2] if len(p) > 2 else "")
    return st


def todo(entry_json, d, retry=False):
    """Print the lines of this entry that are not done or failed in d/<entry>.status (d: the results branch). With
    retry (the first leg of a resumed batch), failed lines run again: a failure inside one push's chain of legs is
    not retried, since it would most likely fail the same way on every leg."""
    e = json.loads(entry_json)
    st = read_status(os.path.join(d, e["entry"] + ".status")) if d else {}
    skip = ("done",) if retry else ("done", "failed")
    for l in e["lines"].split("\n"):
        if st.get(l.split()[0], ("new",))[0] not in skip:
            print(l)


def collect(d, batch, leg, matrix):
    """After a leg: STATUS.md over every entry's status file in d, and the number of entries with lines left."""
    rows, left = [], 0
    for e in json.loads(matrix):
        st = read_status(os.path.join(d, e["entry"] + ".status"))
        outs = [l.split()[0] for l in e["lines"].split("\n")]
        states = [st.get(o, ("new", "no status published"))for o in outs]
        if any(s[0] in ("paused", "new") for s in states):
            left += 1
        for o, s in zip(outs, states):
            rows.append(f"| {e['entry']} | {o} | {s[0]} | {s[1]} |")
    with open(os.path.join(d, "STATUS.md"), "w") as f:
        f.write(f"# batch {batch}, after leg {leg}\n\n{left} runner(s) still have lines to run.\n\n"
                "| runner | job | state | note |\n|---|---|---|---|\n" + "\n".join(rows) + "\n")
    print(left)


def selftest():
    jobs = {"j.txt": "# vm jobs\na colsel 31 lex 32 16 200000 assets/minds/std_pool.txt cg_every=8\n"
                     "b 1 3 100000 score assets/minds/h1.raw.txt\n"}
    b = parse_batch("t1", "ref=main\nuse=j.txt only=a\nc PP_GATE=off emerge free 16 40 100000 assets/minds/std_pool.txt P=4 SEED=41\n"
                    + "".join(f"r{i} {i} 3 100000 score assets/minds/h1.raw.txt lesson=scratch/x.txt:0.1\n" for i in range(9)),
                    read=lambda p: jobs[p])
    assert b["ref"] == "main" and not b["resume"] and b["leg_minutes"] == 320, b
    assert parse_batch("t", "leg_minutes=20\nresume=1\nx 1 2 3 score g")["leg_minutes"] == 20
    assert [(o, k) for o, k, _ in b["lines"]][:2] == [("a", "colsel"), ("c", "emerge")], b["lines"][:2]
    es = entries(b)
    assert [e["entry"] for e in es] == ["a", "c", "pack001", "pack002", "pack003"], es
    assert es[2]["lines"].count("\n") == PACK - 1 and es[4]["lines"].count("\n") == 0, es
    assert split_line("c PP_GATE=off X_Y=1 emerge free 16 40 100000 f.txt")[1] == {"PP_GATE": "off", "X_Y": "1"}
    bad = [("a b", "x 1 2 3 score g"), ("a", "# only comment"), ("a", "x 1 2 3 score g\nx 2 2 3 score g"),
           ("a", "x 1 2 3 score /mnt/project-files/g.txt"), ("a", "x 1 2 3 score g lesson=../g:0.1"),
           ("a", "x 1 2 3 score g founders=a@b:1,/etc/x@c:1"), ("a", "x 1 2 3 score g out=y"),
           ("a", "x BIN=/bin/sh 1 2 3 score g"), ("a", "x GITHUB_TOKEN=1 1 2 3 score g"), ("a", "x colsel 31 lex 32 16 g"),
           ("a", "x seed 2 3 score g"), ("a", "x/y 1 2 3 score g"), ("a", "ref=../etc\nx 1 2 3 score g"),
           ("a", "use=j.txt only=zz"), ("a", "use=/etc/passwd"), ("a", "resume=yes\nx 1 2 3 score g"), ("a", "leg_minutes=400\nx 1 2 3 score g"), ("a", "leg_minutes=2\nx 1 2 3 score g"), ("a", "x PP_A=1")]
    for n, t in bad:
        try:
            parse_batch(n, t, read=lambda p: jobs[p])
        except (ValueError, KeyError):
            continue
        sys.exit(f"selftest: {n!r} {t!r} was accepted")
    try:
        entries(parse_batch("big", "".join(f"r{i} {i} 3 1 score g\n" for i in range(PACK * MAX_ENTRIES + 1))))
        sys.exit("selftest: a batch over the matrix limit was accepted")
    except ValueError:
        pass
    # todo/collect over a status file: done and failed lines are not rerun, paused and new ones are
    import tempfile, io, contextlib
    d = tempfile.mkdtemp()
    open(os.path.join(d, "pack001.status"), "w").write("r0 done exit 0\nr1 failed exit 1\nr2 paused round 4\n")
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        todo(json.dumps(es[2]), d)
    assert [l.split()[0] for l in buf.getvalue().splitlines()] == ["r2", "r3"], buf.getvalue()
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        todo(json.dumps(es[2]), d, retry=True)
    assert [l.split()[0] for l in buf.getvalue().splitlines()] == ["r1", "r2", "r3"], buf.getvalue()
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        collect(d, "t1", 1, json.dumps(es))
    assert buf.getvalue().strip() == str(len(es)), buf.getvalue()  # every runner has a line not done
    print(f"selftest ok: {len(bad) + 1} refused, packing, todo and collect")


if __name__ == "__main__":
    a = sys.argv[1:]
    if a == ["--selftest"]:
        selftest()
    elif a[:1] == ["plan"]:
        try:
            plan()
        except ValueError as e:
            sys.exit(f"refused: {e}")
    elif a[:1] == ["todo"]:
        todo(a[1], a[2] if len(a) > 2 else "", retry=a[3:4] == ["retry"])
    elif a[:1] == ["collect"]:
        collect(a[1], a[2], a[3], os.environ["MATRIX"])
    else:
        sys.exit(__doc__)
