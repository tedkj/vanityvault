"""Generates the OpenCL kernel source for the Solana vanity grinder.

Everything numeric - SHA-512 constants, ed25519 curve constants, the
precomputed 2^i*B niels table, the field-multiply coefficient pattern -
is computed by ed25519_ref.py and injected as text, never transcribed by hand.
"""

import ed25519_ref as ref

MAX_PAT = 16          # max prefix/suffix length in chars
MATCH_STRIDE = 112    # seed[32] + pub[32] + addrlen[1] + addr[44] + pad[3]
K = 32                # keys per Montgomery batched-inversion group (one field
                      # inversion per K keys). 32 is the RTX 5080 sweet spot:
                      # OpenCL 135->143 M/s, CUDA ~157. Bigger K = fewer
                      # inversions but more per-work-item local memory; drop
                      # back toward 16 on GPUs with a smaller register file / L1.


# ---------------------------------------------------------------------------
# Generated field arithmetic (radix 2^25.5, 10 signed int32 limbs)
# ---------------------------------------------------------------------------

# limb weights in bits: e_i = ceil(25.5 * i) for i in 0..9
_E = [ (51 * i + 1) // 2 for i in range(10) ]  # 0,26,51,77,102,128,153,179,204,230

# Max operand magnitude per limb entering fe_mul/fe_sq (ref10 envelope,
# ~1.65*2^bits), even limbs 26-bit, odd 25-bit. Bounds the prescale split.
MAXB = [(1651 * (1 << 26)) // 1000 + 1 if i % 2 == 0
        else (1651 * (1 << 25)) // 1000 + 1 for i in range(10)]


def _split_coeff(c, ia, ib):
    """Distribute the integer factors of coefficient c (some 2s and one
    optional 19) across the two operands (limb ia, limb ib) so BOTH prescaled
    int32 operands stay < 2^31. Returns (scale_a, scale_b). Any valid split
    yields the identical product value; this just keeps every IMAD.WIDE input
    in range (the fe_sq c=38-on-an-even-limb overflow trap)."""
    from itertools import product as _prod
    fac, cc = [], c
    while cc % 19 == 0:
        fac.append(19); cc //= 19
    while cc % 2 == 0:
        fac.append(2); cc //= 2
    assert cc == 1, c
    LIM = 1 << 31
    for assign in _prod((0, 1), repeat=len(fac)):
        sa = sb = 1
        for f, a in zip(fac, assign):
            if a == 0:
                sa *= f
            else:
                sb *= f
        if sa * MAXB[ia] < LIM and sb * MAXB[ib] < LIM:
            return sa, sb
    raise AssertionError(("no int32-safe coefficient split", c, ia, ib))


def _gen_mul_terms(square: bool):
    """Coefficient pattern for fe_mul / fe_sq, derived from limb weights.

    value = sum f_i * 2^e_i, e_i = ceil(25.5*i). A product term f_i*g_j lands
    in limb k=(i+j) mod 10; the leftover exponent
        e_i + e_j - e_k - (255 if i+j>=10 else 0)
    is always 0 or 1, giving an integer factor of 1 or 2. On wrap (i+j>=10)
    an extra x19 comes from 2^255 == 19 (mod p). Squaring folds the symmetric
    f_i*f_j / f_j*f_i pair with an extra x2 for i<j.
    """
    terms = {k: [] for k in range(10)}
    for i in range(10):
        for j in range(10):
            if square and j < i:
                continue
            k = i + j
            wrap = k >= 10
            if wrap:
                k -= 10
            exp = _E[i] + _E[j] - _E[k] - (255 if wrap else 0)
            assert exp in (0, 1), (i, j, exp)
            c = (2 ** exp) * (19 if wrap else 1)
            if square and i < j:
                c *= 2
            terms[k].append((c, i, j))
    return terms


def _emit_products(terms, gname):
    """Emit each product term as a single (long)a*b (one IMAD.WIDE), with a
    preamble of prescaled int32 operands (19*g[j], 2*f[i], ...) so no 64-bit
    constant multiply survives. Prescaled operands are deduped; for fe_sq
    (gname=='f') both operands share the 'f' namespace so they dedup too."""
    pres = {}                       # (letter, arr, idx, scale) -> varname
    prod = {k: [] for k in range(10)}

    def operand(letter, arr, idx, scale):
        if scale == 1:
            return f"{arr}[{idx}]"
        name = f"{letter}{idx}_{scale}"
        pres[(letter, arr, idx, scale)] = name
        return name

    gletter = "g" if gname == "g" else "f"
    for k in range(10):
        for c, i, j in terms[k]:
            sf, sg = _split_coeff(c, i, j)
            a = operand("f", "f", i, sf)
            b = operand(gletter, gname, j, sg)
            prod[k].append(f"(long){a}*{b}")

    lines = []
    for (letter, arr, idx, scale), name in sorted(pres.items()):
        lines.append(f"  int {name} = {scale}*{arr}[{idx}];")
    for k in range(10):
        lines.append(f"  t[{k}] = " + " + ".join(prod[k]) + ";")
    return "\n".join(lines)


# ref10 12-step two-independent-chain carry (replaces the 21-step schedule).
# The two chains (0->1->2->3->4 and 4->5->6->7->8) dual-issue; the tail
# wraps limb 9 into limb 0 with *19 (2^255 == 19 mod p) and re-settles limb 0.
FE_CARRY = """\
  {
    long c0, c4, c1, c5, c2, c6, c3, c7, c8, c9;
    c0=(t[0]+(1L<<25))>>26; t[1]+=c0; t[0]-=c0<<26;
    c4=(t[4]+(1L<<25))>>26; t[5]+=c4; t[4]-=c4<<26;
    c1=(t[1]+(1L<<24))>>25; t[2]+=c1; t[1]-=c1<<25;
    c5=(t[5]+(1L<<24))>>25; t[6]+=c5; t[5]-=c5<<25;
    c2=(t[2]+(1L<<25))>>26; t[3]+=c2; t[2]-=c2<<26;
    c6=(t[6]+(1L<<25))>>26; t[7]+=c6; t[6]-=c6<<26;
    c3=(t[3]+(1L<<24))>>25; t[4]+=c3; t[3]-=c3<<25;
    c7=(t[7]+(1L<<24))>>25; t[8]+=c7; t[7]-=c7<<25;
    c4=(t[4]+(1L<<25))>>26; t[5]+=c4; t[4]-=c4<<26;
    c8=(t[8]+(1L<<25))>>26; t[9]+=c8; t[8]-=c8<<26;
    c9=(t[9]+(1L<<24))>>25; t[0]+=c9*19; t[9]-=c9<<25;
    c0=(t[0]+(1L<<25))>>26; t[1]+=c0; t[0]-=c0<<26;
  }
  for (int m = 0; m < 10; m++) h[m] = (int)t[m];
"""


def gen_fe_mul():
    return ("static void fe_mul(int h[10], const int f[10], const int g[10]) {\n"
            "  long t[10];\n" + _emit_products(_gen_mul_terms(False), "g")
            + "\n" + FE_CARRY + "}\n")


def gen_fe_sq():
    body = _emit_products(_gen_mul_terms(True), "f")
    return ("static void fe_sq(int h[10], const int f[10]) {\n"
            "  long t[10];\n" + body + "\n" + FE_CARRY + "}\n")


# ref10 addition chain for z^(p-2): 254 squarings + 11 multiplies.
# ('sq', dst, src, n)  => dst = src^(2^n)   ('mul', dst, a, b) => dst = a*b
INV_CHAIN = [
    ('sq', 't0', 'z', 1),   ('sq', 't1', 't0', 2),  ('mul', 't1', 'z', 't1'),
    ('mul', 't0', 't0', 't1'), ('sq', 't2', 't0', 1), ('mul', 't1', 't1', 't2'),
    ('sq', 't2', 't1', 5),  ('mul', 't1', 't2', 't1'), ('sq', 't2', 't1', 10),
    ('mul', 't2', 't2', 't1'), ('sq', 't3', 't2', 20), ('mul', 't2', 't3', 't2'),
    ('sq', 't2', 't2', 10), ('mul', 't1', 't2', 't1'), ('sq', 't2', 't1', 50),
    ('mul', 't2', 't2', 't1'), ('sq', 't3', 't2', 100), ('mul', 't2', 't3', 't2'),
    ('sq', 't2', 't2', 50), ('mul', 't1', 't2', 't1'), ('sq', 't1', 't1', 5),
    ('mul', 'out', 't1', 't0'),
]


def gen_fe_invert():
    L = ["static void fe_invert(int out[10], const int z[10]) {",
         "  int t0[10], t1[10], t2[10], t3[10];"]
    for op in INV_CHAIN:
        if op[0] == 'sq':
            _, d, s, nq = op
            L.append(f"  fe_sq({d}, {s});")
            for _ in range(nq - 1):
                L.append(f"  fe_sq({d}, {d});")
        else:
            _, d, a, b = op
            L.append(f"  fe_mul({d}, {a}, {b});")
    L.append("}")
    return "\n".join(L)


def _check_inv_chain():
    """Verify INV_CHAIN computes z^(p-2) mod p (== modular inverse)."""
    import random
    P = ref.P
    for _ in range(200):
        z = random.randrange(1, P)
        env = {'z': z}
        for op in INV_CHAIN:
            if op[0] == 'sq':
                _, d, s, nq = op
                env[d] = pow(env[s], 1 << nq, P)
            else:
                _, d, a, b = op
                env[d] = env[a] * env[b] % P
        assert env['out'] == pow(z, P - 2, P), "INV_CHAIN wrong"


def _carry12(t):
    """Python mirror of FE_CARRY, for the symbolic checker."""
    t = list(t)

    def cr(i, nb, to, m19=False):
        c = (t[i] + (1 << (nb - 1))) >> nb
        t[to] += c * 19 if m19 else c
        t[i] -= c << nb
    cr(0, 26, 1); cr(4, 26, 5); cr(1, 25, 2); cr(5, 25, 6)
    cr(2, 26, 3); cr(6, 26, 7); cr(3, 25, 4); cr(7, 25, 8)
    cr(4, 26, 5); cr(8, 26, 9); cr(9, 25, 0, True); cr(0, 26, 1)
    return t


def _check_fe_arithmetic():
    """Statically prove fe_mul/fe_sq: value equality mod p, int64 accumulation
    bound, int32 prescale bound, and that the 12-step carry preserves value and
    lands every output limb back inside the operand envelope. Runs at build
    time so a bad refactor fails the build, not a silent wrong key on the GPU."""
    import random
    P = ref.P
    for square in (False, True):
        terms = _gen_mul_terms(square)
        for trial in range(3000):
            if trial < 1500:
                f = [random.randint(-MAXB[i], MAXB[i]) for i in range(10)]
            else:  # worst case: every limb at +/- its bound
                f = [random.choice((-MAXB[i], MAXB[i])) for i in range(10)]
            g = f if square else [random.randint(-MAXB[i], MAXB[i]) for i in range(10)]
            t = [sum(c * f[i] * g[j] for (c, i, j) in terms[k]) for k in range(10)]
            for k in range(10):
                assert abs(t[k]) < (1 << 62), ("int64", square, k)
            lhs = sum(t[k] << _E[k] for k in range(10))
            rhs = (sum(f[i] << _E[i] for i in range(10)) *
                   sum(g[j] << _E[j] for j in range(10)))
            assert (lhs - rhs) % P == 0, ("valeq", square, trial)
            h = _carry12(t)
            assert sum(h[k] << _E[k] for k in range(10)) % P == lhs % P, ("carry", square)
            for k in range(10):
                assert abs(h[k]) <= MAXB[k], ("envelope", square, k, h[k])
        # per-term int32 prescale bound
        for k in range(10):
            for (c, i, j) in terms[k]:
                sf, sg = _split_coeff(c, i, j)
                assert sf * MAXB[i] < (1 << 31) and sg * MAXB[j] < (1 << 31), \
                    ("int32", square, c, i, j)


# ---------------------------------------------------------------------------
# Baked constant arrays
# ---------------------------------------------------------------------------

def _fmt_int_list(vals, per_line=10):
    out = []
    for i in range(0, len(vals), per_line):
        out.append(", ".join(str(v) for v in vals[i:i + per_line]))
    return ",\n  ".join(out)


def gen_constants():
    K = ref.sha512_K()
    H = ref.sha512_H()
    sha_k = ",\n  ".join(
        ", ".join(f"0x{v:016X}UL" for v in K[i:i + 4]) for i in range(0, 80, 4))
    sha_h = ", ".join(f"0x{v:016X}UL" for v in H)

    two_p = [2 * (2**26 - 19)] + [2 * (2**b - 1) for b in ref.LIMB_BITS[1:]]

    # p as 8 little-endian 32-bit words, for the final conditional subtract
    p_words = [(ref.P >> (32 * i)) & 0xFFFFFFFF for i in range(8)]

    return f"""
__constant ulong SHA_K[80] = {{
  {sha_k}
}};
__constant ulong SHA_H0[8] = {{ {sha_h} }};

__constant int TWO_P[10] = {{ {', '.join(str(v) for v in two_p)} }};
__constant uint P_WORDS[8] = {{ {', '.join(f'0x{v:08X}U' for v in p_words)} }};
__constant uchar B58_ALPHA[58] = "{ref.B58_ALPHABET}";
"""


# ---------------------------------------------------------------------------
# Static kernel body
# ---------------------------------------------------------------------------

KERNEL_BODY = r"""
/* ------------------------------ SHA-512 (one 128-byte block: 32-byte msg) */

static ulong rotr64(ulong x, uint n) { return (x >> n) | (x << (64 - n)); }

static void sha512_32(const uchar msg[32], uchar out[64]) {
  /* rolling 16-word schedule: unrolled by the compiler so every w[] index is
     compile-time -> registers (no 640-byte w[80] local-memory spill). */
  ulong w[16];
  for (int i = 0; i < 4; i++) {
    ulong v = 0;
    for (int j = 0; j < 8; j++) v = (v << 8) | msg[i * 8 + j];
    w[i] = v;
  }
  w[4] = (ulong)0x80 << 56;
  for (int i = 5; i < 15; i++) w[i] = 0;
  w[15] = 256; /* message length in bits */
  ulong a = SHA_H0[0], b = SHA_H0[1], c = SHA_H0[2], d = SHA_H0[3];
  ulong e = SHA_H0[4], f = SHA_H0[5], g = SHA_H0[6], h = SHA_H0[7];
#pragma unroll
  for (int r = 0; r < 80; r++) {
    int k = r & 15;
    ulong wr;
    if (r < 16) {
      wr = w[k];
    } else {
      ulong a1 = w[(k + 1) & 15], a14 = w[(k + 14) & 15];
      ulong s0 = rotr64(a1, 1) ^ rotr64(a1, 8) ^ (a1 >> 7);
      ulong s1 = rotr64(a14, 19) ^ rotr64(a14, 61) ^ (a14 >> 6);
      w[k] += s0 + w[(k + 9) & 15] + s1;
      wr = w[k];
    }
    ulong S1 = rotr64(e, 14) ^ rotr64(e, 18) ^ rotr64(e, 41);
    ulong ch = (e & f) ^ (~e & g);
    ulong t1 = h + S1 + ch + SHA_K[r] + wr;
    ulong S0 = rotr64(a, 28) ^ rotr64(a, 34) ^ rotr64(a, 39);
    ulong mj = (a & b) ^ (a & c) ^ (b & c);
    h = g; g = f; f = e; e = d + t1; d = c; c = b; b = a; a = t1 + S0 + mj;
  }
  ulong hh[8];
  hh[0] = a + SHA_H0[0]; hh[1] = b + SHA_H0[1];
  hh[2] = c + SHA_H0[2]; hh[3] = d + SHA_H0[3];
  hh[4] = e + SHA_H0[4]; hh[5] = f + SHA_H0[5];
  hh[6] = g + SHA_H0[6]; hh[7] = h + SHA_H0[7];
  for (int i = 0; i < 8; i++)
    for (int j = 0; j < 8; j++)
      out[i * 8 + j] = (uchar)(hh[i] >> (56 - 8 * j));
}

/* ------------------------------ field helpers */

static void fe_add(int h[10], const int f[10], const int g[10]) {
  for (int i = 0; i < 10; i++) h[i] = f[i] + g[i];
}
static void fe_sub(int h[10], const int f[10], const int g[10]) {
  for (int i = 0; i < 10; i++) h[i] = f[i] - g[i];
}
static void fe_copy(int h[10], const int f[10]) {
  for (int i = 0; i < 10; i++) h[i] = f[i];
}
static void fe_0(int h[10]) { for (int i = 0; i < 10; i++) h[i] = 0; }
static void fe_1(int h[10]) { h[0] = 1; for (int i = 1; i < 10; i++) h[i] = 0; }

/* fe_invert (z^(p-2)) is generated by gen_fe_invert() and injected before
   this body - an addition chain, 254 squarings + 11 multiplies. */

/* signed limbs -> canonical 32 little-endian bytes (value mod p) */
static void fe_tobytes(uchar s[32], const int h[10]) {
  long t[10];
  for (int i = 0; i < 10; i++) t[i] = (long)h[i] + TWO_P[i];
  long c;
  for (int r = 0; r < 2; r++) {
    c = t[0] >> 26; t[1] += c; t[0] -= c << 26;
    c = t[1] >> 25; t[2] += c; t[1] -= c << 25;
    c = t[2] >> 26; t[3] += c; t[2] -= c << 26;
    c = t[3] >> 25; t[4] += c; t[3] -= c << 25;
    c = t[4] >> 26; t[5] += c; t[4] -= c << 26;
    c = t[5] >> 25; t[6] += c; t[5] -= c << 25;
    c = t[6] >> 26; t[7] += c; t[6] -= c << 26;
    c = t[7] >> 25; t[8] += c; t[7] -= c << 25;
    c = t[8] >> 26; t[9] += c; t[8] -= c << 26;
    c = t[9] >> 25; t[0] += c * 19; t[9] -= c << 25;
  }
  /* pack 10 limbs (now in [0, 2^26)) into 8 LE 32-bit words */
  uint w[8];
  ulong acc = 0; int accbits = 0, wi = 0;
  const int BITS[10] = {26,25,26,25,26,25,26,25,26,25};
  for (int i = 0; i < 10; i++) {
    acc |= (ulong)((uint)t[i]) << accbits;
    accbits += BITS[i];
    while (accbits >= 32 && wi < 8) {
      w[wi++] = (uint)acc;
      acc >>= 32; accbits -= 32;
    }
  }
  if (wi < 8) w[wi++] = (uint)acc;  /* flush trailing partial word */
  while (wi < 8) w[wi++] = 0;
  /* while >= p: subtract p (at most twice) */
  for (int r = 0; r < 2; r++) {
    uint d[8]; ulong borrow = 0;
    for (int i = 0; i < 8; i++) {
      ulong sub = (ulong)P_WORDS[i] + borrow;
      ulong wv = (ulong)w[i];
      d[i] = (uint)(wv - sub);
      borrow = (wv < sub) ? 1UL : 0UL;
    }
    if (!borrow) for (int i = 0; i < 8; i++) w[i] = d[i];
  }
  for (int i = 0; i < 8; i++) {
    s[i*4]   = (uchar)(w[i]);
    s[i*4+1] = (uchar)(w[i] >> 8);
    s[i*4+2] = (uchar)(w[i] >> 16);
    s[i*4+3] = (uchar)(w[i] >> 24);
  }
}

/* ------------------------------ point arithmetic (radix-256 comb) */

/* accumulator (extended coords) += (digit * 2^(8*pos)) * B, complete mixed add.
   COMB is a __global buffer, 32 ints per niels entry (30 used + 2 pad);
   entry (pos,digit) at COMB + (pos*256 + digit)*32. digit MUST be uint. */
static void ge_madd_comb(int X[10], int Y[10], int Z[10], int T[10],
                         uint pos, uint digit,
                         __global const int * restrict COMB) {
  __global const int * restrict n = COMB + (pos * 256u + digit) * 32u;
  int ypx[10], ymx[10], t2d[10];
#pragma unroll
  for (int i = 0; i < 10; i++) { ypx[i] = n[i]; ymx[i] = n[10+i]; t2d[i] = n[20+i]; }
  int A[10], B[10], C[10], D[10], E[10], F[10], G[10], H[10], t0[10];
  fe_sub(t0, Y, X); fe_mul(A, t0, ymx);
  fe_add(t0, Y, X); fe_mul(B, t0, ypx);
  fe_mul(C, T, t2d);
  fe_add(D, Z, Z);
  fe_sub(E, B, A); fe_sub(F, D, C); fe_add(G, D, C); fe_add(H, B, A);
  fe_mul(X, E, F); fe_mul(Y, G, H); fe_mul(Z, F, G); fe_mul(T, E, H);
}

/* same, but omit the final fe_mul(T,E,H): T is not consumed after the last add */
static void ge_madd_comb_last(int X[10], int Y[10], int Z[10], int T[10],
                              uint pos, uint digit,
                              __global const int * restrict COMB) {
  __global const int * restrict n = COMB + (pos * 256u + digit) * 32u;
  int ypx[10], ymx[10], t2d[10];
#pragma unroll
  for (int i = 0; i < 10; i++) { ypx[i] = n[i]; ymx[i] = n[10+i]; t2d[i] = n[20+i]; }
  int A[10], B[10], C[10], D[10], E[10], F[10], G[10], H[10], t0[10];
  fe_sub(t0, Y, X); fe_mul(A, t0, ymx);
  fe_add(t0, Y, X); fe_mul(B, t0, ypx);
  fe_mul(C, T, t2d);
  fe_add(D, Z, Z);
  fe_sub(E, B, A); fe_sub(F, D, C); fe_add(G, D, C); fe_add(H, B, A);
  fe_mul(X, E, F); fe_mul(Y, G, H); fe_mul(Z, F, G);
}

/* seed -> projective point (X:Y:Z) via 32-byte radix-256 comb (no inversion) */
static void ge_scalarmult_comb(const uchar seed[32],
                               int X[10], int Y[10], int Z[10],
                               __global const int * restrict COMB) {
  uchar dig[64];
  sha512_32(seed, dig);
  uchar s[32];
  for (int i = 0; i < 32; i++) s[i] = dig[i];
  s[0] &= 248; s[31] &= 127; s[31] |= 64;

  int T[10];
  fe_0(X); fe_1(Y); fe_1(Z); fe_0(T);
  for (uint i = 0; i < 31; i++)
    ge_madd_comb(X, Y, Z, T, i, (uint)s[i], COMB);
  ge_madd_comb_last(X, Y, Z, T, 31u, (uint)s[31], COMB);
}

/* K seeds -> K compressed pubkeys with ONE modular inversion (Montgomery's
   trick). Phase A: K scalarmults + running Z-products. Phase B: one invert.
   Phase C: walk back, recovering each 1/Z with 2 muls, then affine y + sign.
   Staging arrays are write-once in A, read-only in C (never aliased). */
static void pubkeys_batch(const uchar seeds[K][32], uchar pubs[K][32],
                          __global const int * restrict COMB) {
  int sX[K][10], sY[K][10], sZ[K][10], pre[K][10];
#pragma unroll 1
  for (int j = 0; j < K; j++) {
    ge_scalarmult_comb(seeds[j], sX[j], sY[j], sZ[j], COMB);
    if (j == 0) fe_copy(pre[0], sZ[0]);
    else        fe_mul(pre[j], pre[j-1], sZ[j]);   /* pre[j] = Z0*...*Zj */
  }
  int inv[10];
  fe_invert(inv, pre[K-1]);                        /* 1/(Z0*...*Z_{K-1}) */
#pragma unroll 1
  for (int j = K-1; j >= 0; j--) {
    int zinv[10];
    if (j > 0) { fe_mul(zinv, inv, pre[j-1]); fe_mul(inv, inv, sZ[j]); }
    else         fe_copy(zinv, inv);
    int x[10], y[10];
    fe_mul(y, sY[j], zinv);
    fe_mul(x, sX[j], zinv);
    uchar xb[32];
    fe_tobytes(pubs[j], y);
    fe_tobytes(xb, x);
    pubs[j][31] |= (xb[0] & 1) << 7;
  }
}

/* ------------------------------ base58 */

static int b58enc(const uchar pub[32], uchar addr[44]) {
  uchar digits[45];
  int ndig = 0;
  for (int i = 0; i < 32; i++) {
    uint carry = pub[i];
    for (int k = 0; k < ndig; k++) {
      carry += (uint)digits[k] << 8;
      digits[k] = (uchar)(carry % 58);
      carry /= 58;
    }
    while (carry) {
      digits[ndig++] = (uchar)(carry % 58);
      carry /= 58;
    }
  }
  int zeros = 0;
  while (zeros < 32 && pub[zeros] == 0) zeros++;
  int len = zeros + ndig;
  for (int i = 0; i < zeros; i++) addr[i] = '1';
  for (int i = 0; i < ndig; i++) addr[zeros + i] = B58_ALPHA[digits[ndig - 1 - i]];
  return len;
}

/* ------------------------------ pattern match (patterns uploaded at runtime,
   so the kernel source is pattern-independent -> compiles once, cache reused).
   PATS layout per pattern (PAT_STRIDE bytes): [pre_len][suf_len][pre MAX_PAT][suf MAX_PAT] */

static uchar norm_c(uchar c, uint ci) {
  return (ci && c >= 'A' && c <= 'Z') ? (uchar)(c + 32) : c;
}

static int matches_any(const uchar addr[44], int len,
                       __global const uchar * restrict PATS, uint npat, uint ci) {
  for (uint p = 0; p < npat; p++) {
    __global const uchar *pp = PATS + p * PAT_STRIDE;
    int pl = pp[0], sl = pp[1];
    if (pl + sl > len) continue;
    __global const uchar *pre = pp + 2;
    __global const uchar *suf = pp + 2 + MAX_PAT;
    int ok = 1;
    for (int i = 0; i < pl && ok; i++)
      if (norm_c(addr[i], ci) != pre[i]) ok = 0;
    for (int i = 0; i < sl && ok; i++)
      if (norm_c(addr[len - sl + i], ci) != suf[i]) ok = 0;
    if (ok) return 1;
  }
  return 0;
}

/* ------------------------------ hunt list (hundreds or thousands of words at once)
   LIST is one buffer of uints, built by the host:
     [0, 128)          address character -> 6-bit code (the host folds case here when wanted)
     [128, 8320)       bitmap over the codes of the FIRST three characters of every start word
     [8320, 16512)     bitmap over the codes of the LAST three characters of every end word
     [16512, ...)      npre entries for start words, then nsuf entries for end words
   An entry is 4 uints: w0, w1, m0, m1. Up to ten characters, 6 bits each, five per uint,
   counted from the address end the word sits at; m0/m1 mask the characters the word has.
   Entries are sorted by their three-character code (w0 & 0x3FFFF).
   Almost every key fails the bitmap, so the cost is the same for 5 words or 5000. */
#define LIST_BITS 128u
#define LIST_ENTRIES 16512u

static int list_hit(const uchar addr[44], int len, int rev, uint code,
                    __global const uint * restrict LIST, uint base, uint n) {
  uint lo = 0, hi = n;
  while (lo < hi) {
    uint mid = (lo + hi) >> 1;
    if ((LIST[base + 4u * mid] & 0x3FFFFu) < code) lo = mid + 1; else hi = mid;
  }
  if (lo >= n || (LIST[base + 4u * lo] & 0x3FFFFu) != code) return 0;
  uint a0 = 0, a1 = 0;
  for (int i = 0; i < 5; i++) {
    a0 |= LIST[addr[rev ? len - 1 - i : i]] << (6 * i);
    a1 |= LIST[addr[rev ? len - 6 - i : 5 + i]] << (6 * i);
  }
  for (; lo < n; lo++) {
    __global const uint *e = LIST + base + 4u * lo;
    if ((e[0] & 0x3FFFFu) != code) break;
    if ((((a0 ^ e[0]) & e[2]) | ((a1 ^ e[1]) & e[3])) == 0u) return 1;
  }
  return 0;
}

static int list_any(const uchar addr[44], int len,
                    __global const uint * restrict LIST, uint npre, uint nsuf) {
  if (npre) {
    uint c = LIST[addr[0]] | (LIST[addr[1]] << 6) | (LIST[addr[2]] << 12);
    if (((LIST[LIST_BITS + (c >> 5)] >> (c & 31u)) & 1u)
        && list_hit(addr, len, 0, c, LIST, LIST_ENTRIES, npre)) return 1;
  }
  if (nsuf) {
    uint c = LIST[addr[len - 1]] | (LIST[addr[len - 2]] << 6) | (LIST[addr[len - 3]] << 12);
    if (((LIST[LIST_BITS + 8192u + (c >> 5)] >> (c & 31u)) & 1u)
        && list_hit(addr, len, 1, c, LIST, LIST_ENTRIES + 4u * npre, nsuf)) return 1;
  }
  return 0;
}

/* ------------------------------ entry points */

__kernel void grind(__global const uchar *base24,
                    ulong counter_base,
                    uint iters,
                    __global volatile uint *mcount,
                    __global uchar *mout,
                    __global const int * restrict COMB,
                    __global const uchar * restrict PATS,
                    uint npat, uint ci,
                    __global const uint * restrict LIST,
                    uint npre, uint nsuf) {
  size_t gid = get_global_id(0);
  uchar base[24];
  for (int i = 0; i < 24; i++) base[i] = base24[i];
  uint nsub = iters / K;
  for (uint sb = 0; sb < nsub; sb++) {
    uchar seeds[K][32], pubs[K][32];
    for (int j = 0; j < K; j++) {
      for (int i = 0; i < 24; i++) seeds[j][i] = base[i];
      ulong ctr = counter_base + (ulong)gid * iters + (ulong)sb * K + (uint)j;
      for (int i = 0; i < 8; i++) seeds[j][24 + i] = (uchar)(ctr >> (8 * i));
    }
    pubkeys_batch(seeds, pubs, COMB);
    for (int j = 0; j < K; j++) {
      uchar addr[44];
      int len = b58enc(pubs[j], addr);
      if (matches_any(addr, len, PATS, npat, ci) || list_any(addr, len, LIST, npre, nsuf)) {
        uint idx = atomic_inc(mcount);
        if (idx < MAX_MATCHES) {
          __global uchar *m = mout + idx * MATCH_STRIDE;
          for (int i = 0; i < 32; i++) m[i] = seeds[j][i];
          for (int i = 0; i < 32; i++) m[32 + i] = pubs[j][i];
          m[64] = (uchar)len;
          for (int i = 0; i < len; i++) m[65 + i] = addr[i];
        }
      }
    }
  }
}

/* per seed: sha512[64] | pub[32] | addrlen[1] | addr[44] = 141 bytes.
   Each work-item derives a full K-batch (exercises the batched-inversion
   walkback at all K positions). n must be a multiple of K. */
__kernel void selftest(__global const uchar *seeds,
                       __global uchar *out,
                       uint n,
                       __global const int * restrict COMB) {
  size_t gid = get_global_id(0);
  uint bpos = (uint)gid * K;
  uchar sd[K][32], pb[K][32];
  for (int j = 0; j < K; j++) {
    uint idx = bpos + j;
    for (int i = 0; i < 32; i++) sd[j][i] = (idx < n) ? seeds[idx * 32 + i] : 0;
  }
  pubkeys_batch(sd, pb, COMB);
  for (int j = 0; j < K; j++) {
    uint idx = bpos + j;
    if (idx >= n) continue;
    uchar dig[64], addr[44];
    sha512_32(sd[j], dig);
    int len = b58enc(pb[j], addr);
    __global uchar *o = out + idx * 141;
    for (int i = 0; i < 64; i++) o[i] = dig[i];
    for (int i = 0; i < 32; i++) o[64 + i] = pb[j][i];
    o[96] = (uchar)len;
    for (int i = 0; i < len; i++) o[97 + i] = addr[i];
  }
}
"""


PAT_STRIDE = 2 + 2 * MAX_PAT   # [pre_len][suf_len][pre MAX_PAT][suf MAX_PAT]


def build_source(max_matches=256):
    """The kernel source is pattern-independent: patterns are uploaded to a
    runtime buffer (see pack_patterns), so this compiles once and the cache is
    reused across every pattern/case-mode."""
    _check_fe_arithmetic()   # static proof of fe_mul/fe_sq bounds before build
    _check_inv_chain()       # static proof the inverse chain == z^(p-2)
    defines = f"""
#define MAX_MATCHES {max_matches}
#define MATCH_STRIDE {MATCH_STRIDE}
#define MAX_PAT {MAX_PAT}
#define PAT_STRIDE {PAT_STRIDE}
#define K {K}
"""
    return (gen_constants() + defines + gen_fe_mul() + gen_fe_sq()
            + gen_fe_invert() + "\n" + KERNEL_BODY)


def pack_patterns(patterns):
    """patterns: list of (prefix, suffix) strings (case-normalized by the host
    when ignore_case). Returns a flat uint8 array for the PATS buffer."""
    import numpy as np
    rows = []
    for pre, suf in (patterns or [("", "")]):
        row = bytearray(PAT_STRIDE)
        row[0] = len(pre)
        row[1] = len(suf)
        row[2:2 + len(pre)] = pre.encode()
        row[2 + MAX_PAT:2 + MAX_PAT + len(suf)] = suf.encode()
        rows.append(row)
    return np.frombuffer(b"".join(rows), dtype=np.uint8).copy()


if __name__ == "__main__":
    src = build_source()
    print(f"kernel source: {len(src)} bytes, {src.count(chr(10))} lines")
