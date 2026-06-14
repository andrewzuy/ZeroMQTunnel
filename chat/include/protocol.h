#ifndef PROTOCOL_H
#define PROTOCOL_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include "crypto.h"  /* For crypto_key_t */

/* Message frame types */
typedef enum {
    MSG_FRAME_NONE     = 0x00,   /* Handshake/auth frames */
    MSG_FRAME_TYPE     = 0x01,   /* Protocol header identifier */
    MSG_FRAME_CMD      = 0x02,   /* Client command (text) */
    MSG_FRAME_MSG      = 0x03    /* Encrypted payload message */
} msg_frame_t;

/* Auth states in handshake protocol */
typedef enum {
    STATE_NONE         = 0,
    STATE_HELLO        = 1,      /* CLIENT: HELLO<fingerprint> → SERVER */
    STATE_CHALLENGE,             /* SERVER: CHALLENGE<nonce> ↑ to client */
    STATE_AUTH,                  /* CLIENT: AUTH<signature> ← signature of nonce */
    STATE_WELCOME              /* SERVER: accept + encrypt forward channel */
} protocol_state_t;

/* Secure payload structure (variable length) */
typedef struct {
    uint64_t timestamp;          /* UTC millisecond for replay protection */
    
    /* Inline RSA encrypted AES session key + IV after it */
    size_t rsa_encrypted_key_len;  /* Length of enc'd AES key (max EVP_PKEY_size) */
    unsigned char rsa_encrypted_key[1];  /* Flexible array member */
} secure_payload_t;

/* Protocol header (fixed length, frame type identifier) */
typedef struct {
    msg_frame_t frame_type;      /* Always 0x00 for MSG frames, 0x01 for handshake */
} protocol_header_t;

/* Connection state (used by server for whitelisted clients) */
typedef struct {
    char identity[256];          /* ZeroMQ identity or client name */
    char fingerprint[65];         /* hex SHA256 (64 chars + null) */
    EVP_PKEY *public_key;         /* loaded public key for recipient lookup */
    protocol_state_t state;       /* current handshake state per connection */
    bool authenticated;           /* true after successful AUTH handshake */
} conn_t;

/* Protocol utility functions */
uint64_t get_timestamp_ms(void);
char* format_hex(const unsigned char *data, size_t len, char *out, size_t out_size);
void protocol_cleanup_conn(conn_t *conn);

#endif /* PROTOCOL_H */
