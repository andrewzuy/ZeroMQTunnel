#ifndef PROTOCOL_ZEROMQ_H
#define PROTOCOL_ZEROMQ_H 1

#include <stdint.h>
#include <zmq.h>

/* Forward declarations of key types */
typedef struct server_ctx server_ctx_t;
typedef struct client_ctx client_ctx_t;

typedef struct {
    int zero_mq_port;                 /* ZeroMQ port for ROUTER/DEALER sockets */
    const char *identity_prefix;      /* ZeroMQ identity prefix (e.g., "ztunnel-") */
    socket_type_t zmq_socktype_used;   /* ZMQ_PAIR | ZMQ_STREAM/etc. */
} zmq_config_s;

typedef struct {
    char route_addr[2048];            /* ROUTER: tcp://server.example.com:7771 */
    char dealer_addr[2048];           /* DEALER (clients): tcp://:5560 */
    int zctx_id;                      /* ZeroMQ context ID for debugging */
} zmq_socket_config;

socket_type_t get_zmq_socket_type(void);

#endif /* PROTOCOL_ZEROMQ_H 1 */
