"""Pure-Python ed25519 reference implementation.

Used for two things:
  1. Baking constants and the precomputed basepoint table into the OpenCL kernel.
  2. CPU-verifying every match the GPU reports before it is saved to disk.

Slow (Python big ints), but only runs on a handful of keys.
"""

import hashlib

P = 2**255 - 19
D = (-121665 * pow(121666, P - 2, P)) % P  # curve constant d


def _inv(x: int) -> int:
    return pow(x, P - 2, P)


def _recover_x(y: int, sign: int) -> int:
    # x^2 = (y^2 - 1) / (d*y^2 + 1)
    xx = (y * y - 1) * _inv(D * y * y + 1) % P
    x = pow(xx, (P + 3) // 8, P)
    if (x * x - xx) % P != 0:
        x = x * pow(2, (P - 1) // 4, P) % P
    if (x * x - xx) % P != 0:
        raise ValueError("not a square")
    if x & 1 != sign:
        x = P - x
    return x


B_Y = 4 * _inv(5) % P
B_X = _recover_x(B_Y, 0)
BASEPOINT = (B_X, B_Y)


def edwards_add(p1, p2):
    x1, y1 = p1
    x2, y2 = p2
    denom = D * x1 * x2 * y1 * y2 % P
    x3 = (x1 * y2 + x2 * y1) * _inv(1 + denom) % P
    y3 = (y1 * y2 + x1 * x2) * _inv(1 - denom) % P
    return (x3, y3)


def scalarmult_base(s: int):
    q = None
    p = BASEPOINT
    while s:
        if s & 1:
            q = p if q is None else edwards_add(q, p)
        p = edwards_add(p, p)
        s >>= 1
    return q


def clamp(digest32: bytes) -> int:
    a = bytearray(digest32)
    a[0] &= 248
    a[31] &= 127
    a[31] |= 64
    return int.from_bytes(a, "little")


def pubkey_from_seed(seed32: bytes) -> bytes:
    """seed (32 bytes) -> ed25519 public key (32 bytes), same as Solana keypairs."""
    h = hashlib.sha512(seed32).digest()
    s = clamp(h[:32])
    x, y = scalarmult_base(s)
    return (y | ((x & 1) << 255)).to_bytes(32, "little")


# ---------------------------------------------------------------------------
# base58 (Bitcoin/Solana alphabet)
# ---------------------------------------------------------------------------

B58_ALPHABET = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"


def b58encode(data: bytes) -> str:
    n = int.from_bytes(data, "big")
    out = ""
    while n:
        n, r = divmod(n, 58)
        out = B58_ALPHABET[r] + out
    pad = 0
    for b in data:
        if b == 0:
            pad += 1
        else:
            break
    return "1" * pad + out


# ---------------------------------------------------------------------------
# Limb packing helpers for the kernel (ref10 radix-25.5: 10 limbs, 26/25 bits)
# ---------------------------------------------------------------------------

LIMB_BITS = [26, 25, 26, 25, 26, 25, 26, 25, 26, 25]


def fe_to_limbs(x: int):
    x %= P
    limbs = []
    for bits in LIMB_BITS:
        limbs.append(x & ((1 << bits) - 1))
        x >>= bits
    assert x == 0
    return limbs


def niels_table():
    """Precomputed 2^i * B for i in 0..254, in niels form (y+x, y-x, 2*d*x*y)."""
    table = []
    p = BASEPOINT
    for _ in range(255):
        x, y = p
        table.append((
            fe_to_limbs(y + x),
            fe_to_limbs(y - x),
            fe_to_limbs(2 * D * x * y),
        ))
        p = edwards_add(p, p)
    return table


def _limbs_to_int(limbs):
    """Inverse of fe_to_limbs: 10 radix-25.5 limbs -> integer mod p."""
    return sum(int(v) << ((51 * i + 1) // 2) for i, v in enumerate(limbs)) % P


def _niels_to_affine(entry):
    """niels (y+x, y-x, 2dxy) limb-triple -> affine (x, y)."""
    ypx, ymx, _t2d = (_limbs_to_int(v) for v in entry)
    inv2 = _inv(2)
    x = (ypx - ymx) * inv2 % P
    y = (ypx + ymx) * inv2 % P
    return (x, y)


def comb_table():
    """Radix-256 fixed-base comb table.

    T[i][j] = (j * 2^(8*i)) * B in niels form (y+x, y-x, 2*d*x*y),
    for i in 0..31 (byte position), j in 0..255 (byte value).
    j == 0 is the identity in niels form (1, 1, 0), so a zero scalar byte
    adds the neutral element (the complete ge_madd handles it exactly).
    Lets the 32 clamped-scalar bytes index directly: 32 additions, no
    doublings, no bit recode, fully warp-uniform.
    """
    table = []
    base_i = BASEPOINT                       # 2^(8*0) * B
    for i in range(32):
        row = [(fe_to_limbs(1), fe_to_limbs(1), fe_to_limbs(0))]  # j=0 identity
        acc = base_i
        for j in range(1, 256):
            x, y = acc
            row.append((fe_to_limbs(y + x), fe_to_limbs(y - x),
                        fe_to_limbs((2 * D * x * y) % P)))
            if j < 255:
                acc = edwards_add(acc, base_i)
        table.append(row)
        for _ in range(8):                   # base_i *= 2^8
            base_i = edwards_add(base_i, base_i)
    return table


def validate_comb_table(table):
    """Exhaustive integrity check - the n-sample self-test cannot cover every
    one of the 8192 entries (coupon-collector), so per-entry correctness is
    proven here at generation time. Raises AssertionError on any bad entry."""
    import random
    inv2 = _inv(2)
    for i in range(32):
        for j in range(256):
            ypx, ymx, t2d = (_limbs_to_int(v) for v in table[i][j])
            x = (ypx - ymx) * inv2 % P
            y = (ypx + ymx) * inv2 % P
            if j == 0:
                assert (x, y) == (0, 1), ("identity", i, j)
                assert t2d == 0, ("identity-t2d", i, j)
                continue
            # on-curve: -x^2 + y^2 == 1 + d*x^2*y^2
            assert (-x * x + y * y - 1 - D * x * x * y * y) % P == 0, ("curve", i, j)
            assert t2d == (2 * D * x * y) % P, ("t2d", i, j)
        # telescoping endpoint: point(T[i][255]) + 2^(8i)*B == 2^(8(i+1))*B
        p255 = _niels_to_affine(table[i][255])
        base_i_pt = scalarmult_base(pow(2, 8 * i))
        assert edwards_add(p255, base_i_pt) == scalarmult_base(pow(2, 8 * (i + 1))), \
            ("telescope", i)
    # independent spot checks against a fresh scalar multiply
    for _ in range(64):
        i = random.randrange(32)
        j = random.randrange(1, 256)
        assert _niels_to_affine(table[i][j]) == scalarmult_base(j * pow(2, 8 * i)), \
            ("spot", i, j)


# ---------------------------------------------------------------------------
# SHA-512 round constants, computed (not transcribed)
# ---------------------------------------------------------------------------

def _primes(n):
    out, c = [], 2
    while len(out) < n:
        if all(c % q for q in out):
            out.append(c)
        c += 1
    return out


def _iroot(n, k):
    """Integer k-th root of n."""
    hi = 1 << ((n.bit_length() + k - 1) // k + 1)
    lo = 0
    while lo < hi - 1:
        mid = (lo + hi) // 2
        if mid**k <= n:
            lo = mid
        else:
            hi = mid
    return lo


def sha512_K():
    return [_iroot(p * (1 << 192), 3) & 0xFFFFFFFFFFFFFFFF for p in _primes(80)]


def sha512_H():
    return [_iroot(p * (1 << 128), 2) & 0xFFFFFFFFFFFFFFFF for p in _primes(8)]


if __name__ == "__main__":
    # sanity: constants must match hashlib / known ed25519 test vector
    assert hashlib.sha512(b"abc").hexdigest().startswith("ddaf35a1")
    K = sha512_K()
    assert K[0] == 0x428A2F98D728AE22, hex(K[0])
    assert sha512_H()[0] == 0x6A09E667F3BCC908
    # RFC 8032 test vector 1: seed -> public key
    seed = bytes.fromhex(
        "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60")
    pub = pubkey_from_seed(seed)
    assert pub.hex() == (
        "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a"), pub.hex()
    print("ed25519_ref self-test OK")
