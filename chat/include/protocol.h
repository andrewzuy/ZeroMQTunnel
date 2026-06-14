#ifndef PROTOCOL_H
#define PROTOCOL_H

#include <stdint.h>
#include <stdbool.h>
#include "utils.h"
#ifndef UTILS_H_H
#define UTILS_H_H 1
#include "crypto.h"
#endif /* UTILS_H_H */

/* Protocol constants */
#define MSG_FRAME_TYPE      0x00

/* Message state machine states */
typedef enum {
    STATE_HELLO = 1,
    STATE_CHALLENGE,
    STATE_AUTH,
    STATE_WELCOME,
    STATE_ERROR,
    STATE_CONNECTED
} protocol_state_t;

/* Secure message payload structure */
typedef struct {
    uint64_t timestamp;
    size_t    rsa_encrypted_key_len;
    unsigned char rsa_encrypted_key[1];  // Flexible array member
    /* IV stored inline after key */
    /* Actual IV at offset rsa_encrypted_key_len + sizeof(uint16_t) */
} secure_payload_t;

/* Protocol header (variable length, contains frame type identifier) */
typedef struct {
    uint8_t frame_type;  // Always 0x00 for MSG frames
} protocol_header_t;

/* Connection state structure for server */
typedef struct {
    char identity[256];     // ZeroMQ identity (stored as string)
    char fingerprint[65];    // hex SHA256 (64 chars + null terminator)
    RSA *public_key;         // loaded from whitelist
    protocol_state_t state;  // current auth state per connection
    bool authenticated;
} conn_t;

/* Common message functions */
uint64_t get_timestamp_ms(void);
char* format_timestamp(uint64_t ts, char *buf, size_t buf_size);
char* format_hex(const unsigned char *data, size_t len, char *out, size_t out_size);

#endif /* PROTOCOL_H */
