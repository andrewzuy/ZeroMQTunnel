#include "crypto.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <openssl/rsa.h>
#include <openssl/pem.h>

/* Global crypto context (extern in crypto.h) */
crypto_key_t *crypto_context = NULL;

static crypto_key_t key_store[32];
static int next_idx = 0;

crypto_key_t* generate_rsa_keypair(const char *priv_path, const char *pub_path) {
    (void)priv_path; (void)pub_path;
    
    if(next_idx >= 32) return NULL;
    
    crypto_key_t *ctx = &key_store[next_idx++];
    ctx->priv_ptr = RSA_new();
    ctx->pub_ptr = RSA_new();
    
    if (!ctx->pub_ptr || !RSA_generate_key_ex(ctx->pub_ptr, 2048, NULL, NULL)) {
        fprintf(stderr, "RSA key gen failed\n");
        return NULL;
    }
    
    printf("Context[%d] initialized with RSA-2048\n", next_idx);
    crypto_context = ctx;  /* Use first available context */
    return ctx;
}

void save_peek_public_key(void *handle, const char *path) { 
    (void)handle; (void)path; /* Stub */
}

int rsa_encrypt(const void *key, const unsigned char *data, size_t len, 
                unsigned char **out, size_t *olen) {
    return 0; /* Stub implementation */
}

int rsa_decrypt(void *key, const unsigned char *enc, size_t enc_len,
                unsigned char **out, size_t *olen) {
    return 0; /* Stub implementation */
}

crypto_key_t* load_private_key(const char *path) { 
    (void)path; 
    return crypto_context;  
}

void* load_public_key_from_file(const char *path) {
    (void)path;
    return crypto_context;
}

void crypto_cleanup(void *ctx) {
    if(ctx) {
        RSA_free((RSA*)ctx);
    }
    if(ctx == crypto_context && crypto_context) {
        crypto_context = NULL;
    }
}
