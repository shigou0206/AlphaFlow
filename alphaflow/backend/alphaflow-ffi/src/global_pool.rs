// backend/ffi_interface/src/global_pool.rs
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use once_cell::sync::Lazy;
use std::sync::Mutex;

// 全局: Lazy + Mutex + Option<Pool>
pub static GLOBAL_DB_POOL: Lazy<Mutex<Option<Pool<ConnectionManager<SqliteConnection>>>>> =
    Lazy::new(|| Mutex::new(None));

// 设置全局池
pub fn set_global_db_pool(pool: Pool<ConnectionManager<SqliteConnection>>) {
    let mut guard = GLOBAL_DB_POOL.lock().unwrap();
    *guard = Some(pool);
}

// 获取全局池
pub fn get_global_db_pool() -> Option<Pool<ConnectionManager<SqliteConnection>>> {
    GLOBAL_DB_POOL.lock().unwrap().clone()
}