# SCRATCH (teacher lane): countjobs.py NAME [REGEX] -- how many finished job tgzs a batch has in the bucket (runs/NAME/jobs/).
import os, sys, importlib.util, re
spec=importlib.util.spec_from_file_location('ic',os.path.join(os.path.dirname(os.path.abspath(__file__)), 'inrun_cloud.py')); ic=importlib.util.module_from_spec(spec); spec.loader.exec_module(ic)
g=ic.g; s=g.gcs(); n=0; tok=None; names=[]
while True:
    p={"prefix":f"runs/{sys.argv[1]}/jobs/","maxResults":1000}
    if tok: p["pageToken"]=tok
    r=s.get(f"{g.GCS}/{g.BUCKET}/o",params=p,timeout=60).json(); names+=[i['name'] for i in r.get('items',[])]; tok=r.get('nextPageToken')
    if not tok: break
pat=re.compile(sys.argv[2]) if len(sys.argv)>2 else re.compile('.')
print(sum(1 for x in names if pat.search(x.rsplit('/',1)[1])))
