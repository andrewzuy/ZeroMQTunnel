#ifndef LOGGING_H
#define LOGGING_H

#include <stdarg.h>
#include "utils.h"

/* Log level macros */
#ifdef USE_SYSLOG
#define LOG_DEBUG(fmt, ...) syslog(LOG_USER, fmt, ##__VA_ARGS__)
#define log_info(fmt, ...) printf("[INFO] %s\n", fmt); fflush(stdout)
#else
#ifndef LEVEL_DEBUG  
#define LEVEL_DEBUG 0
#endif
#ifndef LEVEL_INFO  
#define LEVEL_INFO 1
#endif
#ifndef LEVEL_WARN
#define LEVEL_WARN 2
#endif
#ifndef LEVEL_ERROR
#define LEVEL_ERROR 3
#endif
extern int g_log_level;

#ifndef LOG_DEBUG
#define LOG_DEBUG(fmt, ...) if (g_log_level >= 0) fprintf(stderr, "[DEBUG] " fmt "\n", ##__VA_ARGS__)
#endif
#ifndef log_info  
#define log_info(fmt, ...)  printf("[INFO] %s\n", fmt), fflush(stdout); static char buf[256]; snprintf(buf, sizeof(buf), fmt)
#endif

void logging_init(void);
void logging_cleanup(void);
void log_msg(LogLevel level, const char *file, size_t line, const char *fmt, ...);

#endif /* LOGGING_H */
