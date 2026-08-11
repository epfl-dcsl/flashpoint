/*****************************************************************************
 * Copyright (c) 2021 IBM Corporation
 * All rights reserved.
 * This program and the accompanying materials
 * are made available under the terms of the BSD License
 * which accompanies this distribution, and is available at
 * http://www.opensource.org/licenses/bsd-license.php
 *
 * Contributors:
 *     IBM Corporation - initial implementation
 *****************************************************************************/

/*
 *  See: NIST standard for SHA-512 and SHA-384 in FIPS PUB 180-4 & RFC 6234
 */

 //#include "config.h"
 #include "../tpm_includes/swap.h"
 #include "../tpm_includes/sha.h"
 // #include "string.h"

 void *my_memset(void *blk, int c, u64 n)
{
    u64  i;

    for (i = 0; i < n; ++i)
        ((unsigned char *) blk)[i] = c;

    return blk;
}

void *my_memcpy(void *dest, const void *src, u64 n)
{
    u64  i;

    for ( u64 i = 0; i < n; ++i){
        ((unsigned char *) dest)[i] = ((unsigned char *) src)[i];
    }
    return dest;
}

static inline u32 ror(u32 val, u16 ror) {
    // Rotation is equivalent to combining a right shift and a left shift.
    // The shift amount 'ror' is masked by 31 (for 32-bit rotation) 
    // to handle shifts greater than 31 bits, which is safe in C.
    // Note: C compilers often optimize this pattern into a single instruction 
    // if the target platform supports it (like the 'ror' instruction).
    return (val >> ror) | (val << (32 - ror));
}

