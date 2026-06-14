#include "protocol.h"
#include <stdio.h>
#include <time.h>

uint64_t get_timestamp_ms(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (uint64_t)ts.tv_sec * 1000 + ts.tv_nsec/1000000;
}

char* format_hex(const unsigned char *data, size_t len, char *out, size_t out_size) {
    if(len >= out_size) len = out_size - 1;
    
    for(size_t i = 0; i < len; i++) {
        snprintf(out + (i*2), 3, "%02x", data[i]);
    }
    out[len*2] = '\0';
    return out;
}
