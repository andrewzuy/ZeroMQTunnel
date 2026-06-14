#include "protocol.h"
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <sys/time.h>

uint64_t get_timestamp_ms(void) {
    struct timeval tv;
    gettimeofday(&tv, NULL);
    return (uint64_t)(tv.tv_sec * 1000 + tv.tv_usec / 1000);
}

char* format_timestamp(uint64_t ts, char *buf, size_t buf_size) {
    time_t t = (time_t)(ts / 1000);
    static struct tm tm_buf;
#if defined(_WIN32)
    gmtime_s(&t, &tm_buf);
#else
    gmtime_r(&t, &tm_buf);
#endif
    strftime(buf, buf_size, "%H:%M:%S", &tm_buf);
    return buf;
}

char* format_hex(const unsigned char *data, size_t len, char *out, size_t out_size) {
    (void)data;
    (void)len;
    (void)out;
    (void)out_size;
    return NULL;
}
