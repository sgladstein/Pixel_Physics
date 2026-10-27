"""Stream a batch's small/*.tgz from the bucket; keep only the compact tables. usage: fetch.py BATCH OUTDIR"""
import sys, io, os, re, csv, tarfile, concurrent.futures as cf
sys.path.insert(0, "/home/user/Pixel_Physics/Reports/brain-training/cloud-compute/gcp")
import gcp_train_job as g
B, OUT = sys.argv[1], sys.argv[2]
os.makedirs(OUT, exist_ok=True)
s, tok, names = g.gcs(), None, []
while True:
    p = {"prefix": f"runs/{B}/", "maxResults": 1000}
    if tok: p["pageToken"] = tok
    r = s.get(f"{g.GCS}/{g.BUCKET}/o", params=p, timeout=60).json()
    names += [i["name"][len(f"runs/{B}/"):] for i in r.get("items", [])]
    tok = r.get("nextPageToken")
    if not tok: break
for f in ("jobs.txt", "ARMSUM.txt", "progress.txt"):
    if f in names:
        open(f"{OUT}/{f}", "wb").write(g.bucket_get(B, f))
small = [n for n in names if n.startswith("small/") and n.endswith(".tgz")]
DONE = re.compile(rb"done: (\d+) frames in ([\d.]+)s")
def one(rel):
    job = rel[6:-4]
    if os.path.exists(f"{OUT}/{job}.ok"): return job, "skip"
    raw = g.bucket_get(B, rel)
    tim, inrun = [], []
    with tarfile.open(fileobj=io.BytesIO(raw), mode="r:gz") as t:
        for m in t:
            if not m.isfile(): 
                continue
            n = m.name
            if n.endswith("/colsel.csv") or n.endswith("/cg.csv"):
                os.makedirs(f"{OUT}/{job}", exist_ok=True)
                open(f"{OUT}/{job}/{os.path.basename(n)}", "wb").write(t.extractfile(m).read())
            elif n.endswith(".log"):
                mm = re.search(r"/(cg/r(\d+)/c(\d+)|c(\d+)/r(\d+))\.log$", n)
                d = DONE.findall(t.extractfile(m).read())
                if mm and d:
                    kind = "cg" if mm.group(2) else "train"
                    rnd = mm.group(2) or mm.group(5); col = mm.group(3) or mm.group(4)
                    tim.append((kind, rnd, col, d[-1][0].decode(), d[-1][1].decode()))
            else:
                mm = re.match(r"[^/]+/r(\d+)/(nest|stock)\.csv$", n)
                if mm:
                    rows = list(csv.DictReader(io.TextIOWrapper(t.extractfile(m))))
                    if rows:
                        last = rows[-1]
                        inrun.append((mm.group(1), mm.group(2), ";".join(f"{k}={v}" for k, v in last.items())))
                elif re.match(r"[^/]+/EXTINCT$", n):
                    inrun.append(("-", "EXTINCT", "1"))
    if tim:
        with open(f"{OUT}/{job}.timing.csv", "w") as f:
            f.write("kind,round,colony,frames,secs\n"); f.writelines(",".join(x) + "\n" for x in tim)
    if inrun:
        with open(f"{OUT}/{job}.inrun.csv", "w") as f:
            f.write("round,file,vals\n"); f.writelines(",".join(x) + "\n" for x in inrun)
    open(f"{OUT}/{job}.ok", "w").close()
    return job, len(raw)
with cf.ThreadPoolExecutor(int(os.environ.get("TP", "8"))) as ex:
    n = 0
    for job, sz in ex.map(one, small):
        n += 1
print(B, "jobs", n)
