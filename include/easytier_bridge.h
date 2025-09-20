#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef struct CortexWebClient {
  const char *config_server_url;
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

const char *cortex_get_error_msg(void);

void cortex_core_free_string(const char *s);

void cortex_free_instance_list(const char *const *instances, int count);

int cortex_core_set_and_init_console_logging(const char *level, const char *module_name);

int cortex_core_set_and_init_file_logging(const char *level,
                                          const char *module_name,
                                          const char *log_path);

int32_t cortex_web_set_and_init_console_logging(const char *level, const char *module_name);

int32_t cortex_web_set_and_init_file_logging(const char *level,
                                             const char *module_name,
                                             const char *log_path);

const char *cortex_core_get_last_panic(void);

void cortex_core_clear_last_panic(void);

void cortex_core_init_panic_recovery(void);

char *cortex_web_get_last_panic(void);

void cortex_web_clear_last_panic(void);

void cortex_web_init_panic_recovery(void);

void cortex_easytier_web_free_string(char *ptr);

/**
 * Create and start a web client instance that connects to a configuration server
 * Returns 0 on success, -1 on error
 */
int cortex_start_web_client(const struct CortexWebClient *client_config);

/**
 * Stop a web client instance
 * Returns 0 on success, -1 on error
 */
int cortex_stop_web_client(const char *instance_name);

/**
 * Get network information for a web client instance
 * Returns 0 on success, -1 on error
 * The caller must free the returned CortexNetworkInfo using cortex_free_network_info
 */
int cortex_get_web_client_network_info(const char *instance_name,
                                       const struct CortexNetworkInfo **info);

/**
 * List all active web client instances
 * Returns the number of instances, -1 on error
 * The caller must free the returned array using cortex_free_instance_list
 */
int cortex_list_web_client_instances(const char *const **instances, int max_count);

/**
 * Create and start an EasyTier core instance
 * Returns 0 on success, -1 on error
 */
int start_easytier_core(const struct EasyTierCoreConfig *core_config);

/**
 * Stop an EasyTier core instance
 * Returns 0 on success, -1 on error
 */
int stop_easytier_core(const char *instance_name);

/**
 * 创建 NetworkConfigService 单例
 *
 * # Safety
 *
 * 这个函数是不安全的，因为它接受原始指针作为参数
 */
bool create_network_config_service_singleton(const char *db_url,
                                             const char *geoip_path,
                                             char **err_msg);

/**
 * 启动 NetworkConfigService 的监听器
 *
 * # Safety
 *
 * 这个函数是不安全的，因为它接受原始指针作为参数
 */
bool network_config_service_singleton_start(const char *protocol, uint16_t port, char **err_msg);

/**
 * 销毁 NetworkConfigService 实例并释放资源
 *
 * # Safety
 *
 * 这个函数是不安全的
 */
bool destroy_network_config_service_singleton(char **err_msg);

/**
 * 释放由 C 字符串指针占用的内存
 *
 * # Safety
 *
 * 这个函数是不安全的，因为它接受原始指针作为参数
 */
void free_c_char(char *s);

/**
 * 收集单个网络实例信息
 *
 * # Safety
 *
 * 这个函数是不安全的，因为它接受原始指针作为参数
 */
bool network_config_service_collect_one_network_info(const char *org_id,
                                                     const char *device_id,
                                                     const char *inst_id,
                                                     char **result_json_out,
                                                     char **err_msg);

/**
 * 收集多个网络实例信息
 *
 * # Safety
 *
 * 这个函数是不安全的，因为它接受原始指针作为参数
 */
bool network_config_service_collect_network_info(const char *org_id,
                                                 const char *device_id,
                                                 const char *inst_ids_json,
                                                 char **result_json_out,
                                                 char **err_msg);

/**
 * 列出网络实例 ID
 *
 * # Safety
 *
 * 这个函数是不安全的，因为它接受原始指针作为参数
 */
bool network_config_service_list_network_instance_ids(const char *org_id,
                                                      const char *device_id,
                                                      char **result_json_out,
                                                      char **err_msg);

/**
 * 删除网络实例
 *
 * # Safety
 *
 * 这个函数是不安全的，因为它接受原始指针作为参数
 */
bool network_config_service_remove_network_instance(const char *org_id,
                                                    const char *device_id,
                                                    const char *inst_id,
                                                    char **err_msg);

/**
 * 列出设备
 *
 * # Safety
 *
 * 这个函数是不安全的，因为它接受原始指针作为参数
 */
bool network_config_service_list_devices(const char *org_id,
                                         char **result_json_out,
                                         char **err_msg);

/**
 * 更新网络状态
 *
 * # Safety
 *
 * 这个函数是不安全的，因为它接受原始指针作为参数
 */
bool network_config_service_update_network_state(const char *org_id,
                                                 const char *device_id,
                                                 const char *inst_id,
                                                 bool disabled,
                                                 char **err_msg);

/**
 * 验证网络配置
 *
 * # Safety
 *
 * 这个函数是不安全的，因为它接受原始指针作为参数
 */
bool network_config_service_validate_config(const char *org_id,
                                            const char *device_id,
                                            const char *config_json,
                                            char **err_msg);

/**
 * 运行网络实例
 *
 * # Safety
 *
 * 这个函数是不安全的，因为它接受原始指针作为参数
 */
bool network_config_service_run_network_instance(const char *org_id,
                                                 const char *device_id,
                                                 const char *config_json,
                                                 char **inst_id_out,
                                                 char **err_msg);
