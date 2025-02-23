use std::ffi::{CString, CStr};
use libc::c_char;

/// 把 Rust String 转成 C风格字符串 (*mut c_char)
pub fn to_c_string(s: String) -> *mut c_char {
    // 如果 s 内含 `\0`，CString::new 会 panic，可自行处理
    CString::new(s).unwrap().into_raw()
}

/// 把 C字符串(*const c_char) 转成 Rust String
pub fn from_c_str(raw: *const c_char) -> String {
    if raw.is_null() {
        return String::new();
    }
    let cstr = unsafe { CStr::from_ptr(raw) };
    cstr.to_string_lossy().into_owned()
}

/// 释放由 `to_c_string` 分配的内存
///
/// 和 Swift bridging header 保持同名:
///   extern void free_string_ffi(char*);
#[no_mangle]
pub extern "C" fn free_string_ffi(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = CString::from_raw(ptr);
        }
    }
}