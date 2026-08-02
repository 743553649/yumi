#!/bin/bash
set -e

APP_DIR="/storage/emulated/0/yumi/android-app"
BUILD_DIR="$APP_DIR/build"
ANDROID_JAR="$HOME/.android-sdk/platforms/android-34/android.jar"
KEYTOOL="$PREFIX/lib/jvm/java-21-openjdk/bin/keytool"
JARSIGNER="$PREFIX/lib/jvm/java-21-openjdk/bin/jarsigner"

rm -rf "$BUILD_DIR"
mkdir -p "$BUILD_DIR/gen" "$BUILD_DIR/classes" "$BUILD_DIR/dex"

echo "[1/6] 编译 Android 资源 (aapt2 compile)..."
aapt2 compile --dir "$APP_DIR/app/src/main/res" -o "$BUILD_DIR/res.zip"

echo "[2/6] 链接资源并生成 R.java (aapt2 link)..."
aapt2 link -I "$ANDROID_JAR" \
  --manifest "$APP_DIR/app/src/main/AndroidManifest.xml" \
  --min-sdk-version 26 \
  --target-sdk-version 34 \
  -o "$BUILD_DIR/unaligned.apk" \
  --java "$BUILD_DIR/gen" \
  "$BUILD_DIR/res.zip"

echo "[3/6] 编译 Java 源码 (javac)..."
javac -source 8 -target 8 -bootclasspath "$ANDROID_JAR" \
  -d "$BUILD_DIR/classes" \
  "$BUILD_DIR/gen/com/yumi/bridge/R.java" \
  $(find "$APP_DIR/app/src/main/java" -name "*.java")

echo "[4/6] 转换 Bytecode 为 DEX (d8)..."
d8 --lib "$ANDROID_JAR" \
  --min-api 26 \
  --output "$BUILD_DIR/dex" \
  $(find "$BUILD_DIR/classes" -name "*.class")

echo "[5/6] 打包 APK 文件..."
cp "$BUILD_DIR/unaligned.apk" "$BUILD_DIR/yumi-bridge.apk"
cd "$BUILD_DIR/dex"
zip -u "$BUILD_DIR/yumi-bridge.apk" classes.dex
cd "$APP_DIR"

echo "[6/6] 签名 APK (keytool + jarsigner)..."
KEYSTORE="$BUILD_DIR/debug.keystore"
if [ ! -f "$KEYSTORE" ]; then
  "$KEYTOOL" -genkey -v -keystore "$KEYSTORE" -storepass android -alias androiddebugkey -keypass android -keyalg RSA -keysize 2048 -validity 10000 -dname "CN=Android Debug,O=Android,C=US" >/dev/null 2>&1
fi

"$JARSIGNER" -keystore "$KEYSTORE" -storepass android -keypass android "$BUILD_DIR/yumi-bridge.apk" androiddebugkey >/dev/null

# 同步复制到项目根目录及 module 目录
cp "$BUILD_DIR/yumi-bridge.apk" "/storage/emulated/0/yumi/yumi-bridge.apk"
cp "$BUILD_DIR/yumi-bridge.apk" "/storage/emulated/0/yumi/module/yumi-bridge.apk"

echo "✅ APK 打包完成！绝对路径："
echo "1. /storage/emulated/0/yumi/yumi-bridge.apk"
echo "2. /storage/emulated/0/yumi/module/yumi-bridge.apk"
echo "3. /storage/emulated/0/yumi/android-app/build/yumi-bridge.apk"
