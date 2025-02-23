#!/usr/bin/env bash
set -e

#######################################
# 彩色打印函数
#######################################
YELLOW="\e[93m"
GREEN="\e[32m"
RED="\e[31m"
ENDCOLOR="\e[0m"

function printMsg() {
    echo -e "${YELLOW}alphaflow-staticlib : $1${ENDCOLOR}"
}

function printSuccess() {
    echo -e "${GREEN}alphaflow-staticlib : $1${ENDCOLOR}"
}

function printError() {
    echo -e "${RED}alphaflow-staticlib : $1${ENDCOLOR}"
    exit 1
}

#######################################
# 配置路径 (需按你实际目录修改)
#######################################
ROOT_DIR="$(pwd)/alphaflow"  # 工程顶层
BACKEND_DIR="$ROOT_DIR/backend"
FFI_INTERFACE_DIR="$BACKEND_DIR/ffi_interface"
FRONTEND_DIR="$ROOT_DIR/frontend"

# Rust crate name (在 Cargo.toml [package] name="ffi_interface")
# crate-type=["staticlib"] => 产物名可能是 libffi_interface.a
LIB_NAME="ffi_interface"

# iOS device (arm64)
TARGET_TRIPLE="aarch64-apple-ios"

# 产物文件名 => libffi_interface.a
STATICLIB_NAME="lib${LIB_NAME}.a"

# build 输出路径
STATICLIB_BUILD_PATH="$FFI_INTERFACE_DIR/target/$TARGET_TRIPLE/release/$STATICLIB_NAME"

# iOS vendored 路径
VENDORED_DIR="$FRONTEND_DIR/ios/Vendored"

# 要运行的真机 device id (可换成你设备ID或 environment variable)
DEVICE_ID="00008130-0006092900E1401C"

#######################################
# 0) 可选：删除旧 Pods (若你怀疑旧引用 .dylib)
#######################################
# printMsg "0) [Optional] Removing old Pods, re-install..."
# cd "$FRONTEND_DIR/ios"
# rm -rf Pods Podfile.lock
# pod install
# cd "$FRONTEND_DIR"

#######################################
# 1) 清理旧构建
#######################################
printMsg "1) cargo clean (removing old Rust build artifacts)"
cd "$FFI_INTERFACE_DIR"
cargo clean

#######################################
# 2) 删除 iOS/Vendored 下的旧 .dylib (避免误用)
#######################################
printMsg "2) Removing any leftover .dylib from $VENDORED_DIR"
mkdir -p "$VENDORED_DIR"
rm -f "$VENDORED_DIR"/*.dylib

#######################################
# 3) 重编译 rust => staticlib .a
#######################################
printMsg "3) Building $LIB_NAME as staticlib for iOS device ($TARGET_TRIPLE)..."
rustup target add $TARGET_TRIPLE >/dev/null 2>&1 || true

# 正常用 cargo build
cargo build --release --target $TARGET_TRIPLE
printSuccess "Built => $STATICLIB_BUILD_PATH"

if [ ! -f "$STATICLIB_BUILD_PATH" ]; then
    printError "Static library not found at $STATICLIB_BUILD_PATH"
fi

#######################################
# 4) 拷贝到 ios/Vendored
#######################################
printMsg "4) Copying $STATICLIB_NAME to $VENDORED_DIR"
cp "$STATICLIB_BUILD_PATH" "$VENDORED_DIR/"
printSuccess "Copied => $VENDORED_DIR/$STATICLIB_NAME"

#######################################
# 5) (可选) pod install
#######################################
cd "$FRONTEND_DIR/ios"
if [ -f "Podfile" ]; then
    printMsg "5) Running pod install"
    pod install
    printSuccess "Pod install finished"
else
    printMsg "No Podfile found, skip."
fi

#######################################
# 6) flutter clean & pub get
#######################################
cd "$FRONTEND_DIR"
printMsg "6) flutter clean & flutter pub get"
flutter clean
flutter pub get

#######################################
# 7) flutter run -d <device>
#######################################
printMsg "7) Running on iOS device: $DEVICE_ID"
flutter run -d "$DEVICE_ID"

printSuccess "Done! If everything is correct, static library is linked and app should run, no more .dylib issues."