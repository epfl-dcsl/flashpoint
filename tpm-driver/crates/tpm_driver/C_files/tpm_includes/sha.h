#ifndef __SHA_H
#define __SHA_H

#include "types.h" // u32

void sha1(const unsigned char *data, unsigned int length, unsigned char *hash);
void sha256(const unsigned char *data, unsigned int length, unsigned char *hash);
void sha384(const unsigned char *data, unsigned int length, unsigned char *hash);
void sha512(const unsigned char *data, unsigned int length, unsigned char *hash);

#endif // sha.h