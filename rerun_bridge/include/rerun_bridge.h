#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

#define INT8 1

#define UINT8 2

#define INT16 3

#define UINT16 4

#define INT32 5

#define UINT32 6

#define FLOAT32 7

#define FLOAT64 8

/**
 * Opaque handle to a Rerun recording
 */
typedef struct RerunRecording RerunRecording;

/**
 * Opaque handle to a streaming Rerun recording
 * This maintains state across multiple chunk writes for incremental streaming
 */
typedef struct RerunStreamingRecording RerunStreamingRecording;

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

/**
 * Create a new streaming Rerun recording
 * This recording can be used to incrementally add data and extract RRD chunks
 */
struct RerunStreamingRecording *rerun_create_streaming_recording(const char *application_id);

/**
 * Destroy a streaming Rerun recording
 */
void rerun_destroy_streaming_recording(struct RerunStreamingRecording *handle);

/**
 * Log image data to streaming recording
 */
int32_t rerun_streaming_log_image(struct RerunStreamingRecording *handle,
                                  const char *entity_path,
                                  uint32_t width,
                                  uint32_t height,
                                  const uint8_t *data,
                                  uintptr_t data_len);

/**
 * Flush and get any new RRD data from the streaming recording
 * Returns new RRD chunk data that should be sent to client
 * This is non-destructive - the recording stream continues
 */
int32_t rerun_streaming_flush_chunk(struct RerunStreamingRecording *handle,
                                    uint8_t **out_data,
                                    uintptr_t *out_len);
