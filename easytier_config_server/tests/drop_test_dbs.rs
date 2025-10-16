//! 用于清理测试数据库的工具
//!
//! 这个模块提供了一个独立的测试，可以用来清理所有测试数据库
//! 运行方式: cargo test --test drop_test_dbs -- --nocapture

#[path = "common/mod.rs"]
mod common;

use sea_orm::{ConnectionTrait, Database as SeaOrmDatabase, DatabaseBackend, Statement};

#[tokio::test]
async fn drop_all_test_databases() {
    // 数据库连接信息
    let base_url = "mysql://root:root123@127.0.0.1:3306";

    println!("连接到 MySQL 服务器...");
    let conn = SeaOrmDatabase::connect(base_url)
        .await
        .expect("无法连接到 MySQL 服务器");

    println!("查询所有测试数据库...");
    let result = conn
        .query_all(Statement::from_string(
            DatabaseBackend::MySql,
            "SHOW DATABASES WHERE `Database` LIKE 'cortex_test%'".to_owned(),
        ))
        .await
        .expect("查询数据库列表失败");

    if result.is_empty() {
        println!("未找到测试数据库");
        return;
    }

    println!("找到以下测试数据库:");
    let mut count = 0;
    for row in &result {
        let db_name: String = row.try_get("", "Database").expect("无法获取数据库名");
        println!("  - {}", db_name);
        count += 1;
    }

    println!("\n准备删除 {} 个测试数据库", count);
    println!("按 Ctrl+C 取消，或等待 5 秒继续...");
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    for row in result {
        let db_name: String = row.try_get("", "Database").expect("无法获取数据库名");
        println!("删除数据库: {}", db_name);

        let drop_query = format!("DROP DATABASE `{}`", db_name);
        match conn
            .execute(Statement::from_string(DatabaseBackend::MySql, drop_query))
            .await
        {
            Ok(_) => println!("  - 成功删除 {}", db_name),
            Err(e) => println!("  - 删除 {} 失败: {}", db_name, e),
        }
    }

    println!("\n测试数据库清理完成");
}
