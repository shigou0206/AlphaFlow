#!/usr/bin/env bash
set -e  # 遇到错误即退出

#######################################
# 彩色打印函数
#######################################
YELLOW="\e[93m"
GREEN="\e[32m"
RED="\e[31m"
ENDCOLOR="\e[0m"

function printMsg() {
    echo -e "${YELLOW}alphaflow : $1${ENDCOLOR}"
}

function printSuccess() {
    echo -e "${GREEN}alphaflow : $1${ENDCOLOR}"
}

#######################################
# 1. 创建 alphaflow 目录结构
#######################################
printMsg "1. Creating alphaflow structure..."
mkdir -p alphaflow
cd alphaflow

mkdir -p backend
mkdir -p frontend

#######################################
# 2. 创建 data_service (Rust + Diesel)
#######################################
printMsg "2. Creating data_service crate..."
cd backend
cargo new data_service --lib

cd data_service
cat <<EOF > Cargo.toml
[package]
name = "data_service"
version = "0.1.0"
edition = "2021"

[dependencies]
diesel = { version = "2.2.7", features = ["sqlite", "r2d2"] }
r2d2 = "0.8"
r2d2-diesel = "1.0.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
EOF

# 创建目录结构：db, models, schema
mkdir -p src/db src/models src/schema

# src/lib.rs
cat <<EOF > src/lib.rs
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
EOF

# src/db/mod.rs
cat <<EOF > src/db/mod.rs
pub mod user_ops;
EOF

# src/db/user_ops.rs
cat <<EOF > src/db/user_ops.rs
use diesel::prelude::*;
use diesel::insert_into;

use crate::schema::users;
use crate::models::user::{NewUser, User};

pub fn create_user(conn: &mut SqliteConnection, new_user: NewUser) -> QueryResult<User> {
    insert_into(users::table)
        .values(&new_user)
        .execute(conn)?;
    users::table.order(users::id.desc()).first(conn)
}

pub fn get_user_by_id(conn: &mut SqliteConnection, user_id: i32) -> QueryResult<User> {
    use crate::schema::users::dsl::*;
    users.filter(id.eq(user_id)).first(conn)
}
EOF

# src/models/mod.rs
cat <<EOF > src/models/mod.rs
pub mod user;
EOF

# src/models/user.rs
cat <<EOF > src/models/user.rs
use diesel::prelude::*;
use crate::schema::users;
use serde::{Serialize, Deserialize};

#[derive(Queryable, Serialize, Deserialize, Debug)]
#[diesel(table_name = users)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub email: String,
    pub hashed_password: String,
}

#[derive(Insertable)]
#[diesel(table_name = users)]
pub struct NewUser<'a> {
    pub username: &'a str,
    pub email: &'a str,
    pub hashed_password: &'a str,
}
EOF

# src/schema/mod.rs
cat <<EOF > src/schema/mod.rs
use diesel::prelude::*;

diesel::table! {
    users (id) {
        id -> Integer,
        username -> Text,
        email -> Text,
        hashed_password -> Text,
    }
}
EOF

cd ..  # 回到 alphaflow/backend

#######################################
# 3. 创建 ffi_interface (Rust + .dylib)
#######################################
printMsg "3. Creating ffi_interface crate..."
cargo new ffi_interface --lib
cd ffi_interface

cat <<EOF > Cargo.toml
[package]
name = "ffi_interface"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
data_service = { path = "../data_service" }
diesel = { version = "2.2.7", features = ["sqlite", "r2d2"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
libc = "0.2"

[dev-dependencies]
cbindgen = "0.24"
EOF

cat <<EOF > src/lib.rs
use libc::c_char;
use std::ffi::{CString, CStr};

use data_service::db::user_ops;
use data_service::models::user::NewUser;
use data_service::establish_connection_pool;

fn to_c_string(s: String) -> *mut c_char {
    CString::new(s).unwrap().into_raw()
}

fn from_c_str(raw: *const c_char) -> String {
    if raw.is_null() {
        return "".to_string();
    }
    let cstr = unsafe { CStr::from_ptr(raw) };
    cstr.to_string_lossy().into_owned()
}

#[no_mangle]
pub extern "C" fn free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe { let _ = CString::from_raw(ptr); }
    }
}

