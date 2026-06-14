#ifndef CLIENT_H
#define CLIENT_H

#include "../include/protocol.h"
#include "../include/crypto.h"

/* Client internal state */
typedef struct {
    RSA *my_priv_key;            // My private key for signing/decrypting
    RSA *my_pub_key;             // My public key (for server lookup)
    unsigned char my_fp[65];     // Hex fingerprint string
    protocol_state_t state;      // Current auth/connection state
} client_ctx_t;

/* Client command types */
typedef enum {
    CMD_NONE = 0,
    CMD_SET_RECIPIENT = '/to',
    CMD_SEND_FILE = '/bin',
    CMD_QUIT = '/quit'
} cmd_type_t;

/* Initialize client and generate/load keys */
void client_init(const char *priv_key_path, const char *pub_key_path,
                 unsigned char *fp_out);

/* Process user command (text input from stdin) */
int process_command(client_ctx_t *ctx, char *input);

/* Receive message handler */
void receive_message(client_ctx_t *ctx, uint8_t frame_type, 
                     const void *payload, size_t payload_len);

/* Send encrypted message to server */
void send_to_server(client_ctx_t *ctx, unsigned char *payload, size_t len);

/* Cleanup client */
void client_cleanup(client_ctx_t *ctx);

#endif /* CLIENT_H */
