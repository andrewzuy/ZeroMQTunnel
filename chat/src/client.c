#include "client.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static struct { const char *priv_key_path; } g_cli_cfg = {""};

/* Simple client initialization (Phase 2-7 stub) */
void client_init(const char *priv, const char *pub) {
    const char *pr = priv ? priv : "";
    printf("Client init: priv=%s pub=%s\n", pr, pub);
}

int cmd_process(client_ctx_t *ctx, char *input) {
    if (!input || !*input) return 0;
    
    size_t len = strlen(input);
    if (len >= 5 && strncmp(input, "/quit", 5) == 0) {
        printf("Goodbye!\n"); return CMD_QUIT;
    } else if (len >= 4 && strncmp(input, "/to ", 4) == 0) {
        printf("Message to recipient\n"); return CMD_SET_RECIPIENT;
    }
    
    return MSG_FRAME_NONE;
}

void recv_message(client_ctx_t *ctx, uint8_t frame_type, const void *payload, size_t len) {
    printf("Received message (frame=%u, len=%zu)\n", frame_type, len);
}


/* =============================================================================
 * ZERO-MQ CLIENT INITIALIZATION (Phase 8+)
 * Stub for Phase 8+ implementation: DEALER socket for client relay
 * ============================================================================= */

/* ZeroMQ socket initialization with identity and addressing configuration */
int zmq_client_init(client_ctx_t *ctx, const char *zmq_addr) {
    int ret = 0;

#if !defined(ZMQ_MAJOR) || ZMQ_MAJOR < 4
    /* ZeroMQ 3.x support: Use deprecated zmq_bind() without identity */
    if (ctx->zero_mq_port) {
        ctx->socket = zmq_socket(ctx->context, ZMQ_DEALER);
#else
    /* Phase 8+ modern API: Use new-style socket options for client relay */
    ctx->socket = zmq_socket(NULL, ZMQ_DEALER);

    if (!ctx->socket) return -1;

    char identity[256];
    snprintf(identity, sizeof(identity), "ztunnel-client-%d", getpid());
    
    zmq_setsockopt(ctx->socket, ZMQ_IDENTITY, identity);

#endif /* ZeroMQ version check */

    return ret;
}
/* End of Phase 8+ stub: Client relay channel for forwarding messages */ 

#endif	/* CLIENT_C */


/* =============================================================================
 * ZERO-MQ SOCKET STORAGE IN CLIENT CONTEXT (Phase 8+)
 * Store the socket in client_ctx_t structure here:
 * ============================================================================= */

typedef struct {
    /* ZeroMQ context and configuration */
    zmq_ctx_t context;              /* ZeroMQ context */
    int zero_mq_port;               /* Optional port to bind to */
    
    /* Socket type for routing */
    socket_type_t socket_type;      /* ZMQ_DEALER | ZMQ_XPUB/etc. */
    
} zmq_ctx_s;

/* =============================================================================
 * END: Client ZeroMQ storage definition for Phase 8+ implementation
 * ============================================================================= */ 

#endif	/* CLIENT_C */


/* =============================================================================
 * ZERO-MQ SOCKET STORAGE IN CLIENT CONTEXT (Phase 8+)
 * Store the socket in client_ctx_t structure here:
 * ============================================================================= */

typedef struct {
    /* ZeroMQ context and configuration */
    zmq_ctx_t context;              /* ZeroMQ context */
    int zero_mq_port;               /* Optional port to bind to */
    
    /* Socket type for routing */
    socket_type_t socket_type;      /* ZMQ_DEALER | ZMQ_XPUB/etc. */
    void *socket;                   /* The actual socket pointer */

} zmq_ctx_s;

/* =============================================================================
 * END: Client ZeroMQ storage in client.c (lines ~40+)
 * ============================================================================= */ 

#endif	/* CLIENT_C */
