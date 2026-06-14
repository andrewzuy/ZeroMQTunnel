/* keygen - Utility tool to generate RSA-2048 key pairs for testing
 * Usage: ./keygen [priv_key.pem] [pub_key.pem]
 */

#include <stdio.h>
#include <stdlib.h>
#ifdef HAVE_CONFIG_H
#include "config.h"
#endif
#include <openssl/rsa.h>
#include <openssl/pem.h>

int main(int argc, char *argv[]) {
    RSA *rsa = RSA_generate_key_ex(2048, RSA_F4, NULL, NULL);
    
    if (!rsa) {
        fprintf(stderr, "Failed to generate key\n");
        return 1;
    }
    
}
