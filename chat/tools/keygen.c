#include <stdio.h>
#include <stdlib.h>
#include "crypto.h"

int main(int argc, char **argv) {
    const char *priv_path = NULL;
    const char *pub_path = NULL;
    
    if(argc >= 3) {
        priv_path = argv[1];
        pub_path = argv[2];
    } else if(argc == 2) {
        priv_path = argv[1];
        pub_path = priv_path;
    }
    
    printf("Generating RSA-2048 keypair...\n");
    crypto_context = generate_rsa_keypair(priv_path, pub_path);
    
    return crypto_context ? 0 : 1;
}
