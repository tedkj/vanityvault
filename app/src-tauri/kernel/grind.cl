
__constant ulong SHA_K[80] = {
  0x428A2F98D728AE22UL, 0x7137449123EF65CDUL, 0xB5C0FBCFEC4D3B2FUL, 0xE9B5DBA58189DBBCUL,
  0x3956C25BF348B538UL, 0x59F111F1B605D019UL, 0x923F82A4AF194F9BUL, 0xAB1C5ED5DA6D8118UL,
  0xD807AA98A3030242UL, 0x12835B0145706FBEUL, 0x243185BE4EE4B28CUL, 0x550C7DC3D5FFB4E2UL,
  0x72BE5D74F27B896FUL, 0x80DEB1FE3B1696B1UL, 0x9BDC06A725C71235UL, 0xC19BF174CF692694UL,
  0xE49B69C19EF14AD2UL, 0xEFBE4786384F25E3UL, 0x0FC19DC68B8CD5B5UL, 0x240CA1CC77AC9C65UL,
  0x2DE92C6F592B0275UL, 0x4A7484AA6EA6E483UL, 0x5CB0A9DCBD41FBD4UL, 0x76F988DA831153B5UL,
  0x983E5152EE66DFABUL, 0xA831C66D2DB43210UL, 0xB00327C898FB213FUL, 0xBF597FC7BEEF0EE4UL,
  0xC6E00BF33DA88FC2UL, 0xD5A79147930AA725UL, 0x06CA6351E003826FUL, 0x142929670A0E6E70UL,
  0x27B70A8546D22FFCUL, 0x2E1B21385C26C926UL, 0x4D2C6DFC5AC42AEDUL, 0x53380D139D95B3DFUL,
  0x650A73548BAF63DEUL, 0x766A0ABB3C77B2A8UL, 0x81C2C92E47EDAEE6UL, 0x92722C851482353BUL,
  0xA2BFE8A14CF10364UL, 0xA81A664BBC423001UL, 0xC24B8B70D0F89791UL, 0xC76C51A30654BE30UL,
  0xD192E819D6EF5218UL, 0xD69906245565A910UL, 0xF40E35855771202AUL, 0x106AA07032BBD1B8UL,
  0x19A4C116B8D2D0C8UL, 0x1E376C085141AB53UL, 0x2748774CDF8EEB99UL, 0x34B0BCB5E19B48A8UL,
  0x391C0CB3C5C95A63UL, 0x4ED8AA4AE3418ACBUL, 0x5B9CCA4F7763E373UL, 0x682E6FF3D6B2B8A3UL,
  0x748F82EE5DEFB2FCUL, 0x78A5636F43172F60UL, 0x84C87814A1F0AB72UL, 0x8CC702081A6439ECUL,
  0x90BEFFFA23631E28UL, 0xA4506CEBDE82BDE9UL, 0xBEF9A3F7B2C67915UL, 0xC67178F2E372532BUL,
  0xCA273ECEEA26619CUL, 0xD186B8C721C0C207UL, 0xEADA7DD6CDE0EB1EUL, 0xF57D4F7FEE6ED178UL,
  0x06F067AA72176FBAUL, 0x0A637DC5A2C898A6UL, 0x113F9804BEF90DAEUL, 0x1B710B35131C471BUL,
  0x28DB77F523047D84UL, 0x32CAAB7B40C72493UL, 0x3C9EBE0A15C9BEBCUL, 0x431D67C49C100D4CUL,
  0x4CC5D4BECB3E42B6UL, 0x597F299CFC657E2AUL, 0x5FCB6FAB3AD6FAECUL, 0x6C44198C4A475817UL
};
__constant ulong SHA_H0[8] = { 0x6A09E667F3BCC908UL, 0xBB67AE8584CAA73BUL, 0x3C6EF372FE94F82BUL, 0xA54FF53A5F1D36F1UL, 0x510E527FADE682D1UL, 0x9B05688C2B3E6C1FUL, 0x1F83D9ABFB41BD6BUL, 0x5BE0CD19137E2179UL };

