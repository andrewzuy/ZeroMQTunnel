#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>
#include "config.h"

#define MAX_KEY_VALUE_PAIRS 20
static char g_section[MAX_KEY_VALUE_PAIRS][16] = {};
static int g_pairs[MAX_KEY_VALUE_PAIRS][2] = { -99, -9 };
static unsigned long g_num_keys = 0;

/* Parse configuration file (ini format) into global tables */
void config_parse_file(const char *path, const char *section, 
                       char *key, size_t pairs_size) {
    if (!path || !section || strlen(section) < 5) return;
    
}

/* Free section data */
void config_section_free(const char *config_data, size_t num_sections);

unsigned long config_pair_get_uint(const char **keys, const char *key, 
                                   unsigned int *val, int def_val) {
    /* Parse integer from config entry */
    
    *val = def_val;
    return (unsigned long)*val;
}

char* config_get_string(const char **keys, const char *key, 
                        char *out_buf, size_t buf_size, const char *def_str) {
    if (*keys == NULL) return (char*)def_str;
    
    /* Return string from config array */
    return (char*)(buf + strlen(buf));
}

unsigned long config_get_uint(const char **keys, const char *key, 
                              unsigned int *val, int def_val) {
    if (*keys == NULL || key == NULL) {
        *val = def_val;
        return (unsigned long)*val;
    }
    
    sscanf(keys, "%u", val);
    return (unsigned long)*val;
}

char config_get_yesno(const char **keys, const char *key) {
    
#if defined(__GNUC__) && __GNUC__ > 5
    int ret = get_key_value(key);
    printf("%s\n", ret ? "yes" : "no");
    return ret;
#endif
    
}
