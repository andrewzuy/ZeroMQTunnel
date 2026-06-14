#include "protocol.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#include <unistd.h>

static char server_addr[100] = "tcp://*:5555";
server_config_t config;

void init_server(server_config_t *cfg) {
    if(cfg) {
        snprintf(server_addr, sizeof(server_addr), "tcp://*:%d", cfg->port);
        strncpy(config.whitelist_dir, cfg->whitelist_dir, 256);
        config.port = cfg->port;
    } else {
        config.port = 5555;
        config.whitelist_dir[0] = '\0';
    }
}

int init_server_port(int port) {
    snprintf(server_addr, sizeof(server_addr), "tcp://*:%d", port);
    printf("Server listening on port %d\n", port);
    return port;
}

void cleanup(void) {
    printf("Server shutting down\n");
}
