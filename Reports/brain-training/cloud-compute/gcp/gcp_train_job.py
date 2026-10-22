#!/usr/bin/env python3
"""Run one brain-training job on a Google Cloud SPOT VM and bring the results back over the VM's serial console.

Commands (all print only non-secret info; credentials come from env GCP_SA_KEY_B64, never printed):
  start  NAME "TRAIN ARGS" [--ref REF] [--type c4d-highcpu-32] [--hours 6] [--seed TAR]
                              create the VM; it builds and trains by itself
  fetch  NAME [DEST_DIR]      read the serial console, save the newest snapshot as DEST_DIR/NAME-snap.tar.gz + summary files
  status NAME                 VM state, last log lines (@@LOG) and the newest snapshot's age
  delete NAME                 delete the VM (always do this when a job is finished or abandoned)
  watch  NAME "TRAIN ARGS" [start options] [--dest DIR] [--poll 240] [--max-restarts 3]
                              start the job, fetch every --poll seconds, and if the spot VM is preempted (VM gone before @@DONE)
                              restart it seeded from the last fetched snapshot (resume=1 added); at @@DONE do a final fetch,
                              delete the VM and exit.  Run it in the background (nohup / run_in_background).

TRAIN ARGS are the same words as a .github/train-jobs/*.job file (no out=, no every= other than 1); a `start=` genome path
is resolved inside the repo checkout on the VM (so it must exist at REF). REF is a commit sha or branch of
github.com/sgladstein/Pixel_Physics (public).  threads= defaults to the VM's core count.

Why the serial console: the VM has no service account and no secret on it, so the only way out is what it prints.
It prints one base64 snapshot (log, progress/holdout csv, curves.svg, hold-best, best) every 5 minutes and at the end,
between @@SNAP-BEGIN / @@SNAP-END markers.  Console buffer is 1 MiB, enough for the newest snapshots; pictures are
included in the snapshot (curves.svg) as a file.
Durable copy (10-19, after two timing runs lost their console output): when the bucket GCP_BUCKET (default
pixel-physics-train-512114) is reachable, the VM runs as train-writer@<project> (access to that one bucket only) and puts
each snapshot to runs/NAME/latest.tgz and a runs/NAME/DONE marker at the end. fetch/status/watch read the bucket first, so a
preempted VM loses at most 5 minutes and `watch` restarts from the bucket copy; a finished job can be fetched with no VM.
Without the bucket it falls back to the serial console and says so at start. A name that already has results in the bucket
is refused unless it is a continuation.
Safety: the VM deletes itself after --hours + 1 h (max_run_duration), is labelled purpose=pixel-train, and `delete`
removes it.  Spot VMs can be preempted: `status` says 'gone'.
The post-DONE `shutdown` is not reliable (2026-10-20: a VM was still RUNNING ~20 min after @@DONE), so after a final
fetch, check `status` and `delete`. Delete only once the bucket's DONE marker is confirmed, never in the same command as a
fetch: a run deleted at generation 337 of 340 lost its last 3 generations and its hold-best.txt.

Carrying a run on (the trainer's exact resume, `resume=1`): every snapshot holds `state.txt`, so `start NAME "... resume=1"
--seed <a NAME-snap.tar.gz from fetch, or a tar of a train-results/<name> branch>` puts that run's state.txt, progress.csv
and holdout.csv in the new VM's out folder before training, and the run goes on from its last saved generation as if it
had never stopped. Only those files are taken from the tar (rebuilt here, so no path in it reaches the VM); its
STATUS.txt is kept as STATUS-earlier.txt. They travel in the VM's metadata (no bucket, no secret); the trainer refuses a
state from another start, arena, hold-out or parts. After a preemption, start again with --seed of the last fetch."""
import argparse, base64, io, json, os, re, sys, tarfile, time, urllib.parse
from google.oauth2 import service_account
from google.cloud import compute_v1

