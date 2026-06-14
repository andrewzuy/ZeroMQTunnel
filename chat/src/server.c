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


/* =============================================================================
 * ZERO-MQ CONNECTION INITIALIZATION (Phase 8+)
 * Stub for Phase 8+ implementation: ROUTER socket for client relay forwarding
 * ============================================================================= */

/* ZeroMQ socket initialization with identity and routing configuration */
int zmq_conn_init(server_ctx_t *ctx, const char *zmq_addr) {
    int ret = 0;

#if !defined(ZMQ_MAJOR) || ZMQ_MAJOR < 4
    /* ZeroMQ 3.x support: Use deprecated zmq_bind() without identity */
    ctx->socket = zmq_socket(zctx_id(ctx->context), ZMQ_ROUTER);
#else
    /* Phase 8+ modern API: Use new-style socket options for client relay */
    ctx->socket = zmq_socket(zctx_id(ctx->context), ZMQ_ROUTER);

    zmq_connect(ctx->socket, get_zmq_default_address());
    zmq_disconnect(ctx->socket, get_zmq_default_address());

    char identity[256];
    
    snprintf(identity, sizeof(identity), "ztunnel-%d", zctx_id(ctx->context) % 1000);
    ctx->identity = malloc(strlen(identity) + 1); strcpy((char*)ctx->identity, identity);

    zmq_setsockopt(ctx->socket, ZMQ_ROUTER_HANDOVER, 1);   /* Enable message handover forwarding */
    zmq_setsockopt(ctx->socket, ZMQ_IDENTITY, identity);   

#endif /* ZeroMQ version check */

    return ret;
}
/* End of Phase 8+ stub: Server relay channel for client ↔ server message passing */ 

#endif	/* SERVER_C */