#[no_mangle]
pub extern "C" fn create_user_ffi(
    username: *const c_char,
    email: *const c_char,
    password: *const c_char
) -> *mut c_char {
    let username_str = from_c_str(username);
    let email_str = from_c_str(email);
    let password_str = from_c_str(password);

    let pool = establish_connection_pool();
    let mut conn = pool.get().expect("Failed to get conn from pool");

    let new_user = NewUser {
        username: &username_str,
        email: &email_str,
        hashed_password: &password_str,
    };

    let result = user_ops::create_user(&mut conn, new_user);

    let out_json = match result {
        Ok(u) => serde_json::to_string(&u).unwrap(),
        Err(_) => "{\"error\":\"Failed to create user\"}".to_string(),
    };
    to_c_string(out_json)
}

#[no_mangle]
pub extern "C" fn get_user_by_id_ffi(user_id: i32) -> *mut c_char {
    let pool = establish_connection_pool();
    let mut conn = pool.get().expect("Failed to get conn from pool");

    let result = user_ops::get_user_by_id(&mut conn, user_id);
    let out_json = match result {
        Ok(u) => serde_json::to_string(&u).unwrap(),
        Err(_) => "{\"error\":\"User not found\"}".to_string(),
    };
    to_c_string(out_json)
}
EOF

#######################################
# 4. 编译 ffi_interface for iOS simulator (arm64)
#######################################
printMsg "4. Compiling ffi_interface for iOS simulator (arm64)..."
rustup target add aarch64-apple-ios-sim >/dev/null 2>&1 || true
cargo build --release --target aarch64-apple-ios-sim

printMsg "Build success => target/aarch64-apple-ios-sim/release/libffi_interface.dylib"

#######################################
# 5. 拷贝 .dylib => frontend/ios/Vendored
#######################################
cd ../..  # 回到 alphaflow/
mkdir -p frontend/ios/Vendored
cp backend/ffi_interface/target/aarch64-apple-ios-sim/release/libffi_interface.dylib frontend/ios/Vendored/

#######################################
# 6. 创建 Flutter 工程
#######################################
printMsg "6. Creating Flutter project in 'frontend'..."
cd frontend
flutter create . --project-name alphaflow

#######################################
# 7. 创建 AlphaflowFFI.podspec (CocoaPods vendored_libraries)
#######################################
printMsg "7. Creating AlphaflowFFI podspec..."
cat <<EOF > ios/AlphaflowFFI.podspec
Pod::Spec.new do |s|
  s.name             = 'AlphaflowFFI'
  s.version          = '0.0.1'
  s.summary          = 'A Rust dynamic library for iOS'
  s.description      = <<-DESC
                       Embedded Rust .dylib for alphaflow via vendored_libraries.
                       DESC
  s.homepage         = 'https://example.com'
  s.license          = { :type => 'MIT' }
  s.author           = { 'You' => 'liuzhihao0206@gmail.com' }
  s.platform         = :ios, '14.0'
  s.source           = { :path => '.' }
  s.vendored_libraries = 'Vendored/libffi_interface.dylib'
  s.preserve_paths = 'Vendored/libffi_interface.dylib'
end
EOF

#######################################
# 8. 覆盖 Podfile => 强制 iOS 14.0 & CocoaPods config
#######################################
printMsg "8. Overwriting ios/Podfile (set platform to iOS 14.0)..."
cat <<EOF > ios/Podfile
platform :ios, '14.0'
ENV['COCOAPODS_DISABLE_STATS'] = 'true'

project 'Runner', {
  'Debug' => :debug,
  'Profile' => :release,
  'Release' => :release,
}

def flutter_root
  generated_xcode_build_settings_path = File.expand_path(File.join('..', 'Flutter', 'Generated.xcconfig'), __FILE__)
  unless File.exist?(generated_xcode_build_settings_path)
    raise "\#{generated_xcode_build_settings_path} must exist. If you're running pod install manually, make sure \"flutter pub get\" is executed first"
  end
  File.read(generated_xcode_build_settings_path) =~ /FLUTTER_ROOT\=(.*)/
  \$1
