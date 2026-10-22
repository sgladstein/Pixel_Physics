#!/usr/bin/env python3
"""SCRATCH (teacher lane, never for main): run an in-run selection batch on one Google Cloud spot VM.

Reuses Reports/brain-training/cloud-compute/gcp/gcp_train_job.py (credentials, VM creation, status, delete) and only swaps
the VM's startup script: build examples/inrun at REF, run every line of a jobs file (in the repo at REF) in parallel, one
per vCPU, through scratch/inrun/rounds.sh, and put each finished job's folder to the bucket as runs/NAME/jobs/<job>.tgz.

  start  NAME jobs=scratch/inrun/jobs/X.txt --ref BRANCH [--type c4d-highcpu-48] [--hours 2]
  status NAME          VM state and log lines
  pull   NAME DEST [--which jobs]   download progress, ARMSUM.txt and every finished job's small copy (or full copy) into DEST
  delete NAME          delete the VM (only after pull shows DONE)

A jobs line is: OUT SEED ROUNDS FRAMES SEL GENOME [inrun args...]  (the rounds.sh arguments, OUT a bare name).
Trailing blanks are stripped on the VM: xargs -L joins a line that ends in a blank onto the next one, which once ran
the control arm with the next arm's arguments and dropped that arm (rimshape-1022).
"""
import io, os, sys, tarfile, urllib.parse, argparse
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../Reports/brain-training/cloud-compute/gcp"))
import gcp_train_job as g

g.STARTUP = r"""#!/bin/bash
set -u
W=/root/w; mkdir -p $W/out $W/tgz; cd $W
NAME='@NAME@'; REF='@REF@'; ARGS='@ARGS@'
MD=http://metadata.google.internal/computeMetadata/v1/instance
BUCKET=$(curl -sf -H "Metadata-Flavor: Google" $MD/attributes/bucket || true)
tty() { cat > /dev/ttyS0; }
log() { echo "@@LOG $(date -u +%T) $*" | tty; }
put() {
  [ -n "$BUCKET" ] || return 0
  TOK=$(curl -sf -H "Metadata-Flavor: Google" $MD/service-accounts/default/token | python3 -c 'import sys,json;print(json.load(sys.stdin)["access_token"])') || { log "bucket token failed"; return 1; }
  curl -sf -X POST -H "Authorization: Bearer $TOK" -H "Content-Type: application/octet-stream" --data-binary "@$1" \
    "https://storage.googleapis.com/upload/storage/v1/b/$BUCKET/o?uploadType=media&name=runs%2F$NAME%2F$2" > /dev/null || { log "bucket upload of $2 failed"; return 1; }
}
export -f put log tty; export W MD BUCKET NAME
progress() {
  { echo "name: $NAME"; echo "built from: $(cd $W/repo && git rev-parse HEAD)"; echo "vm: $(hostname) $(nproc) vcpus";
    echo "jobs: $(wc -l < $W/jobs.txt) done: $(cat $W/done.list 2>/dev/null | wc -l)"; echo "at: $(date -u +%FT%TZ)"; } > $W/progress.txt
  put $W/progress.txt progress.txt; tar -C $W -czf $W/latest.tgz progress.txt; put $W/latest.tgz latest.tgz
}
log "boot $(nproc) cpus"
export DEBIAN_FRONTEND=noninteractive HOME=/root GIT_LFS_SKIP_SMUDGE=1
apt-get update -qq > $W/apt.log 2>&1
apt-get install -y -qq build-essential git curl pkg-config libssl-dev python3 >> $W/apt.log 2>&1 || log "apt failed"
curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain none > $W/rustup.log 2>&1
. /root/.cargo/env
mkdir -p repo && cd repo && git init -q . && git remote add origin "$REPO_URL"
git fetch -q --depth 1 origin "$REF" > $W/git.log 2>&1 || log "git fetch of $REF failed"
git checkout -q FETCH_HEAD >> $W/git.log 2>&1 || log "checkout failed"
log "building at $(git rev-parse --short HEAD)"
T0=$(date +%s)
RUSTFLAGS="-C target-cpu=x86-64-v3" cargo build --release --locked --example inrun > $W/build.log 2>&1
rc=$?
if [ $rc -ne 0 ]; then
  log "BUILD FAILED rc=$rc"; tail -n 60 $W/build.log > $W/build-tail.log; put $W/build-tail.log build-tail.log
  echo build-failed > $W/done.txt; put $W/done.txt DONE; echo "@@DONE build-failed" | tty; sleep 600; shutdown -h now; exit 1
fi
log "built in $(( $(date +%s) - T0 )) s"
sed "s/[[:space:]]*$//" "${ARGS#jobs=}" > $W/jobs.txt || { log "no jobs file"; echo no-jobs > $W/done.txt; put $W/done.txt DONE; echo "@@DONE no-jobs" | tty; sleep 600; shutdown -h now; exit 1; }
put $W/jobs.txt jobs.txt
export BIN=$W/repo/target/release/examples/inrun RAYON_NUM_THREADS=1
run_one() {
  cd $W/repo; o=$1; shift
  # `OUT colsel P MODE NCOL ROUNDS FRAMES GENOME args`: one colony-selection driver using P cores (colsel.py)
  if [ "$1" = colsel ]; then P=$2 python3 -I scratch/inrun/colsel.py $W/out/$o "${@:3}" > $W/out/$o.log 2>&1; cp $W/out/$o.log $W/out/$o/driver.log
  else bash scratch/inrun/rounds.sh $W/out/$o "$@"; fi
  tar -C $W/out -czf $W/tgz/$o.tgz $o && put $W/tgz/$o.tgz jobs/$o.tgz && rm -f $W/tgz/$o.tgz
  # the small copy pull fetches: everything but the event log and the 4k-frame maps and ant lists (last one of each kept)
  tar -C $W/out -czf $W/tgz/$o-s.tgz --exclude=events.csv --exclude=events.csv.gz --exclude='map_f0[01]*' --exclude='ants_f0[01]*' $o && put $W/tgz/$o-s.tgz small/$o.tgz && rm -f $W/tgz/$o-s.tgz
  echo $o >> $W/done.list
}
export -f run_one
progress
( while sleep 120; do progress; done ) &
PP=$!
log "batch start: $(wc -l < $W/jobs.txt) jobs on $(nproc) vcpus"
xargs -P $(nproc) -L 1 bash -c 'run_one "$@"' _ < $W/jobs.txt
kill $PP 2>/dev/null
progress
python3 -I $W/repo/scratch/inrun/armsum.py $W/out > $W/ARMSUM.txt 2>&1; put $W/ARMSUM.txt ARMSUM.txt
log "batch done in $(( $(date +%s) - T0 )) s"
echo 0 > $W/done.txt; put $W/done.txt DONE
echo "@@DONE 0" | tty
sleep 1500
shutdown -h now
"""


