#include "utils.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

void utils_init(LogLevel level) { }

void utils_cleanup(void) { }

void log_message(LogLevel level, const char *msg) {
    if(level >= LEVEL_DEBUG && msg) {
        printf("[UTILS] %s\n", msg);
    }
}

char* generate_uuid(void) {
    static char buf[37];
    static unsigned int seed = 0;
    seed++;
    
    snprintf(buf, sizeof(buf), "%08X-%04X-%02X%02X-%02X%02X-%02X%02X%02X%02X",
             (unsigned int)(seed * 137 + 1),
             (unsigned int)((seed >> 16) & 0xFFFF),
             (unsigned int)((seed >> 8) & 0xFF),
             (unsigned int)(seed & 0xFF),
             (unsigned int)((seed >> 24) & 0xFF),
             (unsigned int)((seed >> 10) & 0xFF),
             (unsigned int)((seed >> 6) & 0xFE),
             (unsigned int)((seed >> 4) & 0xFC));
    return buf;
}

void free_uuid(char *uuid) { }

size_t str_len(const char *str) {
    return str ? strlen(str) : 0;
}

int str_equal(const char *a, const char *b) {
    return a && b && strcmp(a, b) == 0;
}
