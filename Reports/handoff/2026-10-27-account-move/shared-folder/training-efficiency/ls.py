import sys, collections
sys.path.insert(0, "/home/user/Pixel_Physics/Reports/brain-training/cloud-compute/gcp")
import gcp_train_job as g
s, tok = g.gcs(), None
runs = collections.defaultdict(lambda: [0,0,set()])
while True:
    p = {"prefix": "runs/", "maxResults": 1000}
    if tok: p["pageToken"] = tok
    r = s.get(f"{g.GCS}/{g.BUCKET}/o", params=p, timeout=60).json()
    for it in r.get("items", []):
        parts = it["name"].split("/")
        n = parts[1]; runs[n][0] += 1; runs[n][1] += int(it["size"])
        if len(parts) > 2: runs[n][2].add(parts[2])
    tok = r.get("nextPageToken")
    if not tok: break
for n,(c,b,sub) in sorted(runs.items()):
    print(f"{n:30s} {c:6d} {b/1e6:9.1f}MB {sorted(sub)[:8]}")
