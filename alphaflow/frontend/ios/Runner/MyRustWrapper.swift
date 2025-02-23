import Foundation

class RustDB {
    static func documentsDbPath() -> String {
        let dirs = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)
        guard let docUrl = dirs.first else {
            // fallback to /tmp
            return "/tmp/alpha.db"
        }
        return docUrl.appendingPathComponent("alpha.db").path
    }

    static func initializePool() {
        let dbPath = documentsDbPath()
        let cStr = strdup(dbPath)
        // init_pool_ffi 来自 bridging header, Rust 端的 extern "C"
        init_pool_ffi(cStr)
        free(cStr)
    }
}

class MyRustWrapper {
    static func createUser(userId: String, email: String, pass: String, role: String) -> String {
        let userIdPtr = strdup(userId)
        let emailPtr  = strdup(email)
        let passPtr   = strdup(pass)
        let rolePtr   = strdup(role)

        guard let outPtr = create_user_ffi(userIdPtr, emailPtr, passPtr, rolePtr) else {
            free(userIdPtr); free(emailPtr); free(passPtr); free(rolePtr)
            return "{\"error\":\"create_user_ffi => null pointer\"}"
        }

        let outStr = String(cString: outPtr)
        free_string_ffi(outPtr)

        free(userIdPtr); free(emailPtr); free(passPtr); free(rolePtr)
        return outStr
    }

    static func getUserById(userId: String) -> String {
        let userIdPtr = strdup(userId)
        guard let outPtr = get_user_by_id_ffi(userIdPtr) else {
            free(userIdPtr)
            return "{\"error\":\"get_user_by_id_ffi => null pointer\"}"
        }
        let outStr = String(cString: outPtr)
        free_string_ffi(outPtr)

        free(userIdPtr)
        return outStr
    }

    static func loginUser(email: String, pass: String) -> String {
        let emailPtr = strdup(email)
        let passPtr  = strdup(pass)

        guard let outPtr = login_user_ffi(emailPtr, passPtr) else {
            free(emailPtr); free(passPtr)
            return "{\"error\":\"login_user_ffi => null pointer\"}"
        }

        let outStr = String(cString: outPtr)
        free_string_ffi(outPtr)

        free(emailPtr); free(passPtr)
        return outStr
    }
}