static inline u32 rol(u32 val, u16 rol) {
    // Rotation is equivalent to combining a left shift and a right shift.
    // The amount of rotation for the wrap-around is (32 - rol).
    // The shift amount 'rol' is masked by 31 (for 32-bit rotation), 
    // which is safely handled by the C language standard.
    return (val << rol) | (val >> (32 - rol));
}
 
 typedef struct _sha512_ctx {
     unsigned long long h[8];
 } sha512_ctx;
 
 static inline unsigned long long ror64(unsigned long long x, unsigned char n)
 {
     return (x >> n) | (x << (64 - n));
 }
 
 static inline unsigned long long Ch64(unsigned long long x, unsigned long long y, unsigned long long z)
 {
     return (x & y) ^ ((x ^ 0xffffffffffffffffULL) & z);
 }
 
 static inline unsigned long long Maj64(unsigned long long x, unsigned long long y, unsigned long long z)
 {
     return (x & y) ^ (x & z) ^ (y & z);
 }
 
 static inline unsigned long long sum0_64(unsigned long long x)
 {
     return ror64(x, 28) ^ ror64(x, 34) ^ ror64(x, 39);
 }
 
 static inline unsigned long long sum1_64(unsigned long long x)
 {
     return ror64(x, 14) ^ ror64(x, 18) ^ ror64(x, 41);
 }
 
 static inline unsigned long long sigma0_64(unsigned long long x)
 {
     return ror64(x, 1) ^ ror64(x, 8) ^ (x >> 7);
 }
 
 static inline unsigned long long sigma1_64(unsigned long long x)
 {
     return ror64(x, 19) ^ ror64(x, 61) ^ (x >> 6);
 }
 
 static void sha512_block(unsigned long long *w, sha512_ctx *ctx)
 {
     unsigned int t;
     unsigned long long a, b, c, d, e, f, g, h;
     unsigned long long T1, T2;
 
     /*
      * FIPS 180-4 4.2.2: SHA512 Constants
      */
     static const unsigned long long sha_ko[80] = {
         0x428a2f98d728ae22, 0x7137449123ef65cd, 0xb5c0fbcfec4d3b2f, 0xe9b5dba58189dbbc,
         0x3956c25bf348b538, 0x59f111f1b605d019, 0x923f82a4af194f9b, 0xab1c5ed5da6d8118,
         0xd807aa98a3030242, 0x12835b0145706fbe, 0x243185be4ee4b28c, 0x550c7dc3d5ffb4e2,
         0x72be5d74f27b896f, 0x80deb1fe3b1696b1, 0x9bdc06a725c71235, 0xc19bf174cf692694,
         0xe49b69c19ef14ad2, 0xefbe4786384f25e3, 0x0fc19dc68b8cd5b5, 0x240ca1cc77ac9c65,
         0x2de92c6f592b0275, 0x4a7484aa6ea6e483, 0x5cb0a9dcbd41fbd4, 0x76f988da831153b5,
         0x983e5152ee66dfab, 0xa831c66d2db43210, 0xb00327c898fb213f, 0xbf597fc7beef0ee4,
         0xc6e00bf33da88fc2, 0xd5a79147930aa725, 0x06ca6351e003826f, 0x142929670a0e6e70,
         0x27b70a8546d22ffc, 0x2e1b21385c26c926, 0x4d2c6dfc5ac42aed, 0x53380d139d95b3df,
         0x650a73548baf63de, 0x766a0abb3c77b2a8, 0x81c2c92e47edaee6, 0x92722c851482353b,
         0xa2bfe8a14cf10364, 0xa81a664bbc423001, 0xc24b8b70d0f89791, 0xc76c51a30654be30,
         0xd192e819d6ef5218, 0xd69906245565a910, 0xf40e35855771202a, 0x106aa07032bbd1b8,
         0x19a4c116b8d2d0c8, 0x1e376c085141ab53, 0x2748774cdf8eeb99, 0x34b0bcb5e19b48a8,
         0x391c0cb3c5c95a63, 0x4ed8aa4ae3418acb, 0x5b9cca4f7763e373, 0x682e6ff3d6b2b8a3,
         0x748f82ee5defb2fc, 0x78a5636f43172f60, 0x84c87814a1f0ab72, 0x8cc702081a6439ec,
         0x90befffa23631e28, 0xa4506cebde82bde9, 0xbef9a3f7b2c67915, 0xc67178f2e372532b,
         0xca273eceea26619c, 0xd186b8c721c0c207, 0xeada7dd6cde0eb1e, 0xf57d4f7fee6ed178,
         0x06f067aa72176fba, 0x0a637dc5a2c898a6, 0x113f9804bef90dae, 0x1b710b35131c471b,
         0x28db77f523047d84, 0x32caab7b40c72493, 0x3c9ebe0a15c9bebc, 0x431d67c49c100d4c,
         0x4cc5d4becb3e42b6, 0x597f299cfc657e2a, 0x5fcb6fab3ad6faec, 0x6c44198c4a475817
     };
 
     /*
      * FIPS 180-4 6.4.2: step 1
      *
      *  0 <= i <= 15:
      *    W(t) = M(t)
      * 16 <= i <= 79:
      *    W(t) = sigma1(W(t-2)) + W(t-7) + sigma0(W(t-15)) + W(t-16)
      */
 
     /* w(0)..w(15) are in big endian format */
     for (t = 0; t <= 15; t++)
         w[t] = be64_to_cpu(w[t]);
 
     for (t = 16; t <= 79; t++)
         w[t] = sigma1_64(w[t-2]) + w[t-7] + sigma0_64(w[t-15]) + w[t-16];
 
     /*
      * step 2: a = H0, b = H1, c = H2, d = H3, e = H4, f = H5, g = H6, h = H7
      */
     a = ctx->h[0];
     b = ctx->h[1];
     c = ctx->h[2];
     d = ctx->h[3];
     e = ctx->h[4];
     f = ctx->h[5];
     g = ctx->h[6];
     h = ctx->h[7];
 
     /*
      * step 3: For i = 0 to 79:
      *    T1 = h + sum1(e) + Ch(e,f,g) + K(t) + W(t);
      *    T2 = sum0(a) + Maj(a,b,c)
      *    h = g; g = f; f = e; e = d + T1; d = c; c = b; b = a; a + T1 + T2
      */
     for (t = 0; t <= 79; t++) {
         T1 = h + sum1_64(e) + Ch64(e, f, g) + sha_ko[t] + w[t];
         T2 = sum0_64(a) + Maj64(a, b, c);
         h = g;
         g = f;
         f = e;
         e = d + T1;
         d = c;
         c = b;
         b = a;
         a = T1 + T2;
     }
 
     /*
      * step 4:
      *    H0 = a + H0, H1 = b + H1, H2 = c + H2, H3 = d + H3, H4 = e + H4
      */
     ctx->h[0] += a;
     ctx->h[1] += b;
     ctx->h[2] += c;
     ctx->h[3] += d;
     ctx->h[4] += e;
     ctx->h[5] += f;
     ctx->h[6] += g;
     ctx->h[7] += h;
 }
 
 static void sha512_do(sha512_ctx *ctx, const unsigned char *data32, unsigned int length)
 {
     unsigned int offset;
     unsigned short num;
     unsigned long long bits = 0;
     unsigned long long w[80];
     unsigned long long tmp;
 
     /* treat data in 128-byte/1024 bit chunks */
     for (offset = 0; length - offset >= 128; offset += 128) {
         my_memcpy(w, data32 + offset, 128);
         sha512_block(w, ctx);
         bits += (128 * 8);
     }
 
     /* last block with less than 128 bytes */
     num = length - offset;
     bits += (num << 3);
 
     my_memcpy(w, data32 + offset, num);
     /*
      * FIPS 180-4 5.1: Padding the Message
      */
     ((unsigned char *)w)[num] = 0x80;
     if (128 - (num + 1) > 0)
         my_memset( &((unsigned char *)w)[num + 1], 0, 128 - (num + 1));
 
     if (num >= 112) {
         /* cannot append number of bits here;
          * need space for 128 bits (16 bytes)
          */
         sha512_block((unsigned long long *)w, ctx);
         my_memset(w, 0, 128);
     }
 
     /* write number of bits to end of the block; we write 64 bits */
     tmp = cpu_to_be64(bits);
     my_memcpy(&w[15], &tmp, 8);
 
     sha512_block(w, ctx);
 
     /* need to switch result's endianness */
     for (num = 0; num < 8; num++)
         ctx->h[num] = cpu_to_be64(ctx->h[num]);
 }
 
 void sha384(const unsigned char *data, unsigned int length, unsigned char *hash)
 {
     sha512_ctx ctx = {
         .h = {
             /*
              * FIPS 180-4: 6.2.1
              *   -> 5.3.4: initial hash value
              */
             0xcbbb9d5dc1059ed8,
             0x629a292a367cd507,
             0x9159015a3070dd17,
             0x152fecd8f70e5939,
             0x67332667ffc00b31,
             0x8eb44a8768581511,
             0xdb0c2e0d64f98fa7,
             0x47b5481dbefa4fa4
         }
     };
 
     sha512_do(&ctx, data, length);
     my_memcpy(hash, ctx.h, 384/8);
 }
 
 void sha512(const unsigned char *data, unsigned int length, unsigned char *hash)
 {
     sha512_ctx ctx = {
         .h = {
             /*
              * FIPS 180-4: 6.2.1
              *   -> 5.3.5: initial hash value
              */
             0x6a09e667f3bcc908,
             0xbb67ae8584caa73b,
             0x3c6ef372fe94f82b,
             0xa54ff53a5f1d36f1,
             0x510e527fade682d1,
             0x9b05688c2b3e6c1f,
             0x1f83d9abfb41bd6b,
             0x5be0cd19137e2179
         }
     };
 
     sha512_do(&ctx, data, length);
     my_memcpy(hash, ctx.h, sizeof(ctx.h));
 }

 //  Support for Calculation of SHA1 in SW
