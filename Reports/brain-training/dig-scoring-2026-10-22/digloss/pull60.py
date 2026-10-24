import gzip, sys, io, re, tarfile, os
sys.path.insert(0, '/home/user/nestzone/Reports/brain-training/cloud-compute/gcp'); import gcp_train_job as g
B, OUT = 'night2-1022', sys.argv[1]
for seed in sys.argv[2:]:
    raw = g.bucket_get(B, f'jobs/L60c1_{seed}.tgz')
    with tarfile.open(fileobj=io.BytesIO(raw)) as t:
        for m in t.getmembers():
            mm = re.match(r'[^/]+/r(\d+)/(events\.csv\.gz|births\.csv|nest\.csv|sel\.csv|ants\.csv|age30\.csv|final/index\.csv)$', m.name)
            if mm and int(mm.group(1)) in (0,1,2,3,4,5,6,8,10,15,20):
                dest = f'{OUT}/s{seed}/r{mm.group(1)}/{mm.group(2)}'; os.makedirs(os.path.dirname(dest), exist_ok=True)
                b=t.extractfile(m).read(); b=gzip.decompress(b) if dest.endswith('.gz') else b; open(dest.removesuffix('.gz'),'wb').write(b)
    print(seed, flush=True)
