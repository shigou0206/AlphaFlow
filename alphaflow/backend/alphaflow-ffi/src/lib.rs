// backend/ffi_interface/src/lib.rs

mod ffi_util;       // to_c_string, from_c_str, free_string_ffi
mod global_pool;    // set_global_db_pool, get_global_db_pool
mod user_ffi;       // create_user_ffi, get_user_by_id_ffi, login_user_ffi (用全局池)

use std::os::raw::c_char;
use std::ffi::CStr;
use diesel::r2d2::ConnectionManager;
use diesel::sqlite::SqliteConnection;
use crate::global_pool::set_global_db_pool;

/// 1) 初始化连接池 (iOS/Android侧先调这个)
#[no_mangle]
pub extern "C" fn init_pool_ffi(db_path_ptr: *const c_char) {
    if db_path_ptr.is_null() {
        eprintln!("init_pool_ffi: db_path is null => skip");
        return;
    }
    let c_str = unsafe { CStr::from_ptr(db_path_ptr) };
    let db_path = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => {
            eprintln!("init_pool_ffi: invalid UTF-8");
            return;
        }
    };
    let manager = ConnectionManager::<SqliteConnection>::new(db_path);
    let pool = match diesel::r2d2::Pool::builder().build(manager) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("init_pool_ffi: build pool failed => {e}");
            return;
        }
    };
    set_global_db_pool(pool);
    eprintln!("init_pool_ffi => set pool with path: {}", db_path);
}
