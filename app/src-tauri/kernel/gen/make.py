"""Rebuilds the two GPU files the app ships: ../grind.cl and ../comb.bin.

Run `python make.py` from this folder. Both outputs are checked into the repo, so anyone can run
this and confirm they come out byte for byte the same.

  grind.cl  the OpenCL program: SHA-512, ed25519 base-point multiply, base58, pattern match.
  comb.bin  32 x 256 precomputed multiples of the ed25519 base point, as little-endian int32,
            32 ints per entry (30 used, 2 padding).

Every number in both files is computed by ed25519_ref.py; nothing is typed in by hand. The app does
not trust either file anyway: at startup it derives 1024 keys on the GPU and compares each one with
the CPU, and every address the GPU finds is derived again on the CPU before it is kept.
"""

import hashlib
import os
import struct

import ed25519_ref as ref
import kernelgen

here = os.path.dirname(os.path.abspath(__file__))
out = os.path.dirname(here)

src = kernelgen.build_source().replace("\r\n", "\n")
with open(os.path.join(out, "grind.cl"), "w", encoding="ascii", newline="\n") as f:
    f.write(src)

table = ref.comb_table()
ref.validate_comb_table(table)
blob = bytearray()
for pos in range(32):
    for digit in range(256):
        ypx, ymx, t2d = table[pos][digit]
        blob += struct.pack("<32i", *ypx, *ymx, *t2d, 0, 0)
with open(os.path.join(out, "comb.bin"), "wb") as f:
    f.write(blob)

print("grind.cl", len(src), "bytes", hashlib.sha256(src.encode()).hexdigest())
print("comb.bin", len(blob), "bytes", hashlib.sha256(blob).hexdigest())
