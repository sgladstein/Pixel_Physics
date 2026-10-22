import sys, io, tarfile, re, csv, hashlib, importlib.util, statistics as st
spec=importlib.util.spec_from_file_location('ic','/home/user/cloud/scratch/cloud/inrun_cloud.py'); ic=importlib.util.module_from_spec(spec); spec.loader.exec_module(ic)
B,arm,R,n=sys.argv[1],sys.argv[2],int(sys.argv[3]),int(sys.argv[4]); out=[]
for s in range(1,n+1):
    t=tarfile.open(fileobj=io.BytesIO(ic.g.bucket_get(B,f'jobs/{arm}_{s}.tgz')))
    files={m.name:m for m in t.getmembers() if m.isfile()}
    pre=f'{arm}_{s}/r{R-1}/final/'
    if pre+'index.csv' not in files: continue
    idx=list(csv.DictReader(io.TextIOWrapper(t.extractfile(files[pre+'index.csv']))))
    if not idx: continue
    lines={x['family'].split('.')[0] for x in idx}
    hs=[hashlib.md5(t.extractfile(files[pre+x['id']+'.raw.txt']).read()).hexdigest() for x in idx if pre+x['id']+'.raw.txt' in files]
    top=max(hs.count(h) for h in set(hs))
    out.append((len(idx),len(lines),len(set(hs)),top))
print(B,arm,'round',R-1,'colonies',len(out),'| median ants',st.median(o[0] for o in out),'| founder lines',st.median(o[1] for o in out),'| distinct genomes',st.median(o[2] for o in out),'| most common genome holds',st.median(o[3] for o in out),'ants')
