#ifndef WRAPPER_H
#define WRAPPER_H

#include "../rss/include/arm/acc_definitions_common.h"
#include <stdarg.h>
#include <stdio.h>

void c_log_stub(acc_log_level_t level, const char *module, const char *format,
                ...);

#endif // WRAPPER_H
