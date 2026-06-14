#include "client.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

void client_init(const char *priv_key_path, const char *pub_key_path) {
    printf("Client initializing\n");
}

int cmd_process(client_ctx_t *ctx, char *input) {
    if(!input || !*input) return 0;
    
    size_t len = strlen(input);
    
    if(len >= 5 && strncmp(input, "/quit", 5) == 0) {
        printf("Goodbye!\n");
        return CMD_QUIT;
    } else if(len >= 4 && strncmp(input, "/to ", 4) == 0) {
        printf("Message to recipient\n");
        return CMD_SET_RECIPIENT;
    }
    
    return 0;
}

void recv_message(client_ctx_t *ctx, uint8_t frame_type, const void *payload, size_t len) {
    printf("Received message\n");
}

void cleanup_ctx_init(client_ctx_t *ctx) { }
