#!/usr/bin/env bash
# The **gated** package: the one with the platform's own keyboard. §257.
#
# The tree's default package is built by `cargo apk`, and it carries no Java
# and no dex: the platform's `NativeActivity` serves an input connection that
# gives the commit to nobody, so the tree draws a keyboard of its own (§246).
#
# The gate turns on the platform's path, and the path needs three things the
# default package does not have:
#
#   1. the two Java files, compiled against the platform's own jar;
#   2. a dex, because the platform loads classes from one;
#   3. a manifest that names the Java activity and says the package has code.
#
# So this script does what `cargo apk` cannot: it compiles the Java, dexes it,
# and links a package with `aapt2` — the same tools the SDK ships, and no
# new dependency in the tree.
#
#   scripts/build-android-ime.sh [target-triple]
#
# The default target is `aarch64-linux-android`. The result is
# `target/<triple>/craftkit-ime.apk`.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

TRIPLE="${1:-aarch64-linux-android}"

# The SDK, and the tools in it. The NDK is the tree's own gate's version
# (§241), and the build tools are the newest the SDK carries.
: "${ANDROID_HOME:=$HOME/Android/Sdk}"
: "${ANDROID_SDK_ROOT:=$ANDROID_HOME}"
SDK="${ANDROID_SDK_ROOT}"
NDK="${ANDROID_NDK_HOME:-$SDK/ndk/25.2.9519653}"
# The platform the Java is compiled against.
#
# It is a **newer** platform than the package's own target, because the hook
# the tree needs — the activity's own input connection — arrived in Android
# 12. The package's `minSdkVersion` stays at 23: the method is simply never
# called on an older platform, and the tree's default package (which carries
# no Java at all) is what an older phone runs.
PLATFORM="$SDK/platforms/android-34/android.jar"
BUILD_TOOLS="$(ls -d "$SDK"/build-tools/* | sort -V | tail -1)"

if [[ ! -f "$PLATFORM" ]]; then
  echo "the SDK has no platform jar at $PLATFORM" >&2
  exit 1
fi
for tool in d8 aapt2 zipalign apksigner; do
  if [[ ! -x "$BUILD_TOOLS/$tool" ]]; then
    echo "the build tools have no $tool: $BUILD_TOOLS" >&2
    exit 1
  fi
done
# `javac` is the host's, not the SDK's: the SDK's build tools have no Java
# compiler, and the tree does not ship one.
if ! command -v javac >/dev/null; then
  echo "javac is not on the PATH" >&2
  exit 1
fi

WORK="target/$TRIPLE/ime"
CLASSES="$WORK/classes"
DEX="$WORK/dex"
OUT="target/$TRIPLE/craftkit-ime.apk"

rm -rf "$WORK"
mkdir -p "$CLASSES" "$DEX"

# 1. The library, with the gate on.
export ANDROID_NDK_HOME="$NDK"
export PATH="$NDK/toolchains/llvm/prebuilt/linux-x86_64/bin:$PATH"
case "$TRIPLE" in
  aarch64-linux-android)
    export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER=aarch64-linux-android21-clang
    export CC_aarch64_linux_android=aarch64-linux-android21-clang
    export AR_aarch64_linux_android=llvm-ar
    ;;
  armv7-linux-androideabi)
    export CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER=armv7a-linux-androideabi21-clang
    export CC_armv7_linux_androideabi=armv7a-linux-androideabi21-clang
    export AR_armv7_linux_androideabi=llvm-ar
    ;;
  *)
    echo "the gate builds for a phone's two ABIs, not for $TRIPLE" >&2
    exit 1
    ;;
esac
cargo build -p craftkit-android --target "$TRIPLE" --features platform-ime

# 2. The Java, against the platform's own jar.
javac -Xlint:-options -source 8 -target 8 \
  -bootclasspath "$PLATFORM" -classpath "$PLATFORM" \
  -d "$CLASSES" \
  apps/craftkit-android/java/org/craftocto/craftkit/*.java

# 3. The dex.
"$BUILD_TOOLS/d8" --lib "$PLATFORM" --output "$DEX" "$CLASSES"/org/craftocto/craftkit/*.class

# 4. The resources.
#
# The platform's own description of an input method is a resource, and
# `aapt2 link` takes **compiled** resources: the sources are compiled first
# (§257).
"$BUILD_TOOLS/aapt2" compile --dir apps/craftkit-android/res -o "$WORK/res.zip"

# 5. The package: the manifest, the resources and the dex.
#
# The library is **not** given to `aapt2`: it wants a resource it can parse,
# and a shared library is not one. So the package is linked without it and
# the library is added afterwards, under the path the platform's loader looks
# in — `lib/<abi>/`, which is where the platform's loader looks for it, by
# name (§236).
"$BUILD_TOOLS/aapt2" link \
  -I "$PLATFORM" \
  --manifest apps/craftkit-android/AndroidManifest.ime.xml \
  -R "$WORK/res.zip" \
  --min-sdk-version 23 --target-sdk-version 30 \
  --version-code 16777473 --version-name 0.1.0 \
  -o "$WORK/unsigned.apk"

ABI="arm64-v8a"
if [[ "$TRIPLE" == "armv7-linux-androideabi" ]]; then
  ABI="armeabi-v7a"
fi
mkdir -p "$WORK/stage/lib/$ABI"
cp "target/$TRIPLE/debug/libcraftkit_android.so" "$WORK/stage/lib/$ABI/libcraftkit_android.so"
cp "$DEX/classes.dex" "$WORK/stage/classes.dex"
(cd "$WORK/stage" && zip -X -q "$ROOT/$WORK/unsigned.apk" "classes.dex" "lib/$ABI/libcraftkit_android.so")

# 6. Aligned, and signed with the same key the default package is signed
#    with, so the phone accepts it over the default one.
"$BUILD_TOOLS/zipalign" -f 4 "$WORK/unsigned.apk" "$WORK/aligned.apk"
"$BUILD_TOOLS/apksigner" sign \
  --ks "$HOME/.android/debug.keystore" \
  --ks-pass pass:android --key-pass pass:android \
  --out "$OUT" "$WORK/aligned.apk"

echo "the gated package is $OUT"
ls -l "$OUT"
