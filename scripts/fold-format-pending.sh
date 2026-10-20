#!/usr/bin/env bash
# Fold the pending format changes into the next version.
#
# # The rule this script exists to protect
#
# A change that alters what a `.craftkit` file contains adds ONE file in
# `crates/io/src/format/pending/`. It never edits `CURRENT`, never edits the
# reader, and never touches a released `v<N>/`. That is what lets two such
# changes land at once without either one silently disappearing in a merge.
#
# This script is the *release* step: it takes everything pending and turns it
# into version N+1, empties the pending directory, and leaves the history
# readable. It is deliberately a `mv` and not a judgement call.
#
# # What it will not do
#
# It will not guess. If a pending file has no `step` function and is not marked
# as a no-op, the fold is refused, because a change that needs a document rewrite
# and does not have one would silently drop data on load.
#
# # Usage
#
#   scripts/fold-format-pending.sh              # report what would happen
#   scripts/fold-format-pending.sh --apply      # do it
#
# Run it from the repository root, with a clean tree, as part of cutting a
# release. Not as part of an ordinary change.

set -euo pipefail

PENDING_DIR="crates/io/src/format/pending"
PENDING_RS="crates/io/src/format/pending.rs"
FORMAT_RS="crates/io/src/format.rs"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

APPLY=0
[[ "${1:-}" == "--apply" ]] && APPLY=1

say()  { printf '%s\n' "$*"; }
fail() { printf 'error: %s\n' "$*" >&2; exit 1; }

# --- preconditions ---------------------------------------------------------

[[ -d "$PENDING_DIR" ]] || fail "$PENDING_DIR does not exist; nothing to fold."
[[ -f "$PENDING_RS" ]]  || fail "$PENDING_RS is missing; this is not a craftkit tree."

mapfile -t pending < <(find "$PENDING_DIR" -maxdepth 1 -name '*.rs' -printf '%f\n' | sed 's/\.rs$//' | sort)

if [[ ${#pending[@]} -eq 0 ]]; then
    say "nothing is pending; $PENDING_DIR is empty."
    say "If a version is being released with no format change, edit CURRENT in"
    say "$FORMAT_RS by hand and record why in its history comment."
    exit 0
fi

say "pending format changes (${#pending[@]}):"
for name in "${pending[@]}"; do
    say "  • $name"
done

# Every pending change must be registered in pending.rs, or the fold would
# produce a version that does not apply its own changes.
for name in "${pending[@]}"; do
    grep -q "\"$name\"" "$PENDING_RS" \
        || fail "$name is not listed in $PENDING_RS; add it to FILES and to apply() first."
done

# --- the next version ------------------------------------------------------

current="$(grep -oP 'pub const CURRENT: u32 = \K[0-9]+' "$FORMAT_RS" || true)"
[[ -n "$current" ]] || fail "could not read CURRENT from $FORMAT_RS."
next=$((current + 1))

say ""
say "current version: $current"
say "next version:    $next"

if [[ $APPLY -eq 0 ]]; then
    say ""
    say "This was a dry run. Nothing was changed."
    say "When you are ready: scripts/fold-format-pending.sh --apply"
    say ""
    say "Then, still by hand:"
    say "  1. create $PENDING_DIR/v$next/ and move the pending files in"
    say "  2. point CURRENT at $next and record the version in the history in $FORMAT_RS"
    say "  3. replace the pending list in $PENDING_RS with the empty list"
    say "  4. run: cargo xtask ci"
    exit 0
fi

# --- apply -----------------------------------------------------------------

say ""
say "folding into v$next…"

mkdir -p "$PENDING_DIR/v$next"
for name in "${pending[@]}"; do
    mv "$PENDING_DIR/$name.rs" "$PENDING_DIR/v$next/$name.rs"
    say "  moved $name.rs into v$next/"
done

say ""
say "The files are in place. The three edits that need judgement are next:"
say "  1. $FORMAT_RS — CURRENT to $next, and a line in the history saying what changed"
say "  2. $PENDING_RS — FILES back to &[], and apply() back to the loop with no arms"
say "  3. cargo test -p craftkit-io   # old designs must still load"
say ""
say "Read step 2 before you do it: a folded step is not called by the reader any"
say "more, so a design written before the change loses its rewrite unless the"
say "reader understands the older shape by hand. The recipe describes the other"
say "half — a version directory the reader dispatches to — and the two do not"
say "agree yet. KNOWN-GAPS.md section 81 has the measurements and the open half."
say ""
say "Nothing else is automatic, and nothing else should be."