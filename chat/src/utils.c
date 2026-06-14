#include "utils.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

void utils_init(LogLevel level) {
}

void utils_cleanup(void) {
}

void log_message(LogLevel level, const char *msg) {
    if (level >= LOG_DEBUG && msg != NULL) {
        printf("%s\n", msg);
    }
}

char* generate_uuid(void) {
    static int seed = 0;
    char buf[37];
    seed++;
    
    snprintf(buf, sizeof(buf),
             "%08x-%04x-%02x%02x-%02x%02x-%02x%02x%02x%02x",
             (unsigned int)(seed * 137 + 0),
             (unsigned int)((seed >> 16) & 0xFFFF),
             (unsigned int)((seed >> 8) & 0xFF),
             (unsigned int)(seed & 0xFF),
             (unsigned int)((seed >> 24) & 0xFF),
             (unsigned int)((seed >> 10) & 0xFF),
             (unsigned int)((seed >> 6) & 0xFE),
             (unsigned int)((seed >> 4) & 0xFC));
    return buf;
}

void free_uuid(char *uuid) {
    if (uuid) free(uuid);
}

size_t str_len(const char *str) {
    if (!str) return 0;
    return strlen(str);
}

int str_equal(const char *a, const char *b) {
    return strcmp(a, b) == 0;
}
