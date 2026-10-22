# fetch only each job's last-round final/ dir from the bucket
import sys, io, tarfile, re, importlib.util, os
spec=importlib.util.spec_from_file_location('ic','/home/user/cloud/scratch/cloud/inrun_cloud.py'); ic=importlib.util.module_from_spec(spec); spec.loader.exec_module(ic)
B, job, R, dest = sys.argv[1], sys.argv[2], int(sys.argv[3]), sys.argv[4]
raw=ic.g.bucket_get(B, f'jobs/{job}.tgz')
with tarfile.open(fileobj=io.BytesIO(raw)) as t:
    ms=[m for m in t.getmembers() if re.match(rf'[^/]+/r{R-1}/final/', m.name) and m.isfile()]
    for m in ms:
        out=os.path.join(dest, os.path.basename(m.name)); os.makedirs(dest, exist_ok=True)
        open(out,'wb').write(t.extractfile(m).read())
print(job, len(ms))
