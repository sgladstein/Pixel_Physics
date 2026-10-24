import gzip, io, os, re, sys, tarfile
sys.path.insert(0, '/home/user/shortr/Reports/brain-training/cloud-compute/gcp'); import gcp_train_job as g
OUT, B, JOB = sys.argv[1], sys.argv[2], sys.argv[3]; RS = set(map(int, sys.argv[4].split(',')))
for seed in sys.argv[5:]:
    t = tarfile.open(fileobj=io.BytesIO(g.bucket_get(B, f'jobs/{JOB}_{seed}.tgz')))
    for m in t.getmembers():
        mm = re.match(r'[^/]+/r(\d+)/(events\.csv\.gz|nest\.csv)$', m.name)
        if mm and int(mm.group(1)) in RS:
            p = f'{OUT}/{JOB}_{seed}/r{mm.group(1)}/{mm.group(2)}'.removesuffix('.gz'); os.makedirs(os.path.dirname(p), exist_ok=True)
            b = t.extractfile(m).read(); open(p, 'wb').write(gzip.decompress(b) if m.name.endswith('.gz') else b)
    print(seed, flush=True)