//
//  Copyright (C) 2006-2011 IBM Corporation
//
//  Authors:
//      Stefan Berger <stefanb@linux.vnet.ibm.com>
//
// This file may be distributed under the terms of the GNU LGPLv3 license.
//
//  See: http://www.itl.nist.gov/fipspubs/fip180-1.htm
//       RFC3174, Wikipedia's SHA1 alogrithm description
//

// #include "config.h"
// #include "byteorder.h" // cpu_to_*, __swab64
// #include "sha.h" // sha1
// #include "string.h" // memcpy
// #include "x86.h" // rol

typedef struct _sha1_ctx {
    u32 h[5];
} sha1_ctx;


static void
sha1_block(u32 *w, sha1_ctx *ctx)
{
    u32 i;
    u32 a,b,c,d,e,f;
    u32 tmp;
    u32 idx;

    static const u32 sha_ko[4] = {
        0x5a827999, 0x6ed9eba1, 0x8f1bbcdc, 0xca62c1d6 };

    /* change endianness of given data */
    for (i = 0; i < 16; i++)
        w[i] = be32_to_cpu(w[i]);

    for (i = 16; i <= 79; i++) {
        tmp = w[i-3] ^ w[i-8] ^ w[i-14] ^ w[i-16];
        w[i] = rol(tmp,1);
    }

    a = ctx->h[0];
    b = ctx->h[1];
    c = ctx->h[2];
    d = ctx->h[3];
    e = ctx->h[4];

    for (i = 0; i <= 79; i++) {
        if (i <= 19) {
            f = (b & c) | ((b ^ 0xffffffff) & d);
            idx = 0;
        } else if (i <= 39) {
            f = b ^ c ^ d;
            idx = 1;
        } else if (i <= 59) {
            f = (b & c) | (b & d) | (c & d);
            idx = 2;
        } else {
            f = b ^ c ^ d;
            idx = 3;
        }

        tmp = rol(a, 5) +
              f +
              e +
              sha_ko[idx] +
              w[i];
        e = d;
        d = c;
        c = rol(b, 30);
        b = a;
        a = tmp;
    }

    ctx->h[0] += a;
    ctx->h[1] += b;
    ctx->h[2] += c;
    ctx->h[3] += d;
    ctx->h[4] += e;
}


