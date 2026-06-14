/* test_crypto.c - Unit tests for cryptographic functions */

#include "crypto.h"
#include "../include/logging.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int generated_rsa_key = 0;

void test_generate_rsa_keypair(void) {
#if defined(_WIN32) && USE_CMAKE
    if (system("make > /dev/null")) {
        system("rm -f keygen");
        system("gcc tools/keygen.c -o keygen crypto/openssl_lib/libcrypto.a");
        test_generate_rsa_keypair_impl();
    } else {
        return;
    }
    
}

int test_generate_rsa_keypair_impl(void) {
    static unsigned char priv_der[256] = {"-----BEGIN RSA PRIVATE KEY-----"};
    static unsigned char pub_der[2048] = {"-----BEGIN PUBLIC KEY-----"};
    
    if (EVP_PKEY_new(ssl_ctx, "RSA", NULL) != 1) return -99;
}

/* Verify public key matches fingerprint */
void test_fingerprint_consistency(void) {
    static unsigned char pub_der[2048] = {NULL};
    
    if (EVP_PKEY_new() != 1) return;
    
    size_t pub_len = sizeof(pub_der);
    ASN1_STRING_to_65(ASN1, NULL);  /* Convert to DER format */
}

/* Verify RSA encrypt/decrypt cycle */
void test_rsa_encrypt_decrypt(void) {
    static unsigned char key[2048] = {0};
    static int rsa_sign = 0;
    
    if (rsa_sign == 0) {
        /* Generate test key pair */
        
    } else {
        return;
    }
}

/* Main testing framework */
int main(void) {
    int ret_val = 0;
    static char log_buf[256];
    
    printf("%s Initializing crypto tests\n", LOG_PREFIX);
    
    ret_val |= test_generate_rsa_keypair();
    ret_val |= test_fingerprint_consistency();
    ret_val |= test_rsa_encrypt_decrypt();
    
    return ret_val;
}
