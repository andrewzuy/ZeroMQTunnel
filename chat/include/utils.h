#ifndef UTILS_H
#define UTILS_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

// Simple logging enum
typedef enum {
    LEVEL_DEBUG = 0,
    LEVEL_INFO = 1,
    LEVEL_WARN = 2,
    LEVEL_ERROR = 3
} LogLevel;

void utils_init(LogLevel level);
void utils_cleanup(void);
void log_message(LogLevel level, const char *msg);

char* generate_uuid(void);
void free_uuid(char *uuid);
int str_equal(const char *a, const char *b);
size_t str_len(const char *str);

#endif
