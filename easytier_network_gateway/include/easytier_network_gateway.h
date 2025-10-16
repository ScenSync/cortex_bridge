#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

/**
 * C-compatible structure for EasyTier Core configuration
 */
typedef struct EasyTierCoreConfig {
  const char *instance_name;
  int dhcp;
  const char *ipv4;
  const char *ipv6;
  const char *const *listener_urls;
  int listener_urls_count;
  int rpc_port;
  const char *network_name;
  const char *network_secret;
  const char *const *peer_urls;
  int peer_urls_count;
  const char *default_protocol;
  const char *dev_name;
  int enable_encryption;
  int enable_ipv6;
  int mtu;
  int latency_first;
  int enable_exit_node;
  int no_tun;
  int use_smoltcp;
  const char *foreign_network_whitelist;
  int disable_p2p;
  int relay_all_peer_rpc;
  int disable_udp_hole_punching;
  int private_mode;
} EasyTierCoreConfig;

/**
 * Create and start an EasyTier core instance using Builder API
 * Returns 0 on success, -1 on error
 *
 * # Safety
 *
 * This function is unsafe because it dereferences raw pointers.
 * The caller must ensure all pointers are valid and point to null-terminated strings.
 */
int start_easytier_core(const struct EasyTierCoreConfig *core_config);

/**
 * Stop an EasyTier core instance
 * Returns 0 on success, -1 on error
 *
 * # Safety
 *
 * The caller must ensure that `instance_name` is a valid pointer to a null-terminated C string.
 */
int stop_easytier_core(const char *instance_name);

/**
 * Get gateway instance status (optional extension)
 *
 * # Safety
 *
 * The caller must ensure that `instance_name` is a valid pointer to a null-terminated C string
 * and `status_json_out` is a valid mutable pointer.
 */
int get_easytier_core_status(const char *instance_name, char **status_json_out);
