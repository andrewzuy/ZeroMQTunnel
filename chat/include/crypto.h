#ifndef CRYPTO_H
#define CRYPTO_H

#include <stdint.h>
#include <openssl/rsa.h>
#include <openssl/pem.h>
#include "../include/utils.h"

/* Crypto context type - simplified without RSA pointer */
typedef struct crypto_key_ctx {
    void *priv_ptr;   /* opaque handle to private key */
    void *pub_ptr;    /* opaque handle to public key */
} crypto_key_t;

extern crypto_key_t *crypto_context;

crypto_key_t* generate_rsa_keypair(const char *priv_path, const char *pub_path);
void save_peek_public_key(void *, const char *);
int rsa_encrypt(const void *, const unsigned char *, size_t, unsigned char **, size_t *);
int rsa_decrypt(void *, const unsigned char *, size_t, unsigned char **, size_t *);
crypto_key_t* load_private_key(const char *path);
void* load_public_key_from_file(const char *path);

void crypto_cleanup(void *ctx);

#endif
