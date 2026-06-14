#ifndef PROTOCOL_H
#define PROTOCOL_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include "server.h"
#include "crypto.h"  /* crypto_context extern */

/* Message frame types */
typedef enum {
    MSG_FRAME_NONE = 0,
    MSG_FRAME_TYPE,
    MSG_FRAME_CMD,
    MSG_FRAME_MSG
} msg_frame_t;

/* Protocol states - add STATE_NONE */
typedef enum {
    STATE_NONE = 0,
    STATE_HELLO = 1,
    STATE_CHALLENGE,
    STATE_AUTH,
    STATE_WELCOME,
    STATE_ERROR,
    STATE_CONNECTED
} protocol_state_t;

/* Secure payload structure */
typedef struct {
    uint64_t timestamp;      /* UTC ms for replay protection */
    size_t rsa_encrypted_key_len;
    unsigned char rsa_encrypted_key[1];  /* flexible array member */
} secure_payload_t;

/* Protocol header */
typedef struct {
    msg_frame_t frame_type;  /* Always 0x00 for MSG frames */
} protocol_header_t;

uint64_t get_timestamp_ms(void);
char* format_hex(const unsigned char *data, size_t len, char *out, size_t out_size);

#endif
