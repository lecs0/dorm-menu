#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
: "${ANDROID_NDK_HOME:?Set ANDROID_NDK_HOME to the installed NDK directory}"
# Requires cargo-ndk. arm64 targets modern phones; add armeabi-v7a if needed.
cargo ndk -t arm64-v8a --platform 26 -o app/android/app/src/main/jniLibs build --locked --release -p dorm-app
cd app/android
gradle assembleDebug
