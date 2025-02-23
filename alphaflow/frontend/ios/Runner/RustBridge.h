// RustBridge.h
#ifndef RustBridge_h
#define RustBridge_h

#ifdef __cplusplus
extern "C" {
#endif

// 与 Rust 端 extern "C" fn 签名对应
// 你Rust写了 create_user_ffi, get_user_by_id_ffi, login_user_ffi, free_string_ffi 等

char* create_user_ffi(const char* user_id, const char* email, const char* pass, const char* role);
char* get_user_by_id_ffi(const char* user_id);
char* login_user_ffi(const char* email, const char* pass);
void free_string_ffi(char* ptr);
void init_pool_ffi(const char* db_path);

#ifdef __cplusplus
}
#endif

#endif /* RustBridge_h */