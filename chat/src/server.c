#include "server.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* Phase 8+ Zone-MQ stub: ROUTER socket for client relay */
struct { int zctx_id; } g_zmq_stub = {0};

#define SERVER_C 1
