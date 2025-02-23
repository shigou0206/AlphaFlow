#!/usr/bin/env bash
set -e  # 遇到错误则退出

# 此脚本依赖 entr 工具，请确保已安装：brew install entr

#######################################
# 彩色打印函数
#######################################
YELLOW="\e[93m"
GREEN="\e[32m"
RED="\e[31m"
ENDCOLOR="\e[0m"

function printMsg() {
    echo -e "${YELLOW}$1${ENDCOLOR}"
}

function printSuccess() {
    echo -e "${GREEN}$1${ENDCOLOR}"
}

#######################################
# 定义自动构建和测试过程（内嵌命令）
#######################################
function auto_build_and_test() {
  printMsg ">>> Building Rust ffi_interface for iOS simulator (arm64)..."
  cd backend/ffi_interface
  cargo build --release --target aarch64-apple-ios-sim
  cd ../..
  
  printMsg ">>> Copying .dylib to frontend/ios/Vendored..."
  mkdir -p frontend/ios/Vendored
  cp backend/ffi_interface/target/aarch64-apple-ios-sim/release/libffi_interface.dylib frontend/ios/Vendored/
  
  printMsg ">>> Running pod install in frontend/ios..."
  cd frontend/ios
  rm -rf Pods Podfile.lock
  pod install --verbose
  cd ../..
  
  printMsg ">>> Running Flutter integration tests..."
  cd frontend
  flutter drive --target=test_driver/app.dart
  cd ..
  
  printSuccess ">>> Auto build & test completed."
}

#######################################
# 初次执行一次构建和测试
#######################################
auto_build_and_test

#######################################
# 使用 entr 监控后端和 Flutter 代码的变化，自动重跑
#######################################
printMsg ">>> Monitoring changes in backend/ffi_interface/src and frontend/lib..."
find backend/ffi_interface/src frontend/lib -type f | entr -r bash -c 'auto_build_and_test'
