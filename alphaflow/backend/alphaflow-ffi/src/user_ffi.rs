use libc::c_char;
use crate::ffi_util::{to_c_string, from_c_str};
use alphaflow_sqlite::{
    db::user_ops,
    models::user::NewUser,
};
use serde_json;

#[no_mangle]
pub extern "C" fn create_user_ffi(
    user_id: *const c_char,
    email: *const c_char,
    pass: *const c_char,
    role: *const c_char,
) -> *mut c_char {
    let user_id_str = from_c_str(user_id);
    let email_str   = from_c_str(email);
    let pass_str    = from_c_str(pass);
    let role_str    = from_c_str(role);

    // 改：从全局池里拿
    let pool_opt = crate::global_pool::get_global_db_pool();
    if pool_opt.is_none() {
        let msg = r#"{"error":"No global pool => call init_pool_ffi first"}"#.to_string();
        return to_c_string(msg);
    }
    let pool = pool_opt.unwrap();

    let mut conn = match pool.get() {
        Ok(c) => c,
        Err(e) => {
            let err = format!(r#"{{"error":"pool.get() failed: {e}"}}"#);
            return to_c_string(err);
        }
    };

    let new_u = NewUser {
        id: if user_id_str.is_empty() { None } else { Some(&user_id_str) },
        email: &email_str,
        password_hash: &pass_str,
        role: &role_str,
    };

    // 这里 Diesel insert:
    match user_ops::create_user(&mut conn, &new_u) {
        Ok(user) => {
            // 序列化 User => JSON
            let out_json = serde_json::to_string(&user).unwrap();
            to_c_string(out_json)
        }
        Err(e) => {
            let err = format!(r#"{{"error":"{:?}"}}"#, e);
            to_c_string(err)
        }
    }
}

#[no_mangle]
pub extern "C" fn get_user_by_id_ffi(user_id: *const c_char) -> *mut c_char {
    let uid_str = from_c_str(user_id);

    let pool_opt = crate::global_pool::get_global_db_pool();
    if pool_opt.is_none() {
        let msg = r#"{"error":"No global pool => call init_pool_ffi first"}"#.to_string();
        return to_c_string(msg);
    }
    let pool = pool_opt.unwrap();

    let mut conn = match pool.get() {
        Ok(c) => c,
        Err(e) => {
            let err = format!(r#"{{"error":"pool.get() failed: {e}"}}"#);
            return to_c_string(err);
        }
    };

    match user_ops::get_user_by_id(&mut conn, &uid_str) {
        Ok(u) => {
            let out_json = serde_json::to_string(&u).unwrap();
            to_c_string(out_json)
        },
        Err(e) => {
            let err = format!(r#"{{"error":"{:?}"}}"#, e);
            to_c_string(err)
        },
    }
}

#[no_mangle]
pub extern "C" fn login_user_ffi(
    email_ptr: *const c_char,
    pass_ptr: *const c_char,
) -> *mut c_char {
    let email_str = from_c_str(email_ptr);
    let pass_str  = from_c_str(pass_ptr);

    let pool_opt = crate::global_pool::get_global_db_pool();
    if pool_opt.is_none() {
        let msg = r#"{"error":"No global pool => call init_pool_ffi first"}"#.to_string();
        return to_c_string(msg);
    }
    let pool = pool_opt.unwrap();

    let mut conn = match pool.get() {
        Ok(c) => c,
        Err(e) => {
            let err = format!(r#"{{"error":"pool.get() failed: {e}"}}"#);
            return to_c_string(err);
        }
    };

    // user_ops::login_user => Diesel query ...
    match user_ops::login_user(&mut conn, &email_str, &pass_str) {
        Ok(u) => {
            let out_json = serde_json::to_string(&u).unwrap();
            to_c_string(out_json)
        }
        Err(e) => {
            let err = format!(r#"{{"error":"{:?}"}}"#, e);
            to_c_string(err)
        }
    }
}