# Durable results: a bucket the VM writes every snapshot to (runs/NAME/latest.tgz, and runs/NAME/DONE at the end), so a
# preempted spot VM loses at most 5 minutes and `watch` resumes from the bucket copy. The VM runs as WRITER, which is granted
# access to this one bucket only (set up by hand in the console: see the doc/steps in the cloud-compute notes).
# No bucket reachable -> serial-console fallback, as before.
BUCKET = os.environ.get("GCP_BUCKET", "pixel-physics-train-512114")
WRITER = "train-writer@{project}.iam.gserviceaccount.com"
GCS = "https://storage.googleapis.com/storage/v1/b"

ZONES = ("us-central1-a", "us-central1-b", "us-central1-c", "us-central1-f")
REPO = "https://github.com/sgladstein/Pixel_Physics.git"
NAME_RE = re.compile(r"^[a-z0-9][a-z0-9-]{0,40}$")

STARTUP = r"""#!/bin/bash
set -u
W=/root/w; mkdir -p $W/out $W/snap; cd $W
NAME='@NAME@'; REF='@REF@'; ARGS='@ARGS@'
MD=http://metadata.google.internal/computeMetadata/v1/instance
BUCKET=$(curl -sf -H "Metadata-Flavor: Google" $MD/attributes/bucket || true)
tty() { cat > /dev/ttyS0; }
# the durable copy: PUT one object to the run's bucket with the VM service account's token (that account can touch only this bucket)
put() {  # put <local file> <object name under runs/$NAME/>
  [ -n "$BUCKET" ] || return 0
  TOK=$(curl -sf -H "Metadata-Flavor: Google" $MD/service-accounts/default/token | python3 -c 'import sys,json;print(json.load(sys.stdin)["access_token"])') || { log "bucket token failed"; return 1; }
  curl -sf -X POST -H "Authorization: Bearer $TOK" -H "Content-Type: application/octet-stream" --data-binary "@$1" \
    "https://storage.googleapis.com/upload/storage/v1/b/$BUCKET/o?uploadType=media&name=runs%2F$NAME%2F$2" > /dev/null || { log "bucket upload of $2 failed"; return 1; }
}
log() { echo "@@LOG $(date -u +%T) $*" | tty; }
snap() {
  cd $W/out
  rm -rf $W/snap/*; for f in STATUS.txt STATUS-earlier.txt progress.csv holdout.csv hold-best.txt best.txt curves.svg state.txt cpu.csv; do [ -f $f ] && cp $f $W/snap/; done
  [ -f train.log ] && tail -n 800 train.log > $W/snap/train.log
  [ -f $W/build.log ] && tail -n 60 $W/build.log > $W/snap/build-tail.log
  tar -C $W/snap -czf $W/snap.tgz . 2>/dev/null
  put $W/snap.tgz latest.tgz
  { echo "@@SNAP-BEGIN $NAME $(date -u +%FT%TZ) $(stat -c%s $W/snap.tgz)"; base64 -w 100 $W/snap.tgz | sed 's/^/@@B /'; echo "@@SNAP-END"; } | tty
  cd $W
}
log "boot $(nproc) cpus"
# CPU load every 2 s for the whole life of the VM (build included): total busy %, and how many vCPUs are over 80 % busy
cat > $W/cpu.py <<'PY'
import time, sys
def rd():
    d = {}
    for l in open('/proc/stat'):
        if l.startswith('cpu'):
            p = l.split(); v = list(map(int, p[1:9])); d[p[0]] = (sum(v), v[3] + v[4])
    return d
a = rd(); out = open(sys.argv[1], 'a', buffering=1); out.write('t,busy_pct,vcpus_over_80,nvcpu\n')
while True:
    time.sleep(2); b = rd(); n = hot = 0
    for k in b:
        if k == 'cpu': continue
        dt = b[k][0] - a[k][0]; di = b[k][1] - a[k][1]; n += 1
        if dt > 0 and 1 - di / dt > 0.8: hot += 1
    dt = b['cpu'][0] - a['cpu'][0]; di = b['cpu'][1] - a['cpu'][1]
    out.write(f"{int(time.time())},{100 * (1 - di / dt) if dt else 0:.0f},{hot},{n}\n"); a = b
PY
python3 $W/cpu.py $W/out/cpu.csv &
export DEBIAN_FRONTEND=noninteractive HOME=/root GIT_LFS_SKIP_SMUDGE=1
apt-get update -qq > $W/apt.log 2>&1
apt-get install -y -qq build-essential git curl pkg-config libssl-dev python3 >> $W/apt.log 2>&1 || log "apt failed"
curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain none > $W/rustup.log 2>&1
. /root/.cargo/env
mkdir -p repo && cd repo && git init -q . && git remote add origin "$REPO_URL"
if ! git fetch -q --depth 1 origin "$REF" > $W/git.log 2>&1; then log "git fetch of $REF failed"; fi
git checkout -q FETCH_HEAD >> $W/git.log 2>&1 || log "checkout failed"
cd $W/repo
T0=$(date +%s)
log "building at $(git rev-parse --short HEAD)"
cargo build --release --locked --example train > $W/build.log 2>&1
rc=$?
if [ $rc -ne 0 ]; then
  log "BUILD FAILED rc=$rc"; echo "build failed rc=$rc" > $W/out/STATUS.txt; snap; echo build-failed > $W/done.txt; put $W/done.txt DONE; echo "@@DONE build-failed" | tty; sleep 900; shutdown -h now; exit 1
fi
log "built in $(( $(date +%s) - T0 )) s"
SEED=$(curl -sf -H "Metadata-Flavor: Google" "http://metadata.google.internal/computeMetadata/v1/instance/attributes/seed-out" || true)
if [ -n "$SEED" ]; then
  if echo "$SEED" | base64 -d | tar -xz -C $W/out; then log "seeded out folder at $(grep -m1 '^next ' $W/out/state.txt)"; else log "SEED FAILED"; fi
fi
if [ -n "$SEED" ]; then
  # a resume must be built from the same code as the leg it continues (brain lane / coordinator review of PR 728, S1):
  # the trainer cannot see a scoring change that keeps the arena line and the parts. Same rule as the Actions step.
  PREV=$(cat $W/out/STATUS-earlier.txt $W/out/STATUS.txt 2>/dev/null | grep -h '^built from:' | tail -1 | awk '{print $3}')
  HEADSHA=$(git rev-parse HEAD)
  WHY=""
  if [ -z "$PREV" ]; then WHY="seed has no 'built from:' line to compare"
  elif [ "$PREV" != "$HEADSHA" ]; then
    if ! git fetch -q --depth 1 origin "$PREV" >> $W/git.log 2>&1; then WHY="cannot fetch the earlier leg's commit $PREV"
    elif ! git diff --quiet "$PREV" "$HEADSHA" -- src examples assets Cargo.toml Cargo.lock build.rs rust-toolchain.toml; then
      WHY="code differs between the earlier leg ($PREV) and this build ($HEADSHA)"
    fi
  fi
  if [ -n "$WHY" ]; then
    log "RESUME REFUSED: $WHY"
    { echo "name: $NAME"; echo "resume refused: $WHY"; echo "started: $(date -u +%FT%TZ)"; } > $W/out/STATUS.txt
    snap; echo resume-refused > $W/done.txt; put $W/done.txt DONE; echo "@@DONE resume-refused" | tty; sleep 900; shutdown -h now; exit 1
  fi
fi
read -r -a ARR <<< "$ARGS"
for i in "${!ARR[@]}"; do
  v="${ARR[$i]#*=}"
  if [ "$v" != "${ARR[$i]}" ] && [[ "$v" == */* ]] && [ -f "$W/repo/$v" ]; then ARR[$i]="${ARR[$i]%%=*}=$W/repo/$v"; fi
done
case " ${ARR[*]} " in *" threads="*) ;; *) ARR+=("threads=$(nproc)");; esac
{ echo "name: $NAME"; echo "built from: $(git rev-parse HEAD)"; echo "vm: $(hostname) spot"; echo "args: ${ARR[*]}"
  echo "build seconds: $(( $(date +%s) - T0 ))"; echo "started: $(date -u +%FT%TZ)"; lscpu | grep -E 'Model name|^CPU\(s\)|Thread|Core'; } > $W/out/STATUS.txt
( while sleep 300; do snap; done ) &
SNAPPID=$!
log "training start"
"$W/repo/target/release/examples/train" "${ARR[@]}" out=$W/out > $W/out/train.log 2>&1
rc=$?
kill $SNAPPID 2>/dev/null
{ echo "finished: $(date -u +%FT%TZ)"; echo "exit code: $rc"; } >> $W/out/STATUS.txt
snap
# stay up after DONE: a stopped VM's serial console is unreadable (400), so a short sleep lost the results once (10-19 timing run).
# The caller (watch) deletes the VM right after its final fetch; this sleep is only the backstop if nobody does.
echo "$rc" > $W/done.txt; put $W/done.txt DONE
echo "@@DONE $rc" | tty
sleep 1500
shutdown -h now
"""


