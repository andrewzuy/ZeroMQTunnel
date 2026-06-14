#include "crypto.h"
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

static struct config { const char *priv_key_path; } g_cfg = {""};

void init_crypto_context_init(const struct config *cfg) { 
    if (cfg) strncpy((char*)g_cfg.priv_key_path, cfg->priv_key_path, 256);
}

int main(int argc, char **argv) {
    /* Simple two-key RSA-2048 pair */
    generate_rsa_keypair(NULL, "test.pub");
    return 0;
}
