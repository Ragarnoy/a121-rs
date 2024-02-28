#include <stdarg.h>
#include <stdio.h>
#include "../rss/include/acc_definitions_common.h"

extern void rust_log(uint32_t level, const char *message);

void c_log_stub(acc_log_level_t level, const char *module, const char *format, ...) {
    char message_buffer[128] = {0}; // Half the size for the message
    char log_buffer[256] = {0};     // Full size for the final log string

    // va_list args;
    // va_start(args, format);
    // vsnprintf(message_buffer, sizeof(message_buffer), format, args);
    // va_end(args);

    // Safely format the final log string
    // snprintf(log_buffer, sizeof(log_buffer), "%s: %s", module, message_buffer);

    rust_log(level, format); // Call the Rust function
}
