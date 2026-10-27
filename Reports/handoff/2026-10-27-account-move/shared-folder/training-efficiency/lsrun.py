import sys
sys.path.insert(0, "/home/user/Pixel_Physics/Reports/brain-training/cloud-compute/gcp")
import gcp_train_job as g
s, tok = g.gcs(), None
name = sys.argv[1]
while True:
    p = {"prefix": f"runs/{name}/", "maxResults": 1000}
    if tok: p["pageToken"] = tok
    r = s.get(f"{g.GCS}/{g.BUCKET}/o", params=p, timeout=60).json()
    for it in r.get("items", []):
        print(it["name"], it["size"])
    tok = r.get("nextPageToken")
    if not tok: break