end

require File.expand_path(File.join(flutter_root, 'packages', 'flutter_tools', 'bin', 'podhelper'))

target 'Runner' do
  use_frameworks!
  use_modular_headers!

  flutter_install_all_ios_pods File.dirname(File.realpath(__FILE__))

  pod 'AlphaflowFFI', :path => '.'
end

post_install do |installer|
  installer.pods_project.targets.each do |target|
    target.build_configurations.each do |config|
      config.build_settings['IPHONEOS_DEPLOYMENT_TARGET'] = '14.0'
    end
  end
end
EOF

#######################################
# 9. Overwrite Info.plist => MinimumOSVersion=14.0
#######################################
printMsg "9. Overwriting ios/Runner/Info.plist (MinimumOSVersion=14.0)..."
PLIST_PATH="ios/Runner/Info.plist"
cat <<EOF > "\$PLIST_PATH"
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
 "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>Runner</string>
    <key>CFBundleIdentifier</key>
    <string>\$(PRODUCT_BUNDLE_IDENTIFIER)</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>Runner</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0</string>
    <key>CFBundleSignature</key>
    <string>????</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>LSRequiresIPhoneOS</key>
    <true/>
    <key>UILaunchStoryboardName</key>
    <string>LaunchScreen</string>
    <key>UIRequiredDeviceCapabilities</key>
    <array>
        <string>arm64</string>
    </array>
    <key>UISupportedInterfaceOrientations</key>
    <array>
        <string>UIInterfaceOrientationPortrait</string>
        <string>UIInterfaceOrientationLandscapeLeft</string>
        <string>UIInterfaceOrientationLandscapeRight</string>
    </array>
    <key>UISupportedInterfaceOrientations~ipad</key>
    <array>
        <string>UIInterfaceOrientationPortrait</string>
        <string>UIInterfaceOrientationPortraitUpsideDown</string>
        <string>UIInterfaceOrientationLandscapeLeft</string>
        <string>UIInterfaceOrientationLandscapeRight</string>
    </array>
    <key>MinimumOSVersion</key>
    <string>14.0</string>
</dict>
</plist>
EOF

#######################################
# 10. 运行 pod install (嵌入 .dylib)
#######################################
printMsg "10. Running pod install to embed .dylib..."
cd ios
rm -rf Pods Podfile.lock
pod install --verbose
cd ..

printMsg "✅ pod install finished."

#######################################
# 11. 修复 Xcode Base Configuration
#######################################
printMsg "11. Fixing Xcode project settings for CocoaPods..."
sed -i '' 's|Base Configuration .*|Base Configuration = Target Support Files/Pods-Runner/Pods-Runner.debug.xcconfig|g' ios/Runner.xcodeproj/project.pbxproj || true
sed -i '' 's|Base Configuration .*|Base Configuration = Target Support Files/Pods-Runner/Pods-Runner.release.xcconfig|g' ios/Runner.xcodeproj/project.pbxproj || true
sed -i '' 's|Base Configuration .*|Base Configuration = Target Support Files/Pods-Runner/Pods-Runner.profile.xcconfig|g' ios/Runner.xcodeproj/project.pbxproj || true
printMsg "✅ Fixed Base Configuration in Xcode project."

#######################################
# 12. 覆盖 Flutter FFI 代码 (Dart)
#######################################
printMsg "12. Overwriting lib/ffi_binding.dart + main.dart"
mkdir -p lib
cat <<EOF > lib/ffi_binding.dart
import 'dart:ffi';
import 'package:ffi/ffi.dart';

typedef CreateUserNative = Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>, Pointer<Utf8>);
typedef CreateUserDart = Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>, Pointer<Utf8>);

typedef GetUserNative = Pointer<Utf8> Function(Int32);
typedef GetUserDart = Pointer<Utf8> Function(int);

typedef FreeStringNative = Void Function(Pointer<Utf8>);
typedef FreeStringDart = void Function(Pointer<Utf8>);

class RustBinding {
  static final RustBinding _instance = RustBinding._internal();
  factory RustBinding() => _instance;
  RustBinding._internal();

