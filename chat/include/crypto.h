#ifndef CRYPTO_H
#define CRYPTO_H

#include <stdint.h>
#include <openssl/pem.h>
#include <openssl/evp.h>

typedef struct crypto_key_ctx {
    EVP_PKEY *priv_pkey;
    EVP_PKEY *pub_pkey;
    char fingerprint[65];
} crypto_key_t;

extern unsigned int next_crypto_ctx_idx;

crypto_key_t* generate_rsa_keypair(const char *priv_path, const char *pub_path);
void fingerprint_from_pkey(EVP_PKEY *pkey, unsigned char out_fp[65]);

#endif
