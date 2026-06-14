#ifndef TEST_CRYPTO_KEYGEN_H
#define TEST_CRYPTO_KEYGEN_H

/* Test RSA-2048 key pair generation */
void test_generate_rsa_keypair(void);

/* Test public key fingerprint computation */
void test_fingerprint_consistency(void);

/* Test signature round-trip */
void test_sign_verify_pair(void);

#endif /* TEST_CRYPTO_KEYGEN_H */
