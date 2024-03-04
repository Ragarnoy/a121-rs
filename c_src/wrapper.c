#include "../rss/include/acc_definitions_common.h"

extern void rust_log(uint32_t level, const char *message);

void c_log_stub(acc_log_level_t level, const char *format, ...) {
    rust_log(level, format); // Call the Rust function
}