def cmd_pull(a):
    os.makedirs(a.dest, exist_ok=True)
    s, pre, got, token = g.gcs(), f"runs/{a.name}/", 0, None
    while True:
        params = {"prefix": pre, "maxResults": 1000}
        if token:
            params["pageToken"] = token
        r = s.get(f"{g.GCS}/{g.BUCKET}/o", params=params, timeout=60).json()
        for it in r.get("items", []):
            rel = it["name"][len(pre):]
            if rel.startswith(a.which + "/") and rel.endswith(".tgz"):
                job = rel[len(a.which) + 1:-4]
                if os.path.isdir(f"{a.dest}/{job}"):
                    continue
                raw = g.bucket_get(a.name, rel)
                with tarfile.open(fileobj=io.BytesIO(raw)) as t:
                    for m in t.getmembers():  # our own VM's tars; still refuse anything that would land outside DEST
                        if m.name.startswith("/") or ".." in m.name.split("/") or not (m.isfile() or m.isdir()):
                            sys.exit(f"refusing member {m.name!r}")
                    t.extractall(a.dest)
                got += 1
            elif rel in ("progress.txt", "DONE", "build-tail.log", "jobs.txt", "ARMSUM.txt"):
                open(f"{a.dest}/{rel}", "wb").write(g.bucket_get(a.name, rel))
        token = r.get("nextPageToken")
        if not token:
            break
    print(f"pulled {got} new jobs into {a.dest}")
    for f in ("progress.txt", "DONE"):
        if os.path.exists(f"{a.dest}/{f}"):
            print(f, ":", open(f"{a.dest}/{f}").read().strip().replace("\n", " | "))


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = p.add_subparsers(dest="cmd", required=True)
    s = sub.add_parser("start"); s.add_argument("name"); s.add_argument("args")
    s.add_argument("--ref", required=True); s.add_argument("--type", default="c4d-highcpu-48")
    s.add_argument("--hours", type=float, default=2); s.add_argument("--disk", type=int, default=60, help="boot disk GB (long runs: 200)"); s.add_argument("--seed", default=None)
    pl = sub.add_parser("pull"); pl.add_argument("name"); pl.add_argument("dest")
    pl.add_argument("--which", default="small", choices=("small", "jobs"), help="small: no event logs (default); jobs: everything")
    for c in ("status", "delete"):
        sub.add_parser(c).add_argument("name")
    a = p.parse_args()
    {"start": g.cmd_start, "status": g.cmd_status, "delete": g.cmd_delete, "pull": cmd_pull}[a.cmd](a)


if __name__ == "__main__":
    main()