SEED_FILES = ("state.txt", "progress.csv", "holdout.csv", "STATUS.txt", "STATUS-earlier.txt")
SEED_MAX = 200_000  # base64 bytes; a metadata value may hold 256 KB


def seed_tar(path):
    """The out-folder files a resume needs, from a snapshot or results tar, as a fresh .tar.gz (bytes).
    Only SEED_FILES are taken, by base name; the old STATUS.txt is appended to STATUS-earlier.txt."""
    got = {}
    with tarfile.open(path) as t:
        for m in t.getmembers():
            n = m.name.lstrip("./")
            if m.isfile() and n in SEED_FILES:
                got[n] = t.extractfile(m).read()
    if "state.txt" not in got:
        sys.exit(f"--seed {path}: no state.txt in it (a snapshot from before state.txt was kept cannot be resumed)")
    if "STATUS.txt" in got:
        got["STATUS-earlier.txt"] = got.get("STATUS-earlier.txt", b"") + got.pop("STATUS.txt")
    buf = io.BytesIO()
    with tarfile.open(fileobj=buf, mode="w:gz") as t:
        for n, data in sorted(got.items()):
            info = tarfile.TarInfo(n); info.size = len(data); info.mtime = int(time.time())
            t.addfile(info, io.BytesIO(data))
    return buf.getvalue()


