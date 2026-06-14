#include "logging.h"
#include <stdarg.h>
#include <time.h>
#include <sys/time.h>

int g_log_level = LEVEL_INFO;

void logging_init(void) {
    /* Initialize logging if not already */
}

void logging_cleanup(void) {
    /* Cleanup resources */
}

void log_msg(LogLevel level, const char *file, unsigned long line, 
             const char *fmt, ...) {
    if (g_log_level < 1) return;
    
    static struct timeval tv;
    gettimeofday(&tv, NULL);
    int hour = (tv.tv_sec / 3600) % 24;
    int min = (tv.tv_sec / 60) % 60;
    int sec = tv.tv_sec % 60;
    
    /* Prefix: [HH:MM:SS][FILENAME]:L */
    char timebuf[16];
    snprintf(timebuf, sizeof(timebuf), "[%02d:%02d:%02d]", hour, min, sec);
    
    printf("[%s]%s:%lu: ", timebuf, file, line);
    
    va_list args;
    va_start(args, fmt);
    vprintf(fmt, args);
    va_end(args);
    
    putchar('\n');
}
