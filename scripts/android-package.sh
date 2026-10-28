#!/usr/bin/env bash
# The one home for building a package for a phone. §258.
#
# This file is **sourced**, not run: it holds the steps that the tree's two
# packages share — where the SDK is, which tools it has, how a library is
# built for a phone's ABI, and how a package is linked, staged, aligned and
# signed.
#
# The two packages are:
#
#   scripts/build-android.sh       the default one — no Java, no dex (§258)
#   scripts/build-android-ime.sh   the gated one — the platform's keyboard (§257)
#
# # Why the tree builds its own package
#
# `cargo apk` built the tree's default package, and it is a third-party tool
# the tree does not depend on (§236). Its manifest cannot say what the tree
# needs to say: a package that asks for **no permission at all** cannot open
# a socket on a phone, and the tree's own control channel is a socket (§258).
#
# The ABI table is **not** written here. It is the tree's gate's table
# (`xtask/src/android.rs`), and this file asks the gate for it — one list,
# one home.
#
# # What the platform's tools are called
#
# `aapt2` links a package, `zipalign` aligns one, `apksigner` signs one, and
# `d8` turns the Java into the dex the platform loads classes from. They are
# the SDK's own tools: nothing new is added to the tree.
set -euo pipefail

# The SDK, and the tools in it.
android_sdk_dir() {
  local sdk="${ANDROID_SDK_ROOT:-${ANDROID_HOME:-$HOME/Android/Sdk}}"
  echo "$sdk"
}

# The NDK. The tree's own gate's version (§241), and the newest one the SDK
# carries when the variable is not set.
android_ndk_dir() {
  local sdk
  sdk="$(android_sdk_dir)"
  if [[ -n "${ANDROID_NDK_HOME:-}" ]]; then
    echo "$ANDROID_NDK_HOME"
    return
  fi
  ls -d "$sdk"/ndk/* 2>/dev/null | sort -V | tail -1
}

# The platform jar the Java is compiled against.
#
# It is a **newer** platform than the package's own target, because the hook
# the gated package needs — a view that serves the input connection — arrived
# in Android 12. The package's `minSdkVersion` stays at 23: the method is
# simply never called on an older platform (§257).
android_platform_jar() {
  local sdk
  sdk="$(android_sdk_dir)"
  local jar
  jar="$(ls -d "$sdk"/platforms/android-*/android.jar 2>/dev/null | sort -V | tail -1)"
  echo "$jar"
}

# The build tools: the newest the SDK carries.
android_build_tools_dir() {
  local sdk
  sdk="$(android_sdk_dir)"
  ls -d "$sdk"/build-tools/* | sort -V | tail -1
}

# Fail when a tool is not there.
#
# A build that quietly skips a step is a build that is not the package it
# claims to be.
android_need_tools() {
  local tools
  tools="$(android_build_tools_dir)"
  for tool in "$@"; do
    if [[ ! -x "$tools/$tool" ]]; then
      echo "the build tools have no $tool: $tools" >&2
      exit 1
    fi
  done
}

# The tree's gate's ABI table: a triple and the compiler for it, one line
# each.
#
# The compiler's name is **not** the target's name: the 32-bit target is
# `armv7-linux-androideabi` and the toolchain's compiler for it is
# `armv7a-linux-androideabi21-clang`. That is why the table has two columns,
# and it is the gate's table, not this file's.
android_abi_table() {
  cargo xtask android --abis
}

# The ABI directory the platform's loader looks in.
android_abi_dir() {
  case "$1" in
    aarch64-linux-android) echo "arm64-v8a" ;;
    armv7-linux-androideabi) echo "armeabi-v7a" ;;
    *)
      echo "the tree builds for a phone's two ABIs, not for $1" >&2
      exit 1
      ;;
  esac
}

# The environment a build for one ABI needs: the NDK's compiler is the
# linker, and the C compiler and the archiver are named for the target,
# because `cc` and `ar` on the path are the host's.
android_cargo_env() {
  local triple="$1"
  local clang
  clang="$(android_abi_table | awk -v t="$triple" '$1 == t { print $2 }')"
  if [[ -z "$clang" ]]; then
    echo "the tree's gate does not know the ABI $triple" >&2
    exit 1
  fi
  local ndk
  ndk="$(android_ndk_dir)"
  local upper
  upper="$(echo "$triple" | tr '[:lower:]-' '[:upper:]_')"
  export CARGO_TARGET_"$upper"_LINKER="$clang"
  export CC_"$(echo "$triple" | tr '-' '_')"="$clang"
  export AR_"$(echo "$triple" | tr '-' '_')"="llvm-ar"
  export ANDROID_NDK_HOME="$ndk"
  export PATH="$ndk/toolchains/llvm/prebuilt/linux-x86_64/bin:$PATH"
}

# Build the library for one ABI.
#
# The extra arguments are the gate's: `--features platform-ime` for the
# gated package, and nothing for the default one.
android_build_library() {
  local triple="$1"
  shift
  android_cargo_env "$triple"
  cargo build -p craftkit-android --target "$triple" "$@"
}

# Link a package: the manifest, the resources, and nothing else.
#
# The library is **not** given to `aapt2`: it wants a resource it can parse,
# and a shared library is not one. So the package is linked without it and
# the library is added afterwards, under the path the platform's loader looks
# in — `lib/<abi>/` (§236).
#
# The resources are optional: the default package has none.
android_link() {
  local manifest="$1"
  local out="$2"
  local version_code="$3"
  local resources="${4:-}"
  local tools platform
  tools="$(android_build_tools_dir)"
  platform="$(android_platform_jar)"
  if [[ -n "$resources" ]]; then
    "$tools/aapt2" link -I "$platform" --manifest "$manifest" -R "$resources" \
      --min-sdk-version 23 --target-sdk-version 30 \
      --version-code "$version_code" --version-name 0.1.0 \
      -o "$out"
  else
    "$tools/aapt2" link -I "$platform" --manifest "$manifest" \
      --min-sdk-version 23 --target-sdk-version 30 \
      --version-code "$version_code" --version-name 0.1.0 \
      -o "$out"
  fi
}

# Align a package and sign it, with the debug key the phone accepts.
android_sign() {
  local unsigned="$1"
  local out="$2"
  local tools
  tools="$(android_build_tools_dir)"
  "$tools/zipalign" -f 4 "$unsigned" "$out.aligned"
  "$tools/apksigner" sign \
    --ks "$HOME/.android/debug.keystore" \
    --ks-pass pass:android --key-pass pass:android \
    --out "$out" "$out.aligned"
}
