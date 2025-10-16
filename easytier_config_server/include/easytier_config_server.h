#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

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
