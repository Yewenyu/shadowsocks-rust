#!/bin/bash
set -euo pipefail

# -------------------------- 配置区域 --------------------------
# 1. 请确认你的库名称 (通常与 Cargo.toml 中的 name 一致)
# 读取 Cargo.toml 中 [package] 下的 name 字段，自动处理空格
LIB_NAME=$(awk -F ' *= *' '
  /^\[package\]/ { in_package=1; next }
  in_package && /^name/ { gsub(/[" ]/, "", $2); print $2; exit }
' Cargo.toml)

# 2. 目标文件夹 (完整保留)
CARGO_TARGET_DIR=./target

# 3. cbindgen 配置文件路径 (可选，如果没有则不使用 --config)
CBINDGEN_CONFIG="./cbindgen.toml"

# -------------------------- 逻辑区域 --------------------------

if [ -z "$LIB_NAME" ]; then
  echo "❌ 错误：无法在 Cargo.toml 中找到库名称，请手动修改脚本中的 LIB_NAME。"
  exit 1
fi

# 检查 cbindgen 是否已安装
if ! command -v cbindgen &> /dev/null; then
    echo "❌ 错误：未找到 cbindgen。"
    echo "   请运行以下命令安装：cargo install cbindgen"
    exit 1
fi

# 检查 cargo lipo 是否已安装 (用于 iOS 通用库构建)
if ! command -v cargo-lipo &> /dev/null; then
    echo "❌ 错误：未找到 cargo-lipo。"
    echo "   请运行以下命令安装：cargo install cargo-lipo"
    exit 1
fi

# 静态库文件名
STATIC_LIB="lib${LIB_NAME}.a"
# 最终输出的 XCFramework 名称
XCFW_NAME="${LIB_NAME}.xcframework"
# 生成的头文件名
HEADER_FILE="${LIB_NAME}.h"

# macOS 目标平台定义
MACOS_AARCH64_TARGET="aarch64-apple-darwin"
MACOS_X86_64_TARGET="x86_64-apple-darwin"

echo "🚀 开始构建 ${LIB_NAME} (iOS + macOS)..."

# 1. 清理旧的构建产物
echo "🧹 清理旧文件... $XCFW_NAME, $HEADER_FILE"
cargo clean
rm -rf "$XCFW_NAME"
rm -f "$HEADER_FILE"

# 2. 使用 cbindgen 生成头文件
echo "📝 使用 cbindgen 生成头文件 ${HEADER_FILE}..."
if [ -f "$CBINDGEN_CONFIG" ]; then
  cbindgen --config "$CBINDGEN_CONFIG" --output "$HEADER_FILE"
else
  echo "   提示：未找到 cbindgen.toml，使用默认配置生成。"
  cbindgen --output "$HEADER_FILE"
fi

if [ ! -f "$HEADER_FILE" ]; then
  echo "❌ 错误：cbindgen 未能生成 ${HEADER_FILE}"
  exit 1
fi

# 3. 构建 iOS 通用库
echo "📦 执行 cargo lipo --release (iOS)..."
CARGO_TARGET_DIR=${CARGO_TARGET_DIR} cargo lipo --release

# 4. 构建 macOS 两个架构
echo "📦 构建 macOS (Apple Silicon arm64)..."
CARGO_TARGET_DIR=${CARGO_TARGET_DIR} cargo build --release --target $MACOS_AARCH64_TARGET

echo "📦 构建 macOS (Intel x86_64)..."
CARGO_TARGET_DIR=${CARGO_TARGET_DIR} cargo build --release --target $MACOS_X86_64_TARGET

# 5. 定义所有路径
# iOS 路径
UNIVERSAL_PATH="${CARGO_TARGET_DIR}/universal/release/${STATIC_LIB}"
# macOS 路径
MACOS_AARCH64_LIB="${CARGO_TARGET_DIR}/${MACOS_AARCH64_TARGET}/release/${STATIC_LIB}"
MACOS_X86_64_LIB="${CARGO_TARGET_DIR}/${MACOS_X86_64_TARGET}/release/${STATIC_LIB}"

# 检查文件存在性
if [ ! -f "$UNIVERSAL_PATH" ]; then
  echo "❌ 错误：未找到 iOS 构建产物 ${UNIVERSAL_PATH}"
  exit 1
fi
if [ ! -f "$MACOS_AARCH64_LIB" ]; then
  echo "❌ 错误：未找到 macOS Apple Silicon 构建产物 ${MACOS_AARCH64_LIB}"
  exit 1
fi
if [ ! -f "$MACOS_X86_64_LIB" ]; then
  echo "❌ 错误：未找到 macOS Intel 构建产物 ${MACOS_X86_64_LIB}"
  exit 1
fi

# 6. 创建临时目录
TEMP_DIR=$(mktemp -d)
# iOS 目录
DEVICE_DIR="${TEMP_DIR}/device"
SIM_DIR="${TEMP_DIR}/simulator"
DEVICE_HEADERS_DIR="${DEVICE_DIR}/Headers"
SIM_HEADERS_DIR="${SIM_DIR}/Headers"
# macOS 目录
MACOS_DIR="${TEMP_DIR}/macos"
MACOS_HEADERS_DIR="${MACOS_DIR}/Headers"

mkdir -p "$DEVICE_DIR" "$SIM_DIR" "$DEVICE_HEADERS_DIR" "$SIM_HEADERS_DIR"
mkdir -p "$MACOS_DIR" "$MACOS_HEADERS_DIR"

echo "🔧 处理架构拆分与头文件..."

# --- 处理 iOS ---
cp "$UNIVERSAL_PATH" "$DEVICE_DIR/"
cp "$UNIVERSAL_PATH" "$SIM_DIR/"
cp "$HEADER_FILE" "$DEVICE_HEADERS_DIR/"
cp "$HEADER_FILE" "$SIM_HEADERS_DIR/"

# 生成 modulemap
MODULEMAP_PATH_DEVICE="${DEVICE_HEADERS_DIR}/module.modulemap"
cat > "$MODULEMAP_PATH_DEVICE" <<EOF
module ${LIB_NAME} {
    umbrella header "${LIB_NAME}.h"
    export *
}
EOF
cp "$MODULEMAP_PATH_DEVICE" "$SIM_HEADERS_DIR/module.modulemap"

# 拆分架构
lipo "$DEVICE_DIR/$STATIC_LIB" -thin arm64 -output "$DEVICE_DIR/$STATIC_LIB"
lipo "$SIM_DIR/$STATIC_LIB" -remove arm64 -output "$SIM_DIR/$STATIC_LIB"

# --- 处理 macOS ---
echo "🔗 合并 macOS 架构..."
lipo -create "$MACOS_AARCH64_LIB" "$MACOS_X86_64_LIB" -output "$MACOS_DIR/$STATIC_LIB"

# 复制头文件和 modulemap
cp "$HEADER_FILE" "$MACOS_HEADERS_DIR/"
cp "$MODULEMAP_PATH_DEVICE" "$MACOS_HEADERS_DIR/module.modulemap"

echo "🖇️ 生成 XCFramework (iOS + macOS)..."

# 7. 生成包含 iOS (Device/Sim) 和 macOS 的 XCFramework
xcodebuild -create-xcframework \
  -library "$DEVICE_DIR/$STATIC_LIB" -headers "$DEVICE_HEADERS_DIR" \
  -library "$SIM_DIR/$STATIC_LIB" -headers "$SIM_HEADERS_DIR" \
  -library "$MACOS_DIR/$STATIC_LIB" -headers "$MACOS_HEADERS_DIR" \
  -output "$XCFW_NAME"

# 8. 清理
rm -rf "$TEMP_DIR"

echo ""
echo "✅ 构建成功！"
echo "📂 XCFramework 位置: $(pwd)/${XCFW_NAME}"
open "$(pwd)"