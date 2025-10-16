#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

/**
 * Get last error message
 */
const char *easytier_common_get_error_msg(void);

/**
 * Free a C string allocated by Rust
 */
void easytier_common_free_string(const char *s);

/**
 * Free an array of C strings
 *
 * # Safety
 *
 * The caller must ensure that `arr` was allocated by Rust and contains `count` valid C string pointers.
 */
void easytier_common_free_string_array(const char *const *arr,
                                       int32_t count);

/**
 * FFI wrapper: Initialize console logging
 *
 * # Safety
 *
 * The caller must ensure that `level` and `module_name` are valid C strings.
 */
int easytier_common_init_console_logging(const char *level, const char *module_name);

/**
 * FFI wrapper: Initialize file logging
 *
 * # Safety
 *
 * The caller must ensure that `level`, `module_name`, and `log_path` are valid C strings.
 */
int easytier_common_init_file_logging(const char *level,
                                      const char *module_name,
                                      const char *log_path);
