#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

/**
 * Streaming encoder for generating proper RRD format from MCAP data
 * This uses `re_log_encoding::Encoder` which generates valid RRD files with `RRF2` headers
 */
typedef struct RerunStreamingEncoder RerunStreamingEncoder;

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
 * Create a new streaming encoder
 * This is the CORRECT way to generate RRD format for streaming
 */
struct RerunStreamingEncoder *rerun_encoder_create(const char *application_id);

/**
 * Process MCAP chunk and return RRD bytes
 * This converts MCAP data to RRD format and returns only new data since last call
 */
int32_t rerun_encoder_process_mcap_chunk(struct RerunStreamingEncoder *handle,
                                         const uint8_t *mcap_data,
                                         uintptr_t mcap_len,
                                         uint8_t **out_data,
                                         uintptr_t *out_len);

/**
 * Get initial RRD header chunk (call immediately after creation)
 * This returns the RRF2 header + metadata before any data is logged
 */
int32_t rerun_encoder_get_initial_chunk(struct RerunStreamingEncoder *handle,
                                        uint8_t **out_data,
                                        uintptr_t *out_len);

/**
 * Destroy streaming encoder
 */
void rerun_encoder_destroy(struct RerunStreamingEncoder *handle);
