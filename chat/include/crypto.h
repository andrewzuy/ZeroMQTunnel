#ifndef CRYPTO_H
#define CRYPTO_H

#include <stdint.h>
#include <openssl/rsa.h>
#include "utils.h"

typedef struct crypto_key_ctx crypto_key_t;

/* Key generation and loading */
crypto_key_t* generate_rsa_keypair(const char *priv_path, const char *pub_path);
void save_peek_public_key(RSA *, const char *);
crypto_key_t* load_private_key(const char *path);
RSA* load_public_key_from_file(const char *path);

/* Fingerprint and nonce */
void fingerprint_from_rsa(RSA *, unsigned char [65]);
void generate_nonce(unsigned char *, int);

/* RSA operations (hybrid crypto) */
int rsa_encrypt(const RSA *, const unsigned char *, size_t, unsigned char **, size_t *);
int rsa_decrypt(RSA *, const unsigned char *, size_t, unsigned char **, size_t *);
unsigned char *rsa_sign(const RSA *, const unsigned char *, size_t, size_t *);
int rsa_verify(const RSA *, const unsigned char *, size_t, const unsigned char *, size_t);

/* AES session key crypto */
unsigned char *aes_encrypt_key(const EVP_CIPHER *, const EVP_CIPHER *, const unsigned char *, const unsigned char *, size_t **out_len);
int aes_decrypt_key(const EVP_CIPHER *, const EVP_CIPHER *, const unsigned char *, const unsigned char *, const unsigned char *, size_t, unsigned char **, size_t *);

/* Cleanup */
void crypto_key_cleanup(void *ctx);
void crypto_cleanup_all(void);

#endif /* CRYPTO_H */