static void
sha1_do(sha1_ctx *ctx, const u8 *data32, u32 length)
{
    u32 offset;
    u16 num;
    u32 bits = 0;
    u32 w[80];
    u64 tmp;

    /* treat data in 64-byte chunks */
    for (offset = 0; length - offset >= 64; offset += 64) {
        my_memcpy(w, data32 + offset, 64);
        sha1_block((u32 *)w, ctx);
        bits += (64 * 8);
    }

    /* last block with less than 64 bytes */
    num = length - offset;
    bits += (num << 3);

    my_memcpy(w, data32 + offset, num);
    ((u8 *)w)[num] = 0x80;
    if (64 - (num + 1) > 0)
        my_memset( &((u8 *)w)[num + 1], 0x0, 64 - (num + 1));

    if (num >= 56) {
        /* cannot append number of bits here */
        sha1_block((u32 *)w, ctx);
        my_memset(w, 0x0, 60);
    }

    /* write number of bits to end of block */
    tmp = __swab64(bits);
    my_memcpy(&w[14], &tmp, 8);

    sha1_block(w, ctx);

    /* need to switch result's endianness */
    for (num = 0; num < 5; num++)
        ctx->h[num] = cpu_to_be32(ctx->h[num]);
}


void
sha1(const u8 *data, u32 length, u8 *hash)
{
    sha1_ctx ctx = {
        .h[0] = 0x67452301,
        .h[1] = 0xefcdab89,
        .h[2] = 0x98badcfe,
        .h[3] = 0x10325476,
        .h[4] = 0xc3d2e1f0,
    };

    sha1_do(&ctx, data, length);
    my_memcpy(hash, &ctx.h[0], 20);

    return;
}

typedef struct _sha256_ctx {
    u32 h[8];
} sha256_ctx;

static inline u32 Ch(u32 x, u32 y, u32 z)
{
    return (x & y) | ((x ^ 0xffffffff) & z);
}

static inline u32 Maj(u32 x, u32 y, u32 z)
{
    return (x & y) | (x & z) | (y & z);
}

static inline u32 sum0(u32 x)
{
    return ror(x, 2) ^ ror(x, 13) ^ ror(x, 22);
}

static inline u32 sum1(u32 x)
{
    return ror(x, 6) ^ ror(x, 11) ^ ror(x, 25);
}

static inline u32 sigma0(u32 x)
{
    return ror(x, 7) ^ ror(x, 18) ^ (x >> 3);
}

static inline u32 sigma1(u32 x)
{
    return ror(x, 17) ^ ror(x, 19) ^ (x >> 10);
}

