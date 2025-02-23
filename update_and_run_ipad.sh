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
    echo -e "${YELLOW}alphaflow-update : $1${ENDCOLOR}"
}

function printSuccess() {
    echo -e "${GREEN}alphaflow-update : $1${ENDCOLOR}"
}

#######################################
# 定义一些路径变量 (供后续命令使用)
#######################################
ROOT_DIR="$(pwd)/alphaflow"  # 修改为你的实际工程根路径
BACKEND_DIR="$ROOT_DIR/backend"
FFI_INTERFACE_DIR="$BACKEND_DIR/ffi_interface"
FRONTEND_DIR="$ROOT_DIR/frontend"
DYLIB_NAME="libffi_interface.dylib"

# 改为真机目标 (arm64)
TARGET_TRIPLE="aarch64-apple-ios"

# 构建后 .dylib 路径
DYLIB_BUILD_PATH="$FFI_INTERFACE_DIR/target/$TARGET_TRIPLE/release/$DYLIB_NAME"

# 存放在 iOS 工程 vendored_libraries 的目录
VENDORED_DIR="$FRONTEND_DIR/ios/Vendored"


#######################################
# 1. 进入 ffi_interface 目录, 重新编译 (for iOS device)
#######################################
printMsg "1. Rebuilding ffi_interface for iOS Device ($TARGET_TRIPLE)..."
cd "$FFI_INTERFACE_DIR"

# 如果没加过这个 target, 需要先加
rustup target add $TARGET_TRIPLE >/dev/null 2>&1 || true

# 执行 release 编译 (arm64-apple-ios)
cargo build --release --target $TARGET_TRIPLE

printSuccess "Build success => $DYLIB_BUILD_PATH"

#######################################
# 2. 拷贝 .dylib => frontend/ios/Vendored
#######################################
printMsg "2. Copying $DYLIB_NAME to $VENDORED_DIR"
mkdir -p "$VENDORED_DIR"
cp "$DYLIB_BUILD_PATH" "$VENDORED_DIR/"

#######################################
# 3. (可选) 重新运行 pod install
#######################################
cd "$FRONTEND_DIR/ios"
if [ -f "Podfile" ]; then
    printMsg "3. Updating CocoaPods (pod install)..."
    pod install
    printSuccess "Pod install finished."
else
    printMsg "3. Podfile not found, skipping pod install..."
fi

#######################################
# 4. Flutter端清理 & 重新依赖
#######################################
cd "$FRONTEND_DIR"
printMsg "4. flutter clean & flutter pub get"
flutter clean
flutter pub get

#######################################
# 5. 运行到真机 (注意: 需连接真机 & 有签名配置)
#######################################
printMsg "5. Attempting flutter run on iOS Device"
# flutter run (可视情况带 -d device_id)
flutter run

printSuccess "Done. If everything is OK, the app should launch on iOS device (arm64)!"