import sys, hashlib, os
from PIL import Image

route, name = sys.argv[1], sys.argv[2]
pa = f'target/dm_{name}_on/{route}/rust.png'
pb = f'target/dm_{name}_off/{route}/rust.png'
if not (os.path.exists(pa) and os.path.exists(pb)):
    print("   MISSING", os.path.exists(pa), os.path.exists(pb))
    raise SystemExit(1)
ha = hashlib.md5(open(pa, 'rb').read()).hexdigest()
hb = hashlib.md5(open(pb, 'rb').read()).hexdigest()
if ha == hb:
    print("   OK  incremental == full (byte identical)")
    raise SystemExit(0)
a = Image.open(pa).convert('RGB')
b = Image.open(pb).convert('RGB')
la, lb = a.load(), b.load()
w, h = a.size
pts = [(x, y) for y in range(h) for x in range(w) if la[x, y] != lb[x, y]]
ys = [p[1] for p in pts]
xs = [p[0] for p in pts]
print(f"   MISMATCH n={len(pts)} x=({min(xs)},{max(xs)}) y=({min(ys)},{max(ys)})")
raise SystemExit(2)
