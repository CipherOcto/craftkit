#!/usr/bin/env bash
# The **gated** package: the one with the platform's own keyboard. §257.
#
#   scripts/build-android-ime.sh [target-triple]
#
# The default target is `aarch64-linux-android`. The result is
# `target/<triple>/craftkit-ime.apk`.
#
# The gate turns on the platform's path, and the path needs three things the
# default package does not have:
#
#   1. the Java files, compiled against the platform's own jar;
#   2. a dex, because the platform loads classes from one;
#   3. a manifest that names the Java activity and says the package has code.
#
# The steps this script shares with the default package live in
# `scripts/android-package.sh`, which is the one home for them (§258).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# shellcheck source=scripts/android-package.sh
source "$ROOT/scripts/android-package.sh"

TRIPLE="${1:-aarch64-linux-android}"

WORK="target/$TRIPLE/ime"
CLASSES="$WORK/classes"
DEX="$WORK/dex"
OUT="target/$TRIPLE/craftkit-ime.apk"

# The tools, and the Java compiler.
#
# `javac` is the host's, not the SDK's: the SDK's build tools have no Java
# compiler, and the tree does not ship one.
android_need_tools d8 aapt2 zipalign apksigner
if ! command -v javac >/dev/null; then
  echo "javac is not on the PATH" >&2
  exit 1
fi

rm -rf "$WORK"
mkdir -p "$CLASSES" "$DEX"

# 1. The library, with the gate on.
android_build_library "$TRIPLE" --features platform-ime

# 2. The Java, against the platform's own jar.
PLATFORM="$(android_platform_jar)"
javac -Xlint:-options -source 8 -target 8 \
  -bootclasspath "$PLATFORM" -classpath "$PLATFORM" \
  -d "$CLASSES" \
  apps/craftkit-android/java/org/craftocto/craftkit/*.java

# 3. The dex.
BUILD_TOOLS="$(android_build_tools_dir)"
"$BUILD_TOOLS/d8" --lib "$PLATFORM" --output "$DEX" "$CLASSES"/org/craftocto/craftkit/*.class

# 4. The resources.
#
# The platform's own description of an input method is a resource, and
# `aapt2 link` takes **compiled** resources: the sources are compiled first
# (§257).
"$BUILD_TOOLS/aapt2" compile --dir apps/craftkit-android/res -o "$WORK/res.zip"

# 5. The package: the manifest and the resources.
#
# The library is **not** given to `aapt2`: it wants a resource it can parse,
# and a shared library is not one. So the package is linked without it and
# the library is added afterwards, under the path the platform's loader looks
# in — `lib/<abi>/`, which is where the platform's loader looks for it, by
# name (§236).
android_link "apps/craftkit-android/AndroidManifest.ime.xml" "$WORK/unsigned.apk" 16777473 "$WORK/res.zip"

ABI="$(android_abi_dir "$TRIPLE")"
mkdir -p "$WORK/stage/lib/$ABI"
cp "target/$TRIPLE/debug/libcraftkit_android.so" "$WORK/stage/lib/$ABI/libcraftkit_android.so"
cp "$DEX/classes.dex" "$WORK/stage/classes.dex"
(cd "$WORK/stage" && zip -X -q "$ROOT/$WORK/unsigned.apk" "classes.dex" "lib/$ABI/libcraftkit_android.so")

# 6. Aligned, and signed with the same key the default package is signed
#    with, so the phone accepts it over the default one.
android_sign "$WORK/unsigned.apk" "$OUT"

echo "the gated package is $OUT"
ls -l "$OUT"
