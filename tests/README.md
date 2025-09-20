# 测试数据库管理

## 问题说明

在 cortex-easytier-web 的测试中，每个测试会创建独立的测试数据库（以 `cortex_` 开头），以实现并发测试。然而，这些测试数据库在测试完成后可能没有被正确清理，导致数据库服务器上残留大量测试数据库。

## 解决方案

我们提供了以下几种方法来清理测试数据库：

### 1. 自动清理机制

现在测试框架已经增强，会自动跟踪创建的测试数据库，并提供了自动清理功能：

- `common.rs` 中的 `CREATED_DBS` 静态变量跟踪所有创建的测试数据库
- `drop_all_test_databases()` 函数可以清理所有已跟踪的测试数据库
- `test_cleanup.rs` 提供了一个会在所有测试完成后运行的测试，自动清理测试数据库

### 2. 手动清理工具

如果自动清理失败或需要手动清理，可以使用以下工具：

#### 使用 Rust 测试清理

运行以下命令清理所有测试数据库：

```bash
cargo test --test drop_test_dbs -- --nocapture
```

或者：

```bash
cargo test --test test_cleanup -- --nocapture
```

#### 使用 Shell 脚本清理

我们还提供了一个 Shell 脚本来清理测试数据库：

```bash
chmod +x scripts/cleanup_test_dbs.sh
./scripts/cleanup_test_dbs.sh
```

## 最佳实践

为避免测试数据库残留问题：

1. 尽量使用 `cargo test` 运行完整测试套件，这样 `test_cleanup.rs` 会自动运行并清理数据库
2. 如果单独运行测试，请在测试完成后手动运行清理工具
3. 定期检查 MySQL 服务器上的数据库列表，确保没有残留的测试数据库

## 注意事项

- 清理脚本会删除所有以 `cortex_` 开头的数据库，请确保不要使用相同前缀命名重要的数据库
- 在生产环境或共享数据库服务器上运行测试时，请特别注意数据库清理