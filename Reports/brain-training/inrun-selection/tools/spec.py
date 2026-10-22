# child-minus-parent score at age 30k, from age30.csv + births.csv of each round of a job archive
import sys, io, tarfile, re, csv, importlib.util, statistics as st
spec=importlib.util.spec_from_file_location('ic','/home/user/cloud/scratch/cloud/inrun_cloud.py'); ic=importlib.util.module_from_spec(spec); spec.loader.exec_module(ic)
B=sys.argv[1]; jobs=sys.argv[2:]; diffs=[]; nfam=[]
for job in jobs:
    t=tarfile.open(fileobj=io.BytesIO(ic.g.bucket_get(B, f'jobs/{job}.tgz')))
    files={m.name:m for m in t.getmembers() if m.isfile()}
    rounds=sorted({int(re.search(r'/r(\d+)/',n).group(1)) for n in files if re.search(r'/r\d+/age30.csv$',n)})
    for r in rounds:
        pre=f'{job}/r{r}/'
        a={x['id']:float(x['score30']) for x in csv.DictReader(io.TextIOWrapper(t.extractfile(files[pre+'age30.csv'])))}
        for b in csv.DictReader(io.TextIOWrapper(t.extractfile(files[pre+'births.csv']))):
            if b['kind']=='birth' and b['id'] in a and b['parent'] in a: diffs.append(a[b['id']]-a[b['parent']])
        if pre+'final/index.csv' in files:
            fam=[x['family'].split('.')[0] for x in csv.DictReader(io.TextIOWrapper(t.extractfile(files[pre+'final/index.csv'])))]
            if fam: nfam.append((r,len(set(fam)),len(fam)))
diffs.sort(); n=len(diffs)
print(B, 'pairs',n,'median child-parent',round(st.median(diffs),2),'mean',round(st.mean(diffs),2),'below parent',f"{sum(d<0 for d in diffs)/n:.0%}",'p10',round(diffs[n//10],2),'p90',round(diffs[9*n//10],2))
import collections; c=collections.defaultdict(list)
for r,k,m in nfam: c[r].append(k)
print('distinct founder lines at round end (median):',{r:st.median(v) for r,v in sorted(c.items())})
