import sys, os
sys.path.insert(0, "/home/user/Pixel_Physics/Reports/brain-training/cloud-compute/gcp")
import gcp_train_job as g
name, obj, dest = sys.argv[1:4]
os.makedirs(os.path.dirname(dest) or ".", exist_ok=True)
open(dest, "wb").write(g.bucket_get(name, obj))
