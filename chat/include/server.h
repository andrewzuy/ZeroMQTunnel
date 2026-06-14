#ifndef SERVER_H
#define SERVER_H

#include <stdint.h>

#define LOG_PREFIX "[SERVER]"

// Configuration structure
typedef struct {
    int port;
    char whitelist_dir[256];
} server_config_t;

// Initialize server with configuration
void init_server(server_config_t *config);

// Load fingerprint to public key mapping from file
// Returns: RSA* or NULL on failure
RSA* load_witelist_entry(const char *path);

// Cleanup server resources
void cleanup_server(void);

#endif /* SERVER_H */
