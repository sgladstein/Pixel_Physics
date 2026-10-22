"""SCRATCH (teacher lane, 2026-10-22). Stream each full job tgz of a cloud batch from the bucket and reduce its event logs to one row per colony-round:
batch,job,round,frames,deliver,deliver_far(13+ cols from door x=128),field,homeeat,hungryout,starved,culled,alive_end.
usage: evreduce.py BATCH OUTCSV"""
import sys, io, csv, tarfile, re, collections, concurrent.futures as cf
sys.path.insert(0, __import__('os').path.join(__import__('os').path.dirname(__file__), '..', 'cloud'))
import importlib.util
spec = importlib.util.spec_from_file_location('ic', __import__('os').path.join(__import__('os').path.dirname(__file__), '..', 'cloud', 'inrun_cloud.py')); ic = importlib.util.module_from_spec(spec); spec.loader.exec_module(ic)
g = ic.g
B, OUT = sys.argv[1], sys.argv[2]
s = g.gcs(); names = []; tok = None
while True:
    p = {"prefix": f"runs/{B}/jobs/", "maxResults": 1000}
    if tok: p["pageToken"] = tok
    r = s.get(f"{g.GCS}/{g.BUCKET}/o", params=p, timeout=60).json()
    names += [i["name"] for i in r.get("items", [])]; tok = r.get("nextPageToken")
    if not tok: break
def one(name):
    rel = name[len(f"runs/{B}/"):]; job = rel[5:-4]; rows = []
    raw = g.bucket_get(B, rel)
    with tarfile.open(fileobj=io.BytesIO(raw)) as t:
        ants = {}
        for m in t.getmembers():
            mm = re.match(r'[^/]+/r(\d+)/ants_f0200000\.txt$', m.name)
            if mm: ants[int(mm.group(1))] = sum(1 for _ in t.extractfile(m))
        for m in t.getmembers():
            mm = re.match(r'[^/]+/r(\d+)/events\.csv$', m.name)
            if not mm: continue
            c = collections.Counter(); last = 0
            for e in csv.DictReader(io.TextIOWrapper(t.extractfile(m))):
                k = e['kind']; last = max(last, int(e['frame']))
                if k == 'deliver':
                    c['deliver'] += 1; c['deliver_far'] += abs(int(e['x']) - 128) >= 13
                elif k in ('field', 'homeeat', 'hungryout', 'tdrop'): c[k] += 1
                elif k == 'death': c['starved'] += e['v'] == '1'; c['culled'] += e['v'] == '4'
            rr = int(mm.group(1))
            rows.append([B, job, rr, last, c['deliver'], c['deliver_far'], c['field'], c['homeeat'], c['hungryout'], c['tdrop'], c['starved'], c['culled'], ants.get(rr, -1)])
    return rows
with open(OUT, 'w', newline='') as f, cf.ThreadPoolExecutor(8) as ex:
    w = csv.writer(f); w.writerow(['batch','job','round','lastframe','deliver','deliver_far','field','homeeat','hungryout','tdrop','starved','culled','alive_end'])
    for rows in ex.map(one, names):
        w.writerows(rows)
print(B, len(names), 'jobs')
