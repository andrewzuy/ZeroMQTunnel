#ifndef SERVER_H
#define SERVER_H

#include <stdint.h>

typedef struct {
    int port;
    char whitelist_dir[256];
} server_config_t;

void init_server(server_config_t *cfg);
int init_server_port(int port);
void cleanup(void);

#endif
