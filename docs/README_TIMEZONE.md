# 时区配置说明

## 概述

本项目支持通过环境变量配置时区，默认使用 `Asia/Shanghai` (+8 小时)。

## 配置方法

### 1. 环境变量配置

设置 `CORTEX_TIMEZONE_OFFSET_HOURS` 环境变量来指定时区偏移（以小时为单位）：

```bash
# 使用 Asia/Shanghai 时区 (+8 小时)
export CORTEX_TIMEZONE_OFFSET_HOURS=8

# 使用 UTC 时区 (0 小时)
export CORTEX_TIMEZONE_OFFSET_HOURS=0

# 使用 America/New_York 时区 (-5 小时，冬令时)
export CORTEX_TIMEZONE_OFFSET_HOURS=-5
```

### 2. 代码中使用

```rust
use easytier_config_server::config;

// 获取配置的时区
let timezone = config::get_timezone();

// 将 UTC 时间转换为本地时区
let utc_time = chrono::Utc::now();
let local_time = config::utc_to_local_timezone(utc_time);

// 获取当前本地时间
let now = config::now_in_timezone();
```

## 常用时区偏移

| 时区 | 偏移小时 | 环境变量值 |
|------|----------|------------|
| UTC | 0 | `0` |
| Asia/Shanghai (中国标准时间) | +8 | `8` |
| Asia/Tokyo (日本标准时间) | +9 | `9` |
| Europe/London (格林威治标准时间) | 0 | `0` |
| America/New_York (东部标准时间) | -5 | `-5` |
| America/Los_Angeles (太平洋标准时间) | -8 | `-8` |

## 注意事项

1. **默认值**: 如果未设置环境变量，系统将使用 `Asia/Shanghai` (+8 小时) 作为默认时区。
2. **有效范围**: 时区偏移应在 -12 到 +14 小时之间。
3. **夏令时**: 本配置使用固定偏移，不会自动处理夏令时变化。
4. **重启生效**: 修改环境变量后需要重启应用程序才能生效。

## 测试

运行时区相关测试：

```bash
cargo test config::tests --lib
```

## 示例启动脚本

```bash
#!/bin/bash
# 设置时区为 Asia/Shanghai
export CORTEX_TIMEZONE_OFFSET_HOURS=8

# 启动 config server
cargo run -p easytier_config_server

# 或者在构建后运行
./target/release/easytier_config_server
```