  late final DynamicLibrary nativeLib;
  late final CreateUserDart createUser;
  late final GetUserDart getUserById;
  late final FreeStringDart freeString;

  bool _initialized = false;

  void init() {
    if (_initialized) return;
    // 从 CocoaPods 嵌入的路径加载 .dylib
    nativeLib = DynamicLibrary.open("@executable_path/Frameworks/libffi_interface.dylib");

    createUser = nativeLib
      .lookup<NativeFunction<CreateUserNative>>('create_user_ffi')
      .asFunction();
    getUserById = nativeLib
      .lookup<NativeFunction<GetUserNative>>('get_user_by_id_ffi')
      .asFunction();
    freeString = nativeLib
      .lookup<NativeFunction<FreeStringNative>>('free_string')
      .asFunction();

    _initialized = true;
  }

  String createUserFFI(String username, String email, String password) {
    final unamePtr = username.toNativeUtf8(allocator: calloc);
    final emailPtr = email.toNativeUtf8(allocator: calloc);
    final passPtr = password.toNativeUtf8(allocator: calloc);

    final resultPtr = createUser(unamePtr, emailPtr, passPtr);

    calloc.free(unamePtr);
    calloc.free(emailPtr);
    calloc.free(passPtr);

    if (resultPtr.address == 0) {
      return "Error: Rust returned null pointer";
    }
    final resultStr = resultPtr.toDartString();
    freeString(resultPtr); // 使用 Rust 暴露的 free_string 释放
    return resultStr;
  }

  String getUserFFI(int userId) {
    final resultPtr = getUserById(userId);
    if (resultPtr.address == 0) {
      return "Error: Rust returned null pointer";
    }
    final resultStr = resultPtr.toDartString();
    freeString(resultPtr); // 使用 Rust 暴露的 free_string 释放
    return resultStr;
  }
}
EOF

cat <<EOF > lib/main.dart
import 'package:flutter/material.dart';
import 'ffi_binding.dart';

void main() {
  WidgetsFlutterBinding.ensureInitialized();
  RustBinding().init();
  runApp(const MyApp());
}

class MyApp extends StatefulWidget {
  const MyApp({super.key});
  @override
  State<MyApp> createState() => _MyAppState();
}

class _MyAppState extends State<MyApp> {
  final _usernameController = TextEditingController();
  final _emailController = TextEditingController();
  final _passwordController = TextEditingController();
  String _result = "";

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: "Alphaflow FFI (CocoaPods vendored_libraries)",
      home: Scaffold(
        appBar: AppBar(title: const Text("Alphaflow FFI Demo")),
        body: Padding(
          padding: const EdgeInsets.all(24.0),
          child: Column(
            children: [
              TextField(controller: _usernameController, decoration: const InputDecoration(labelText: "Username")),
              TextField(controller: _emailController, decoration: const InputDecoration(labelText: "Email")),
              TextField(controller: _passwordController, decoration: const InputDecoration(labelText: "Password")),
              const SizedBox(height: 16),
              Row(
                children: [
                  ElevatedButton(
                    onPressed: () {
                      final name = _usernameController.text;
                      final email = _emailController.text;
                      final pass = _passwordController.text;
                      final json = RustBinding().createUserFFI(name, email, pass);
                      setState(() => _result = json);
                    },
                    child: const Text("Create User"),
                  ),
                  const SizedBox(width: 16),
                  ElevatedButton(
                    onPressed: () {
                      final json = RustBinding().getUserFFI(1);
                      setState(() => _result = json);
                    },
                    child: const Text("Get User #1"),
                  ),
                ],
              ),
              const SizedBox(height: 16),
              Expanded(child: SingleChildScrollView(child: Text(_result))),
            ],
          ),
        ),
      ),
    );
  }
}
EOF

printMsg "All done! Next steps:"
printSuccess "cd alphaflow/frontend"
printSuccess "flutter pub add ffi"
printSuccess "flutter clean && flutter pub get"
printSuccess "flutter run"
printSuccess "(We forced iOS 14.0; .dylib is embedded via vendored_libraries; Base Config fixed.)"
exit 0
