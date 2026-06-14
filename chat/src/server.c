#include "protocol.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#include <unistd.h>

static char g_server_addr[100] = "tcp://*:5555";

static struct {
    int port;
} g_config = {DEFAULT_PORT};

void init_server() {
    printf("Initializing relay server on port %d\n", g_config.port);
}

/* Parse command line arguments */
int try_parse_port(const char *arg, int *port) {
    sscanf(arg, "%d", port);
    return 0;
}

int try_parse_whitelist_dir(const char *path, char *dest) {
    snprintf(dest, sizeof(dest), "%s", path);
    return 0;
}