static void sha256_block(u32 *w, sha256_ctx *ctx)
{
    u32 t;
    u32 a, b, c, d, e, f, g, h;
    u32 T1, T2;

    /*
     * FIPS 180-4 4.2.2: SHA256 Constants
     */
    static const u32 sha_ko[64] = {
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5,
        0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
        0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
        0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
        0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc,
        0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
        0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
        0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
        0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3,
        0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5,
        0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
        0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2
    };

    /*
     * FIPS 180-4 6.2.2: step 1
     *
     *  0 <= i <= 15:
     *    W(t) = M(t)
     * 16 <= i <= 63:
     *    W(t) = sigma1(W(t-2)) + W(t-7) + sigma0(W(t-15)) + W(t-16)
     */

    /* w(0)..w(15) are in big endian format */
    for (t = 0; t <= 15; t++)
        w[t] = be32_to_cpu(w[t]);

    for (t = 16; t <= 63; t++)
        w[t] = sigma1(w[t-2]) + w[t-7] + sigma0(w[t-15]) + w[t-16];

    /*
     * step 2: a = H0, b = H1, c = H2, d = H3, e = H4, f = H5, g = H6, h = H7
     */
    a = ctx->h[0];
    b = ctx->h[1];
    c = ctx->h[2];
    d = ctx->h[3];
    e = ctx->h[4];
    f = ctx->h[5];
    g = ctx->h[6];
    h = ctx->h[7];

    /*
     * step 3: For i = 0 to 63:
     *    T1 = h + sum1(e) + Ch(e,f,g) + K(t) + W(t);
     *    T2 = sum0(a) + Maj(a,b,c)
     *    h = g; g = f; f = e; e = d + T1; d = c; c = b; b = a; a + T1 + T2
     */
    for (t = 0; t <= 63; t++) {
        T1 = h + sum1(e) + Ch(e, f, g) + sha_ko[t] + w[t];
        T2 = sum0(a) + Maj(a, b, c);
        h = g;
        g = f;
        f = e;
        e = d + T1;
        d = c;
        c = b;
        b = a;
        a = T1 + T2;
    }

    /*
     * step 4:
     *    H0 = a + H0, H1 = b + H1, H2 = c + H2, H3 = d + H3, H4 = e + H4
     */
    ctx->h[0] += a;
    ctx->h[1] += b;
    ctx->h[2] += c;
    ctx->h[3] += d;
    ctx->h[4] += e;
    ctx->h[5] += f;
    ctx->h[6] += g;
    ctx->h[7] += h;
}

static void sha256_do(sha256_ctx *ctx, const u8 *data32, u32 length)
{
    u32 offset;
    u16 num;
    u32 bits = 0;
    u32 w[64];
    u64 tmp;

    /* treat data in 64-byte chunks */
    for (offset = 0; length - offset >= 64; offset += 64) {
        my_memcpy(w, data32 + offset, 64);
        sha256_block((u32 *)w, ctx);
        bits += (64 * 8);
    }

    /* last block with less than 64 bytes */
    num = length - offset;
    bits += (num << 3);

    my_memcpy(w, data32 + offset, num);
    /*
     * FIPS 180-4 5.1: Padding the Message
     */
    ((u8 *)w)[num] = 0x80;
    if (64 - (num + 1) > 0)
        my_memset( &((u8 *)w)[num + 1], 0, 64 - (num + 1));

    if (num >= 56) {
        /* cannot append number of bits here */
        sha256_block((u32 *)w, ctx);
        my_memset(w, 0, 60);
    }

    /* write number of bits to end of block */
    tmp = cpu_to_be64(bits);
    my_memcpy(&w[14], &tmp, 8);

    sha256_block(w, ctx);

    /* need to switch result's endianness */
    for (num = 0; num < 8; num++)
        ctx->h[num] = cpu_to_be32(ctx->h[num]);
}

void sha256(const u8 *data, u32 length, u8 *hash)
{
    sha256_ctx ctx = {
        .h = {
            /*
             * FIPS 180-4: 6.2.1
             *   -> 5.3.3: initial hash value
             */
            0x6a09e667,
            0xbb67ae85,
            0x3c6ef372,
            0xa54ff53a,
            0x510e527f,
            0x9b05688c,
            0x1f83d9ab,
            0x5be0cd19
        }
    };

    sha256_do(&ctx, data, length);
    my_memcpy(hash, ctx.h, sizeof(ctx.h));
}