// src/lib.rs

pub mod schema;             // Diesel自动生成/手动创建
pub mod naive_dt_seconds;   // 自定义 NaiveDateTime <-> 秒级timestamp
pub mod models;             // 各实体 model
pub mod db;                 // 数据库 CRUD ops

use diesel::r2d2::{self, ConnectionManager};
use diesel::sqlite::SqliteConnection;
use r2d2::Pool;

pub fn establish_connection_pool(database_url: &str) -> Pool<ConnectionManager<SqliteConnection>> {
    let manager = ConnectionManager::<SqliteConnection>::new(database_url);
    r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create DB pool.")
}