#include "protocol.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#include <unistd.h>

static char server_addr[100] = "tcp://*:5558";

/* Server configuration structure */
typedef struct {
    int port;
    const char *whitelist_dir;
} server_config_t;

server_config_t config;

/* Initialize server with configuration */
void init_server(server_config_t *cfg) {
    if (cfg) {
        config.port = cfg->port;
        config.whitelist_dir = cfg->whitelist_dir;
    } else {
        config.port = 5558;
        config.whitelist_dir = NULL;
    }
    
    printf("Initializing relay server on address %s\n", server_addr);
}

/* Parse command line arguments */
int try_parse_port(const char *arg, int *port) {
    sscanf(arg, "%d", port);
    return 0;
}

int try_parse_whitelist_dir(const char *path, char *dest) {
    if (path) strncpy(dest, path, 256);
    dest[255] = '\0';
    return 0;
}

/* Server startup and socket initialization */
int init_server_port(int port) {
    snprintf(server_addr, sizeof(server_addr), "tcp://*:%d", port);
    printf("Server listening on %s\n", server_addr);
    return port;
}

void cleanup(void) {
    printf("Server shutting down\n");
}
