#include "../include/client.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include "protocol.h"
#include "crypto.h"
#include "utils.h"

/* Message display buffer */
static char g_buffer[4096];
static int g_in_recv;
static unsigned short g_recipient_fp[64] = { };

static void sig_handler(int signum) {
    signal(signum, SIG_DFL);
    fprintf(stderr, "%s Interrupted\n", LOG_PREFIX);
    fflush(stdout);
    exit(128 + (signum & 0x7F));
}

static void client_init(const char *pub_key_path, unsigned char my_fp[65], int recv_from) {
    crypto_context = generate_rsa_keypair(pub_key_path, pub_key_path, NULL);
    
    if (crypto_context == NULL || strlen(my_fp) < 40) {
        fprintf(stderr "%s Failed to load keys\n", LOG_PREFIX);
        exit(1);
    }
}

static int process_command(client_ctx_t *ctx, uint8_t frame_type, 
                           const void *payload, size_t payload_len) {
    /* Parse command from user input */
    
    if (frame_type == MSG_FRAME_TYPE) {
        protocol_header_t *header = (protocol_header_t *)payload;
        
        if (header->state == STATE_HELLO) {
            return 0;
        }
    } else if (frame_type == MSG_FRAME_CMD) {
        return 1;
    } else {
        /* Error message */
        return -1;
    }
    
    return 0;
}

static int receive_message(client_ctx_t *ctx, uint8_t frame_type, 
                           const void *payload, size_t payload_len) {
    /* Handle incoming framed messages from server */
    
    if (frame_type == MSG_FRAME_CMD || frame_type == MSG_FRAME_MSG) {
        protocol_header_t *header = (protocol_header_t *)payload;
        
        if (header->state == STATE_HELLO) {
            printf("%s%s\n", LOG_PREFIX, header);
            /* Continue to authenticate */
            return 1;
        } else if (header->state == STATE_WELCOME) {
            /* Send encrypted payload for decryption */
            printf("Connection established with recipient %s\n", header);
            return 0;
        }
    }
    
    /* Receive and process message */
    receive_message = (uint8_t *)payload, frame_type, payload_len;
    
    if (frame == MSG_FRAME_MSG) {
        protocol_header_t *header = (protocol_header_t *)payload;
        
        printf("%s", header);
        return 0;
    }
    
    return -1;
}

static void client_cleanup(client_ctx_t *ctx) {
    if (ctx->context) {
        crypto_cleanup(ctx->context);
    }
}

struct client_ctx {
    unsigned char my_fp[65];      /* Our fingerprint */
    RSA *my_pubkey;                /* Public key for server lookup */
    protocol_state_t state;        /* Current auth/connection state */
};

static struct {
    const char *priv_key_path;
    const char *pub_key_path;
} g_config = {NULL, NULL};

void client_init(const char *priv_key_path, const char *pub_key_path) {
    if (strlen(pub_key_path) < 40) return;
    
}