def _creds():
    info = json.loads(base64.b64decode(os.environ["GCP_SA_KEY_B64"]))
    return info, service_account.Credentials.from_service_account_info(
        info, scopes=["https://www.googleapis.com/auth/cloud-platform"])


_session = None
def gcs():
    """one authorised HTTP session for the bucket (the launcher's own key; plain REST, no extra library)"""
    global _session
    if _session is None:
        from google.auth.transport.requests import AuthorizedSession
        _session = AuthorizedSession(_creds()[1])
    return _session


def bucket_ok():
    """True when the bucket exists and the launcher can list it"""
    try:
        return gcs().get(f"{GCS}/{BUCKET}/o", params={"maxResults": 1}, timeout=30).status_code == 200
    except Exception:
        return False


def bucket_get(job, obj):
    """bytes of runs/<job>/<obj>, or None if it is not there (or the bucket cannot be read)"""
    try:
        r = gcs().get(f"{GCS}/{BUCKET}/o/{urllib.parse.quote(f'runs/{job}/{obj}', safe='')}", params={"alt": "media"}, timeout=120)
    except Exception as e:
        print("bucket read error", type(e).__name__, str(e)[:120]); return None
    return r.content if r.status_code == 200 else None


def clients():
    info, creds = _creds()
    project = info["project_id"]  # the key's own project: GCP_PROJECT may hold the display name
    return project, compute_v1.InstancesClient(credentials=creds), compute_v1.ZoneOperationsClient(credentials=creds)