__constant int TWO_P[10] = { 134217690, 67108862, 134217726, 67108862, 134217726, 67108862, 134217726, 67108862, 134217726, 67108862 };
__constant uint P_WORDS[8] = { 0xFFFFFFEDU, 0xFFFFFFFFU, 0xFFFFFFFFU, 0xFFFFFFFFU, 0xFFFFFFFFU, 0xFFFFFFFFU, 0xFFFFFFFFU, 0x7FFFFFFFU };
__constant uchar B58_ALPHA[58] = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

#define MAX_MATCHES 256
#define MATCH_STRIDE 112
#define MAX_PAT 16
#define PAT_STRIDE 34
#define K 32
static void fe_mul(int h[10], const int f[10], const int g[10]) {
  long t[10];
  int f1_2 = 2*f[1];
  int f1_38 = 38*f[1];
  int f2_19 = 19*f[2];
  int f3_2 = 2*f[3];
  int f3_19 = 19*f[3];
  int f3_38 = 38*f[3];
  int f4_19 = 19*f[4];
  int f5_2 = 2*f[5];
  int f5_19 = 19*f[5];
  int f5_38 = 38*f[5];
  int f6_19 = 19*f[6];
  int f7_2 = 2*f[7];
  int f7_19 = 19*f[7];
  int f7_38 = 38*f[7];
  int f8_19 = 19*f[8];
  int f9_19 = 19*f[9];
  int f9_38 = 38*f[9];
  t[0] = (long)f[0]*g[0] + (long)f1_38*g[9] + (long)f2_19*g[8] + (long)f3_38*g[7] + (long)f4_19*g[6] + (long)f5_38*g[5] + (long)f6_19*g[4] + (long)f7_38*g[3] + (long)f8_19*g[2] + (long)f9_38*g[1];
  t[1] = (long)f[0]*g[1] + (long)f[1]*g[0] + (long)f2_19*g[9] + (long)f3_19*g[8] + (long)f4_19*g[7] + (long)f5_19*g[6] + (long)f6_19*g[5] + (long)f7_19*g[4] + (long)f8_19*g[3] + (long)f9_19*g[2];
  t[2] = (long)f[0]*g[2] + (long)f1_2*g[1] + (long)f[2]*g[0] + (long)f3_38*g[9] + (long)f4_19*g[8] + (long)f5_38*g[7] + (long)f6_19*g[6] + (long)f7_38*g[5] + (long)f8_19*g[4] + (long)f9_38*g[3];
  t[3] = (long)f[0]*g[3] + (long)f[1]*g[2] + (long)f[2]*g[1] + (long)f[3]*g[0] + (long)f4_19*g[9] + (long)f5_19*g[8] + (long)f6_19*g[7] + (long)f7_19*g[6] + (long)f8_19*g[5] + (long)f9_19*g[4];
  t[4] = (long)f[0]*g[4] + (long)f1_2*g[3] + (long)f[2]*g[2] + (long)f3_2*g[1] + (long)f[4]*g[0] + (long)f5_38*g[9] + (long)f6_19*g[8] + (long)f7_38*g[7] + (long)f8_19*g[6] + (long)f9_38*g[5];
  t[5] = (long)f[0]*g[5] + (long)f[1]*g[4] + (long)f[2]*g[3] + (long)f[3]*g[2] + (long)f[4]*g[1] + (long)f[5]*g[0] + (long)f6_19*g[9] + (long)f7_19*g[8] + (long)f8_19*g[7] + (long)f9_19*g[6];
  t[6] = (long)f[0]*g[6] + (long)f1_2*g[5] + (long)f[2]*g[4] + (long)f3_2*g[3] + (long)f[4]*g[2] + (long)f5_2*g[1] + (long)f[6]*g[0] + (long)f7_38*g[9] + (long)f8_19*g[8] + (long)f9_38*g[7];
  t[7] = (long)f[0]*g[7] + (long)f[1]*g[6] + (long)f[2]*g[5] + (long)f[3]*g[4] + (long)f[4]*g[3] + (long)f[5]*g[2] + (long)f[6]*g[1] + (long)f[7]*g[0] + (long)f8_19*g[9] + (long)f9_19*g[8];
  t[8] = (long)f[0]*g[8] + (long)f1_2*g[7] + (long)f[2]*g[6] + (long)f3_2*g[5] + (long)f[4]*g[4] + (long)f5_2*g[3] + (long)f[6]*g[2] + (long)f7_2*g[1] + (long)f[8]*g[0] + (long)f9_38*g[9];
  t[9] = (long)f[0]*g[9] + (long)f[1]*g[8] + (long)f[2]*g[7] + (long)f[3]*g[6] + (long)f[4]*g[5] + (long)f[5]*g[4] + (long)f[6]*g[3] + (long)f[7]*g[2] + (long)f[8]*g[1] + (long)f[9]*g[0];
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
}
static void fe_sq(int h[10], const int f[10]) {
  long t[10];
  int f0_2 = 2*f[0];
  int f1_2 = 2*f[1];
  int f1_4 = 4*f[1];
  int f1_38 = 38*f[1];
  int f2_2 = 2*f[2];
  int f2_19 = 19*f[2];
  int f3_2 = 2*f[3];
  int f3_4 = 4*f[3];
  int f3_38 = 38*f[3];
  int f4_2 = 2*f[4];
  int f4_19 = 19*f[4];
  int f5_38 = 38*f[5];
  int f6_2 = 2*f[6];
  int f6_19 = 19*f[6];
  int f7_2 = 2*f[7];
  int f7_38 = 38*f[7];
  int f8_2 = 2*f[8];
  int f8_19 = 19*f[8];
  int f9_2 = 2*f[9];
  int f9_38 = 38*f[9];
  t[0] = (long)f[0]*f[0] + (long)f1_38*f9_2 + (long)f2_19*f8_2 + (long)f3_38*f7_2 + (long)f4_19*f6_2 + (long)f5_38*f[5];
  t[1] = (long)f0_2*f[1] + (long)f2_19*f9_2 + (long)f3_38*f[8] + (long)f4_19*f7_2 + (long)f5_38*f[6];
  t[2] = (long)f0_2*f[2] + (long)f1_2*f[1] + (long)f3_38*f9_2 + (long)f4_19*f8_2 + (long)f5_38*f7_2 + (long)f6_19*f[6];
  t[3] = (long)f0_2*f[3] + (long)f1_2*f[2] + (long)f4_19*f9_2 + (long)f5_38*f[8] + (long)f6_19*f7_2;
  t[4] = (long)f0_2*f[4] + (long)f1_4*f[3] + (long)f[2]*f[2] + (long)f5_38*f9_2 + (long)f6_19*f8_2 + (long)f7_38*f[7];
  t[5] = (long)f0_2*f[5] + (long)f1_2*f[4] + (long)f2_2*f[3] + (long)f6_19*f9_2 + (long)f7_38*f[8];
  t[6] = (long)f0_2*f[6] + (long)f1_4*f[5] + (long)f2_2*f[4] + (long)f3_2*f[3] + (long)f7_38*f9_2 + (long)f8_19*f[8];
  t[7] = (long)f0_2*f[7] + (long)f1_2*f[6] + (long)f2_2*f[5] + (long)f3_2*f[4] + (long)f8_19*f9_2;
  t[8] = (long)f0_2*f[8] + (long)f1_4*f[7] + (long)f2_2*f[6] + (long)f3_4*f[5] + (long)f[4]*f[4] + (long)f9_38*f[9];
  t[9] = (long)f0_2*f[9] + (long)f1_2*f[8] + (long)f2_2*f[7] + (long)f3_2*f[6] + (long)f4_2*f[5];
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
}
static void fe_invert(int out[10], const int z[10]) {
  int t0[10], t1[10], t2[10], t3[10];
  fe_sq(t0, z);
  fe_sq(t1, t0);
  fe_sq(t1, t1);
  fe_mul(t1, z, t1);
  fe_mul(t0, t0, t1);
  fe_sq(t2, t0);
  fe_mul(t1, t1, t2);
  fe_sq(t2, t1);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_mul(t1, t2, t1);
  fe_sq(t2, t1);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_mul(t2, t2, t1);
  fe_sq(t3, t2);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_mul(t2, t3, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_mul(t1, t2, t1);
  fe_sq(t2, t1);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_mul(t2, t2, t1);
  fe_sq(t3, t2);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_sq(t3, t3);
  fe_mul(t2, t3, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_sq(t2, t2);
  fe_mul(t1, t2, t1);
  fe_sq(t1, t1);
  fe_sq(t1, t1);
  fe_sq(t1, t1);
  fe_sq(t1, t1);
  fe_sq(t1, t1);
  fe_mul(out, t1, t0);
}

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
