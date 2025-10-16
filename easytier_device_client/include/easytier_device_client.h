#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef struct CortexWebClient {
  const char *config_server_url;
  const char *machine_id;
} CortexWebClient;

typedef struct CortexNetworkInfo {
  const char *instance_name;
  const char *network_name;
  const char *virtual_ipv4;
  const char *hostname;
  const char *version;
  int peer_count;
  int route_count;
} CortexNetworkInfo;

/**
 * Start web client in config mode
 *
 * # Safety
 *
 * The caller must ensure that `client_config` is a valid pointer to a properly initialized `CortexWebClient` struct.
 */
int cortex_start_web_client(const struct CortexWebClient *client_config);

/**
 * Stop web client
 *
 * # Safety
 *
 * The caller must ensure that `instance_name` is a valid pointer to a null-terminated C string.
 */
int cortex_stop_web_client(const char *instance_name);

/**
 * Get network info
 *
 * # Safety
 *
 * The caller must ensure that `instance_name` is a valid pointer to a null-terminated C string
 * and `info` is a valid mutable pointer.
 */
int cortex_get_web_client_network_info(const char *instance_name,
                                       const struct CortexNetworkInfo **info);

/**
 * List web client instances
 *
 * # Safety
 *
 * The caller must ensure that `instances` is a valid mutable pointer.
 */
int cortex_list_web_client_instances(const char *const **instances, int max_count);