def find(inst, project, name):
    for z in ZONES:
        try:
            return z, inst.get(project=project, zone=z, instance=name)
        except Exception:
            continue
    return None, None


def vm_name(job):
    n = "pixel-train-" + job
    if not NAME_RE.match(job):
        sys.exit("job name must be lowercase letters, digits, dashes")
    return n


def check_args(args):
    for tok in args.split():
        if "=" not in tok:
            sys.exit(f"argument {tok!r} is not key=value")
        k, v = tok.split("=", 1)
        if k == "out":
            sys.exit("out= is set by the VM")
        if k == "every" and v != "1":
            sys.exit("every must be 1 (curves and hold-out every generation)")
        if v.startswith("/") or ".." in v or "'" in v:
            sys.exit(f"{tok!r}: use a path inside the repo, no quotes")


def cmd_start(a):
    check_args(a.args)
    seed = ""
    if a.seed:
        if " resume=1 " not in f" {a.args} ":
            sys.exit("--seed carries a run on: put resume=1 (and the same arguments, gens= the total) in TRAIN ARGS")
        seed = base64.b64encode(seed_tar(a.seed)).decode()
        if len(seed) > SEED_MAX:
            sys.exit(f"--seed: {len(seed)} base64 bytes, over the {SEED_MAX} a metadata value holds")
        print(f"seeding the out folder from {a.seed} ({len(seed)} base64 bytes)")
    if not re.match(r"^[A-Za-z0-9._/-]{1,100}$", a.ref) or ".." in a.ref:
        sys.exit("bad ref")
    project, inst, ops = clients()
    name = vm_name(a.name)
    use_bucket = bucket_ok()
    if use_bucket:
        if not a.seed and (bucket_get(a.name, "latest.tgz") or bucket_get(a.name, "DONE")):
            sys.exit(f"runs/{a.name} already holds results in the bucket: pick a new job name, or continue it (watch resumes it)")
    else:
        print(f"NOTE: bucket {BUCKET} not reachable: results come back over the serial console only (a takeaway loses the run)")
    script = (STARTUP.replace("@NAME@", a.name).replace("@REF@", a.ref).replace("@ARGS@", a.args)
              .replace('"$REPO_URL"', REPO))
    for zone in ZONES:
        vm = compute_v1.Instance(
            name=name, machine_type=f"zones/{zone}/machineTypes/{a.type}",
            disks=[compute_v1.AttachedDisk(
                boot=True, auto_delete=True,
                initialize_params=compute_v1.AttachedDiskInitializeParams(
                    source_image="projects/ubuntu-os-cloud/global/images/family/ubuntu-2404-lts-amd64",
                    disk_size_gb=getattr(a, "disk", 60), disk_type=f"zones/{zone}/diskTypes/hyperdisk-balanced"))],
            network_interfaces=[compute_v1.NetworkInterface(
                network="global/networks/default",
                access_configs=[compute_v1.AccessConfig(name="External NAT", type_="ONE_TO_ONE_NAT")])],
            scheduling=compute_v1.Scheduling(
                provisioning_model="SPOT", instance_termination_action="DELETE",
                on_host_maintenance="TERMINATE", automatic_restart=False,
                max_run_duration=compute_v1.Duration(seconds=int((a.hours + 1) * 3600))),
            service_accounts=([compute_v1.ServiceAccount(email=WRITER.format(project=project),
                                                         scopes=["https://www.googleapis.com/auth/devstorage.read_write"])]
                              if use_bucket else []),
            metadata=compute_v1.Metadata(items=[compute_v1.Items(key="startup-script", value=script)]
                                         + ([compute_v1.Items(key="seed-out", value=seed)] if seed else [])
                                         + ([compute_v1.Items(key="bucket", value=BUCKET)] if use_bucket else [])),
            labels={"purpose": "pixel-train", "job": a.name[:60]})
        try:
            op = inst.insert(project=project, zone=zone, instance_resource=vm)
            res = ops.wait(project=project, zone=zone, operation=op.name)  # the finished operation carries the error, not the insert's reply
            if res.error and res.error.errors:
                msgs = [f"{e.code}: {e.message}" for e in res.error.errors]
                print(zone, "error:", msgs)
                if any("SERVICE_ACCOUNT_ACCESS_DENIED" in m for m in msgs):
                    sys.exit(f"not started: {WRITER.format(project=project)} cannot be used by the launcher: grant the launcher 'Service Account User' on it")
                continue
            print(f"started {name} type {a.type} zone {zone} ref {a.ref} (expect ~3 min to build, then a snapshot every 5 min)")
            return
        except Exception as e:
            print(zone, type(e).__name__, str(e)[:240])
            if "quota" in str(e).lower() or "permission" in str(e).lower():
                break
    sys.exit("not started")


