//! 测试清理钩子
//!
//! 这个模块提供了一个测试，它会在所有其他测试完成后运行
//! 用于清理测试过程中创建的所有数据库

#[path = "common/mod.rs"]
mod common;

#[tokio::test]
#[serial_test::serial]
async fn cleanup_all_test_databases() {
    // 这个测试会在所有其他测试完成后运行
    // 确保清理所有测试数据库

    println!("开始清理所有测试数据库...");

    match common::drop_all_test_databases().await {
        Ok(_) => println!("所有测试数据库已清理完成"),
        Err(e) => println!("清理测试数据库时出错: {}", e),
    }

    // 额外清理：查找并删除所有以 cortex_test 开头的数据库
    // 这可以捕获那些可能没有被正确跟踪的测试数据库，但不会影响主数据库
    use sea_orm::{ConnectionTrait, Database as SeaOrmDatabase, DatabaseBackend, Statement};

    let base_url = "mysql://root:root123@127.0.0.1:3306";

    match SeaOrmDatabase::connect(base_url).await {
        Ok(conn) => {
            match conn
                .query_all(Statement::from_string(
                    DatabaseBackend::MySql,
                    "SHOW DATABASES WHERE `Database` LIKE 'cortex_test%'".to_owned(),
                ))
                .await
            {
                Ok(result) => {
                    if result.is_empty() {
                        println!("未找到其他测试数据库");
                        return;
                    }

                    println!("找到以下额外的测试数据库:");
                    for row in &result {
                        if let Ok(db_name) = row.try_get::<String>("", "Database") {
                            println!("  - {}", db_name);

                            // 删除数据库
                            let drop_query = format!("DROP DATABASE `{}`", db_name);
                            match conn
                                .execute(Statement::from_string(DatabaseBackend::MySql, drop_query))
                                .await
                            {
                                Ok(_) => println!("  - 成功删除 {}", db_name),
                                Err(e) => println!("  - 删除 {} 失败: {}", db_name, e),
                            }
                        }
                    }
                }
                Err(e) => println!("查询数据库列表失败: {}", e),
            }

            let _ = conn.close().await;
        }
        Err(e) => println!("连接到 MySQL 服务器失败: {}", e),
    }
}
