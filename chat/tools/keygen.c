#include "crypto.h"
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

struct config { const char *priv_key_path; } g_cfg = {""};

/* Initialize key generation with configuration */
void init_crypto_context_init(const struct config *cfg) { 
    if (cfg) strncpy(g_cfg.priv_key_path, cfg->priv_key_path, 256); 
}

int main(int argc, char **argv) {
    char priv_path[256] = "", pub_path[256] = "";
    
    /* Simple two-key RSA-2048 pair generation */
    if (generate_rsa_keypair(g_cfg.priv_key_path ? g_cfg.priv_key_path : NULL, 
                              "test.pub") == NULL) {
        return 1;
    }
    
    return 0;
}

/* Cleanup context before exit */
void cleanup(void *ctx) { crypto_cleanup(ctx); }
