#ifndef LOGGING_H
#define LOGGING_H

#include "../include/utils.h"

#define LOG_DEBUG(msg)  printf("[DEBUG] %s\n", msg)
#define LOG_INFO(msg)   printf("[INFO] %s\n", msg)
#define LOG_WARN(msg)   printf("[WARN] %s\n", msg)
#define LOG_ERROR(msg)  fprintf(stderr, "[ERROR] %s\n", msg)

void logging_init(void);
void logging_cleanup(void);

#endif