def read_serial(inst, project, zone, name):
    out, start = [], 0
    for _ in range(60):
        r = inst.get_serial_port_output(request=compute_v1.GetSerialPortOutputInstanceRequest(
            project=project, zone=zone, instance=name, start=start))
        if not r.contents:
            break
        out.append(r.contents)
        if r.next_ == start:
            break
        start = r.next_
    return "".join(out)


def last_snapshot(text):
    """newest complete SNAP block as (header, bytes) or None"""
    best = None
    for m in re.finditer(r"@@SNAP-BEGIN (\S+) (\S+) (\d+)\r?\n((?:@@B [^\r\n]*\r?\n)+)@@SNAP-END", text):
        raw = base64.b64decode("".join(l[4:].strip() for l in m.group(4).splitlines()))
        if len(raw) == int(m.group(3)):
            best = (f"{m.group(1)} {m.group(2)}", raw)
    return best


def cmd_fetch(a):
    project, inst, _ = clients()
    zone, vm = find(inst, project, vm_name(a.name))
    if not vm and not (bucket_ok() and bucket_get(a.name, "latest.tgz")):
        sys.exit("no such VM and nothing in the bucket (finished and fetched, deleted, or never started)")
    return do_fetch(a, inst, project, zone, vm)


def do_fetch(a, inst, project, zone, vm):
    """save the newest snapshot; returns the serial text"""
    text = ""
    if vm:
        try:
            text = read_serial(inst, project, zone, vm.name)
        except Exception as e:  # a stopped VM's console is unreadable; the bucket copy below still is
            print("serial unreadable:", type(e).__name__, str(e)[:100])
    snap = last_snapshot(text)
    logs = [l for l in text.splitlines() if "@@LOG" in l or "@@DONE" in l]
    print("vm", vm.status if vm else "gone", "| log:", " / ".join(l[l.index("@@"):] for l in logs[-4:]))
    raw = bucket_get(a.name, "latest.tgz")  # the durable copy wins over the console's
    if raw:
        try:
            tarfile.open(fileobj=io.BytesIO(raw)).getmembers()
            snap = ("bucket latest", raw)
        except Exception as e:
            print("bucket snapshot unreadable:", type(e).__name__, str(e)[:100])
    done = bucket_get(a.name, "DONE")
    if done and "@@DONE" not in text:
        text += f"\n@@DONE {done.decode(errors='replace').strip() or '?'}\n"
    if not snap:
        print("no complete snapshot yet"); return text
    dest = a.dest or f"/mnt/project-files/brain-training/gcp-runs/{a.name}"
    os.makedirs(dest, exist_ok=True)
    open(f"{dest}/{a.name}-snap.tar.gz", "wb").write(snap[1])
    with tarfile.open(fileobj=io.BytesIO(snap[1])) as t:
        for member in t.getmembers():
            if member.isfile() and member.name.lstrip("./") in ("STATUS.txt", "train.log", "hold-best.txt", "build-tail.log", "state.txt", "cpu.csv"):
                open(f"{dest}/{os.path.basename(member.name)}", "wb").write(t.extractfile(member).read())
        if "curves.svg" in [m.name.lstrip("./") for m in t.getmembers()]:
            svg = t.extractfile("./curves.svg").read()
            try:
                import cairosvg
                cairosvg.svg2png(bytestring=svg, write_to=f"{dest}/curves.png", scale=1.5)
            except Exception:
                open(f"{dest}/curves.svg", "wb").write(svg)
    print("snapshot", snap[0], "saved to", dest)
    if os.path.exists(f"{dest}/cpu.csv"):
        print(cpu_summary(f"{dest}/cpu.csv"))
    for l in open(f"{dest}/STATUS.txt"):
        print("  ", l.rstrip())
    if os.path.exists(f"{dest}/train.log"):
        gl = [l.rstrip() for l in open(f"{dest}/train.log") if re.match(r"^ *\d+ +-?[0-9.]+ ", l)]
        print("   last generation lines:"); [print("   ", l) for l in gl[-3:]]
    return text


