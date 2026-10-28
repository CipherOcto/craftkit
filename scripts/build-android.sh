#!/usr/bin/env bash
# The tree's **default** package: the one a phone runs. §258.
#
#   scripts/build-android.sh
#
# The result is `target/craftkit-android.apk`, with the tree's library for
# **both** of a phone's ABIs — the 64-bit one and the 32-bit one — because a
# package that carries one ABI does not run on half the phones there are
# (§256).
#
# The package carries **no Java and no dex**: the platform's own
# `NativeActivity` serves an input connection that gives the commit to
# nobody, so the tree draws a keyboard of its own (§246). The gated package,
# the one with the platform's own keyboard, is built by
# `scripts/build-android-ime.sh` (§257).
#
# The manifest is the tree's own, and not `cargo apk`'s, because a package
# that asks for no permission cannot open a socket on a phone — and the
# tree's own control channel is a socket (§258).
#
# The steps this script shares with the gated package live in
# `scripts/android-package.sh`, which is the one home for them.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# shellcheck source=scripts/android-package.sh
source "$ROOT/scripts/android-package.sh"

OUT="target/craftkit-android.apk"
WORK="target/android-package"

# The tools the packaging needs.
android_need_tools aapt2 zipalign apksigner

rm -rf "$WORK"
mkdir -p "$WORK/stage"

# 1. The library, for each of the phone's ABIs.
#
# The ABIs are the tree's gate's table: this script does not have one of its
# own, because two lists of the same thing drift.
while read -r triple clang; do
  android_build_library "$triple"
  abi="$(android_abi_dir "$triple")"
  mkdir -p "$WORK/stage/lib/$abi"
  cp "target/$triple/debug/libcraftkit_android.so" "$WORK/stage/lib/$abi/libcraftkit_android.so"
done < <(android_abi_table)

# 2. The package: the manifest, and no resources.
#
# The library is added afterwards, under the path the platform's loader looks
# in — `lib/<abi>/` — because `aapt2` cannot parse a shared library (§236).
android_link "apps/craftkit-android/AndroidManifest.xml" "$WORK/unsigned.apk" 16777472

# 3. The library, into the package.
(cd "$WORK/stage" && zip -X -q "$ROOT/$WORK/unsigned.apk" lib/*/libcraftkit_android.so)

# 4. Aligned, and signed with the debug key a phone accepts.
android_sign "$WORK/unsigned.apk" "$OUT"

echo "the default package is $OUT"
ls -l "$OUT"
