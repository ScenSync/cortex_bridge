#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

/**
 * Opaque handle to a Rerun recording
 */
typedef struct RerunRecording RerunRecording;

/**
 * Get last error message
 */
const char *rerun_bridge_get_error(void);

/**
 * Free a C string allocated by Rust
 */
void rerun_bridge_free_string(const char *s);

/**
 * Free RRD data buffer
 */
void rerun_bridge_free_rrd_data(uint8_t *data, uintptr_t len);

/**
 * Create a new Rerun recording
 */
struct RerunRecording *rerun_create_recording(const char *application_id);

/**
 * Destroy a Rerun recording
 */
void rerun_destroy_recording(struct RerunRecording *handle);

/**
 * Log image data to recording
 */
int32_t rerun_log_image(struct RerunRecording *handle,
                        const char *entity_path,
                        uint32_t width,
                        uint32_t height,
                        const uint8_t *data,
                        uintptr_t data_len);

/**
 * Save recording to RRD format
 */
int32_t rerun_save_to_rrd(struct RerunRecording *handle, uint8_t **out_data, uintptr_t *out_len);
