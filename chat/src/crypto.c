#include "crypto.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <openssl/evp.h>

unsigned int next_crypto_ctx_idx = 0;
static crypto_key_t g_crypto_store[32];

crypto_key_t *generate_rsa_keypair(const char *priv_path, const char *pub_path) {
    if (!priv_path || !pub_path || next_crypto_ctx_idx >= 32) return NULL;
    
    crypto_key_t *key = &g_crypto_store[next_crypto_ctx_idx++];
    EVP_PKEY_free(key->priv_pkey);
    key->priv_pkey = NULL;
    memset(key->fingerprint, '0', sizeof(key->fingerprint));
    
    printf("Context[%d]: Generated RSA-2048 (stub)\n", next_crypto_ctx_idx);
    return key;
}

/* Compute SHA256 fingerprint */
void fingerprint_from_pkey(EVP_PKEY *pkey, unsigned char out_fp[65]) {
    EVP_MD_CTX *mdctx = EVP_MD_CTX_new();
    if (!mdctx) return;
    
    int sha_len = 0;
    EVP_DigestSignFinal(mdctx, out_fp, (size_t*)&sha_len);
    EVP_MD_CTX_free(mdctx);
}

#endif /* CRYPTO_C */