def cpu_summary(path):
    """How busy the VM was: whole life, and the training part (from the first 2 s sample over 50 % busy to the last).
    `full` = seconds with at least 90 % of the vCPUs over 80 % busy; `thin` = seconds with under half the vCPUs hot."""
    rows = []
    for l in open(path):
        f = l.strip().split(",")
        if len(f) == 4 and f[0].isdigit():
            rows.append((int(f[0]), int(f[1]), int(f[2]), int(f[3])))
    if not rows:
        return "cpu: no samples"
    start = next((i for i, r in enumerate(rows) if r[1] >= 50), 0)
    def line(name, rs):
        if not rs:
            return f"cpu {name}: no samples"
        n = len(rs); nv = rs[0][3]
        full = sum(1 for r in rs if r[2] >= 0.9 * nv) / n
        thin = sum(1 for r in rs if r[2] < 0.5 * nv) / n
        return (f"cpu {name}: {n * 2} s, mean busy {sum(r[1] for r in rs) / n:.0f}%, mean hot vCPUs {sum(r[2] for r in rs) / n:.1f}/{nv}, "
                f"full {100 * full:.0f}% of the time, thin (<half hot) {100 * thin:.0f}%")
    tl = "".join(" .:-=+*#%@"[min(9, r[1] // 10)] for r in rows[start::15])  # one character per 30 s
    return "\n".join([line("whole life", rows), line("training", rows[start:]), f"cpu timeline (30 s per char, ' '=idle '@'=100%): [{tl}]"])


def cmd_status(a):
    project, inst, _ = clients()
    zone, vm = find(inst, project, vm_name(a.name))
    if not vm:
        print("gone (finished, deleted or preempted)"); return
    text = read_serial(inst, project, zone, vm.name)
    logs = [l[l.index("@@"):] for l in text.splitlines() if "@@LOG" in l or "@@DONE" in l]
    snaps = len(re.findall(r"@@SNAP-END", text))
    print(vm.name, vm.status, zone, "| snapshots seen:", snaps, "| log:", " / ".join(logs[-5:]))


def cmd_delete(a):
    project, inst, ops = clients()
    zone, vm = find(inst, project, vm_name(a.name))
    if not vm:
        print("nothing to delete"); return
    ops.wait(project=project, zone=zone, operation=inst.delete(project=project, zone=zone, instance=vm.name).name)
    print("deleted", vm.name)


def cmd_watch(a):
    import types
    project, inst, ops = clients()
    dest = a.dest or f"/mnt/project-files/brain-training/gcp-runs/{a.name}"
    snapfile = f"{dest}/{a.name}-snap.tar.gz"
    restarts, first = 0, True
    while True:
        zone, vm = find(inst, project, vm_name(a.name))
        if not vm:
            donecode = bucket_get(a.name, "DONE") if bucket_ok() else None
            if donecode:  # finished while we were not looking (launcher restarted, or the VM was deleted after DONE)
                print(time.strftime("%H:%M:%S"), "job already finished in the bucket, exit", donecode.decode().strip(), flush=True)
                do_fetch(types.SimpleNamespace(name=a.name, dest=dest), inst, project, zone, None)
                sys.exit(0 if donecode.decode().strip() == "0" else 1)
            raw = bucket_get(a.name, "latest.tgz") if bucket_ok() else None
            if raw:  # resume from the durable copy, not whatever the console last showed
                os.makedirs(dest, exist_ok=True)
                open(snapfile, "wb").write(raw)
            if not first:
                restarts += 1
                print(time.strftime("%H:%M:%S"), f"VM gone before @@DONE: preempted? restart {restarts}/{a.max_restarts}", flush=True)
                if restarts > a.max_restarts:
                    sys.exit("too many restarts")
            sa = types.SimpleNamespace(**vars(a))
            if os.path.exists(snapfile):
                sa.seed = snapfile
                if " resume=1 " not in f" {a.args} ":
                    sa.args = a.args + " resume=1"
            elif a.seed:
                sa.seed = a.seed
            try:
                cmd_start(sa)
            except SystemExit as e:
                if sa.seed and "state.txt" in str(e):  # snapshot without state yet: start over
                    sa.seed, sa.args = None, a.args
                    cmd_start(sa)
                else:
                    raise
            first = False
            time.sleep(60)
            continue
        first = False
        try:
            text = do_fetch(types.SimpleNamespace(name=a.name, dest=dest), inst, project, zone, vm)
        except Exception as e:
            print(time.strftime("%H:%M:%S"), "fetch error", type(e).__name__, str(e)[:150], flush=True)
            time.sleep(a.poll); continue
        m = re.search(r"@@DONE (\S+)", text)
        if m:
            print(time.strftime("%H:%M:%S"), "job finished, exit", m.group(1), "- deleting VM", flush=True)
            do_fetch(types.SimpleNamespace(name=a.name, dest=dest), inst, project, zone, vm)
            ops.wait(project=project, zone=zone, operation=inst.delete(project=project, zone=zone, instance=vm.name).name)
            print("deleted", vm.name)
            sys.exit(0 if m.group(1) == "0" else 1)
        time.sleep(a.poll)


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = p.add_subparsers(dest="cmd", required=True)
    s = sub.add_parser("start"); s.add_argument("name"); s.add_argument("args")
    s.add_argument("--ref", default="main"); s.add_argument("--type", default="c4d-highcpu-32")
    s.add_argument("--hours", type=float, default=6)
    s.add_argument("--seed", help="a .tar.gz holding state.txt (fetch's NAME-snap.tar.gz or a results branch): carry that run on")
    w = sub.add_parser("watch"); w.add_argument("name"); w.add_argument("args")
    w.add_argument("--ref", default="main"); w.add_argument("--type", default="c4d-highcpu-32")
    w.add_argument("--hours", type=float, default=6); w.add_argument("--seed")
    w.add_argument("--dest"); w.add_argument("--poll", type=int, default=240); w.add_argument("--max-restarts", type=int, default=3)
    f = sub.add_parser("fetch"); f.add_argument("name"); f.add_argument("dest", nargs="?")
    for c in ("status", "delete"):
        sub.add_parser(c).add_argument("name")
    a = p.parse_args()
    {"start": cmd_start, "watch": cmd_watch, "fetch": cmd_fetch, "status": cmd_status, "delete": cmd_delete}[a.cmd](a)


if __name__ == "__main__":
    main()
