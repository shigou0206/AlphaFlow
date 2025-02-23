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

function printError() {
    echo -e "${RED}alphaflow-update : $1${ENDCOLOR}"
    exit 1
}

#######################################
# 定义一些路径变量 (供后续命令使用)
#######################################
ROOT_DIR="$(pwd)/alphaflow"  # 修改为你实际的工程路径
BACKEND_DIR="$ROOT_DIR/backend"
FFI_INTERFACE_DIR="$BACKEND_DIR/ffi_interface"
FRONTEND_DIR="$ROOT_DIR/frontend"
DYLIB_NAME="libffi_interface.dylib"

# 指定真机的目标三元组 (arm64)
TARGET_TRIPLE="aarch64-apple-ios"

# 构建后 .dylib 路径
DYLIB_BUILD_PATH="$FFI_INTERFACE_DIR/target/$TARGET_TRIPLE/release/$DYLIB_NAME"

# 放置在 iOS 工程 vendored_libraries 的目录
VENDORED_DIR="$FRONTEND_DIR/ios/Vendored"

#######################################
# 函数: 修正 .dylib 的 install_name = @rpath/libffi_interface.dylib
#######################################
function fixDylibInstallName() {
    if [ -f "$1" ]; then
        printMsg "Fixing install_name for $1"
        # 先查看当前ID
        CURRENT_ID=$(otool -D "$1" | tail -n 1)
        printMsg "Current install_name: $CURRENT_ID"
        if [[ "$CURRENT_ID" != "@rpath/$DYLIB_NAME" ]]; then
            install_name_tool -id "@rpath/$DYLIB_NAME" "$1"
            printSuccess "Updated install_name to @rpath/$DYLIB_NAME"
        else
            printMsg "install_name already @rpath/$DYLIB_NAME, no change."
        fi
    else
        printError "Dylib not found at $1"
    fi
}

#######################################
# 函数: 修正 Runner.debug.dylib 中依赖 /Users/xxx/... => @rpath/libffi_interface.dylib
#######################################
function fixRunnerDebugDylib() {
    local runner_debug="$1/Runner.debug.dylib"
    if [ -f "$runner_debug" ]; then
        # 查询其依赖
        local lines=$(otool -L "$runner_debug" | grep "$DYLIB_NAME")
        if echo "$lines" | grep -q "/Users/"; then
            # Extract old path
            local old_path=$(echo "$lines" | awk '{print $1}')
            printMsg "Replacing $old_path => @rpath/$DYLIB_NAME in Runner.debug.dylib"
            install_name_tool -change "$old_path" "@rpath/$DYLIB_NAME" "$runner_debug"
            printSuccess "Runner.debug.dylib now references @rpath/$DYLIB_NAME"
        else
            printMsg "Runner.debug.dylib does not contain absolute /Users/... reference. No change."
        fi
    else
        printMsg "No Runner.debug.dylib found in $1"
    fi
}

#######################################
# 1. 进入 ffi_interface 目录, 重新编译 (for iOS device)
#######################################
printMsg "1. Rebuilding ffi_interface for iOS device ($TARGET_TRIPLE)..."
cd "$FFI_INTERFACE_DIR"

# 如果没加过这个 target, 需要加一次
rustup target add $TARGET_TRIPLE >/dev/null 2>&1 || true

# 执行 release 编译 (arm64 for iOS device)
cargo build --release --target $TARGET_TRIPLE
printSuccess "Build success => $DYLIB_BUILD_PATH"

#######################################
# 2. 拷贝 .dylib => frontend/ios/Vendored & 修正install_name
#######################################
printMsg "2. Copying $DYLIB_NAME to $VENDORED_DIR"
mkdir -p "$VENDORED_DIR"
cp "$DYLIB_BUILD_PATH" "$VENDORED_DIR/"
fixDylibInstallName "$VENDORED_DIR/$DYLIB_NAME"

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
# 5. 运行到真机
#######################################
printMsg "5. Building & Running on iOS device"
flutter build ios --debug

# 进入 build产物 Runner.app, 修正 Runner.debug.dylib
APP_PATH="$FRONTEND_DIR/build/ios/Debug-iphoneos/Runner.app"
if [ -d "$APP_PATH" ]; then
    fixRunnerDebugDylib "$APP_PATH"
else
    printMsg "No Runner.app at $APP_PATH. Possibly build path differs or build failed."
fi

printMsg "Now launching flutter run on device..."
flutter run -d device

printSuccess "Done. If everything is OK, the app should launch on iOS device (arm64)!"