#include "client.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static struct { const char *priv_key_path; } g_cli_cfg = {""};

/* Simple client initialization (Phase 2-7 stub) */
void client_init(const char *priv, const char *pub) {
    const char *pr = priv ? priv : "";
    printf("Client init: priv=%s pub=%s\n", pr, pub);
}

int cmd_process(client_ctx_t *ctx, char *input) {
    if (!input || !*input) return 0;
    
    size_t len = strlen(input);
    if (len >= 5 && strncmp(input, "/quit", 5) == 0) {
        printf("Goodbye!\n"); return CMD_QUIT;
    } else if (len >= 4 && strncmp(input, "/to ", 4) == 0) {
        printf("Message to recipient\n"); return CMD_SET_RECIPIENT;
    }
    
    return MSG_FRAME_NONE;
}

void recv_message(client_ctx_t *ctx, uint8_t frame_type, const void *payload, size_t len) {
    printf("Received message (frame=%u, len=%zu)\n", frame_type, len);
}
