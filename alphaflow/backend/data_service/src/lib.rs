pub mod db;
pub mod models;
pub mod schema;

use diesel::r2d2::{self, ConnectionManager};
use diesel::sqlite::SqliteConnection;

pub fn establish_connection_pool() -> r2d2::Pool<ConnectionManager<SqliteConnection>> {
    // 使用 /tmp/alphaflow.db 作为数据库文件（适用于模拟器环境）
    let manager = ConnectionManager::<SqliteConnection>::new("/tmp/alphaflow.db");
    r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool.")
}
