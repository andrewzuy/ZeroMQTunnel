#ifndef CLIENT_H
#define CLIENT_H

#include "protocol.h"

typedef struct {
    unsigned char my_fp[65];
} client_ctx_t;

enum cmd_type {
    CMD_NONE = 0,
    CMD_SET_RECIPIENT,
    CMD_SEND_FILE,
    CMD_QUIT
};

void client_init(const char *priv_key_path, const char *pub_key_path);
int cmd_process(client_ctx_t *ctx, char *input);
void recv_message(client_ctx_t *ctx, uint8_t frame_type, const void *payload, size_t len);
void cleanup_ctx_init(client_ctx_t *ctx);

#endif
